#!/usr/bin/env bash
# Drive a persistent Codex CLI thread for one target.
#
#   codex-run.sh start  --role implement|review [--prompt NAME] <target> [extra...]
#   codex-run.sh resume --role implement|review [--prompt NAME] [--notes "..."] <target> [extra...]
#   codex-run.sh show   <target>
#   codex-run.sh reset  <target>
#
# --role picks the model and the sandbox. --prompt picks the template, and
# defaults to the role name: reviewing a plan rather than a diff is the same
# model in the same sandbox reading a different brief, so it is
# `--role review --prompt plan-review`, not a third role.
#
# A "target" is anything that names a unit of work: a GitHub issue ("#12"), a
# branch, a path. State is keyed per target, so two units of work never share a
# thread. Everything lives under state/<key>.{thread,out.txt,events.ndjson},
# which is gitignored.
#
# The design -- one persistent thread per target, notes carried across turns,
# role-selected models -- is adapted from TRIP-workflow. The code is ours; see
# ../NOTICE.md.

set -euo pipefail

SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STATE_DIR="$SKILL_DIR/state"
mkdir -p "$STATE_DIR"

die() { echo "error: $*" >&2; exit 1; }

# Validated here rather than on first use, so a typo fails before we have
# rendered a prompt or touched any state.
CODEX_TIMEOUT="${CODEX_TIMEOUT:-0}"
case "$CODEX_TIMEOUT" in
    ''|*[!0-9]*) die "CODEX_TIMEOUT must be whole seconds (got '$CODEX_TIMEOUT')" ;;
esac
usage() { sed -n '3,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2; exit 64; }

# ---------------------------------------------------------------- roles
#
# Implementation runs Luna at high effort on the fast tier; review runs Sol at
# xhigh on standard routing. A different model for review is the entire point:
# same-model review is blind in the same places the implementation was.
# Override per run with CODEX_MODEL / CODEX_EFFORT / CODEX_TIER.
role_config() {
    case "$1" in
        implement)
            MODEL="${CODEX_MODEL:-gpt-5.6-luna}"
            EFFORT="${CODEX_EFFORT:-high}"
            TIER="${CODEX_TIER:-fast}"
            SANDBOX="workspace-write" ;;
        review)
            MODEL="${CODEX_MODEL:-gpt-5.6-sol}"
            EFFORT="${CODEX_EFFORT:-xhigh}"
            TIER="${CODEX_TIER:-default}"
            SANDBOX="read-only" ;;
        *) die "--role must be 'implement' or 'review' (got '$1')" ;;
    esac
}

# ---------------------------------------------------------------- state keys
#
# Sanitised for readability plus a checksum, so two targets that sanitise to
# the same string ("a/b" and "a__b") still get separate state.
target_key() {
    local t="$1" resolved sanitized sum
    if [ -e "$t" ]; then
        resolved="$(realpath -- "$t" 2>/dev/null || echo "$t")"
    else
        resolved="$t"
    fi
    sanitized="$(printf '%s' "$resolved" | sed 's|^/||; s|/|__|g; s|[^A-Za-z0-9._-]|_|g')"
    sum="$(printf '%s' "$resolved" | cksum | cut -d' ' -f1)"
    printf '%s.%s' "$sanitized" "$sum"
}

# ---------------------------------------------------------------- prompts
#
# Placeholders are substituted with quoted expansions so that a '&' in the
# replacement stays literal.
render() {
    local tpl="$1" body
    [ -f "$tpl" ] || die "prompt template not found: $tpl"
    body="$(cat "$tpl")"
    body="${body//'{{TARGET}}'/"${TARGET-}"}"
    body="${body//'{{TICKET}}'/"${TICKET_FILE-}"}"
    body="${body//'{{EXTRA}}'/"${EXTRA-}"}"
    body="${body//'{{NOTES}}'/"${NOTES-}"}"
    printf '%s\n' "$body"
}

# ---------------------------------------------------------------- ticket
#
# Codex runs sandboxed with no network, so it cannot call `gh` -- measured: a
# review reported "GitHub could not be reached" and had to say it could not
# name a requirement. When the target is an issue, snapshot it to a file first
# and hand the prompt that path. This also makes a run reproducible: the
# reviewer sees the ticket as it was, not as it later became.
snapshot_ticket() {
    case "$TARGET" in
        \#[0-9]*) ;;
        *) TICKET_FILE=""; return 0 ;;
    esac
    local n="${TARGET#\#}"
    TICKET_FILE="$STATE_DIR/$KEY.ticket.md"
    command -v gh >/dev/null 2>&1 || die "target is $TARGET but gh is not on PATH"
    {
        gh issue view "$n" --json number,title,labels,milestone,state \
            --template 'Issue #{{.number}}: {{.title}}
State: {{.state}}   Milestone: {{.milestone.title}}
Labels: {{range .labels}}{{.name}} {{end}}

' || die "cannot read issue $TARGET (gh failed)"
        gh issue view "$n" --json body --jq .body
    } > "$TICKET_FILE"
    [ -s "$TICKET_FILE" ] || die "issue $TARGET produced an empty snapshot"
    echo "  ticket:  $TICKET_FILE ($(wc -l < "$TICKET_FILE" | tr -d ' ') lines)"
}

# ---------------------------------------------------------------- codex
#
# CODEX_TIMEOUT is a circuit breaker against a hung run, not a performance
# target. xhigh reviews legitimately run long, so the default is unbounded.
# macOS ships no `timeout`; fall back to gtimeout and say so rather than
# silently ignoring the setting.
run_codex() {
    local t="$CODEX_TIMEOUT" bin=""
    if [ "$t" -eq 0 ]; then codex "$@"; return; fi
    if   command -v timeout  >/dev/null 2>&1; then bin=timeout
    elif command -v gtimeout >/dev/null 2>&1; then bin=gtimeout
    else
        echo "warning: CODEX_TIMEOUT set but no timeout/gtimeout; running unbounded" >&2
        echo "         (macOS: brew install coreutils)" >&2
        codex "$@"; return
    fi
    local rc=0
    "$bin" --signal=TERM --kill-after=10 "$t" codex "$@" || rc=$?
    if [ "$rc" -eq 124 ] || [ "$rc" -eq 137 ]; then
        echo "error: codex timed out after ${t}s (CODEX_TIMEOUT)" >&2
    fi
    return "$rc"
}

report() {
    echo "$1 $ROLE session for $TARGET"
    [ -n "${THREAD_ID:-}" ] && echo "  thread:  $THREAD_ID"
    echo "  model:   $MODEL / $EFFORT / $TIER"
    echo "  output:  $OUT"
    echo "---"
    cat "$OUT"
}

# ---------------------------------------------------------------- arg parsing
[ $# -ge 1 ] || usage
ACTION="$1"; shift
ROLE=""; NOTES=""; PROMPT_NAME=""
while [ $# -gt 0 ]; do
    case "$1" in
        --role)   ROLE="$2";  shift 2 ;;
        --role=*) ROLE="${1#*=}"; shift ;;
        --prompt)   PROMPT_NAME="$2"; shift 2 ;;
        --prompt=*) PROMPT_NAME="${1#*=}"; shift ;;
        --notes)   NOTES="$2"; shift 2 ;;
        --notes=*) NOTES="${1#*=}"; shift ;;
        --) shift; break ;;
        -*) die "unknown flag: $1" ;;
        *) break ;;
    esac
done
[ $# -ge 1 ] || usage
TARGET="$1"; shift
EXTRA="${*:-}"

case "$PROMPT_NAME" in
    */*|..*) die "--prompt takes a template name, not a path (got '$PROMPT_NAME')" ;;
esac

KEY="$(target_key "$TARGET")"
THREAD="$STATE_DIR/$KEY.thread"
OUT="$STATE_DIR/$KEY.out.txt"
EVENTS="$STATE_DIR/$KEY.events.ndjson"

case "$ACTION" in
show)
    [ -f "$OUT" ] || die "nothing on file for $TARGET"
    [ -f "$THREAD" ] && echo "thread: $(cat "$THREAD")"
    echo "output: $OUT"; echo "---"; cat "$OUT" ;;

reset)
    n=0
    for f in "$THREAD" "$OUT" "$EVENTS" "$EVENTS.stderr"; do
        [ -f "$f" ] && { rm -- "$f"; echo "removed $f"; n=$((n+1)); }
    done
    [ "$n" = 0 ] && echo "no state on file for $TARGET" ;;

start)
    command -v codex >/dev/null || die "codex not on PATH"
    command -v jq    >/dev/null || die "jq not on PATH"
    [ -n "$ROLE" ] || die "start requires --role"
    role_config "$ROLE"
    [ -f "$THREAD" ] && {
        echo "error: a $ROLE thread already exists for $TARGET" >&2
        echo "       thread: $(cat "$THREAD")" >&2
        echo "       use 'resume', or 'reset' to start over." >&2
        exit 2
    }
    snapshot_ticket
    PROMPT="$(render "$SKILL_DIR/prompts/${PROMPT_NAME:-$ROLE}.tpl")"
    run_codex exec --json --skip-git-repo-check --color never \
        --sandbox "$SANDBOX" \
        -c model="$MODEL" -c model_reasoning_effort="$EFFORT" -c service_tier="$TIER" \
        -o "$OUT" "$PROMPT" </dev/null >"$EVENTS" 2>"$EVENTS.stderr" || {
            rc=$?; echo "error: codex exec failed (rc=$rc)" >&2
            tail -20 "$EVENTS.stderr" >&2; exit 1; }
    THREAD_ID="$(jq -r 'select(.type=="thread.started") | .thread_id' "$EVENTS" 2>/dev/null | head -1)"
    [ -n "$THREAD_ID" ] && [ "$THREAD_ID" != null ] || {
        echo "error: no thread.started event in $EVENTS" >&2
        head -20 "$EVENTS" >&2; exit 1; }
    printf '%s\n' "$THREAD_ID" > "$THREAD"
    report started ;;

resume)
    command -v codex >/dev/null || die "codex not on PATH"
    [ -n "$ROLE" ] || die "resume requires --role"
    role_config "$ROLE"
    [ -f "$THREAD" ] || die "no thread for $TARGET; run start first"
    THREAD_ID="$(cat "$THREAD")"
    [ -n "$THREAD_ID" ] || die "thread file is empty: $THREAD (reset, then start)"
    snapshot_ticket
    PROMPT="$(render "$SKILL_DIR/prompts/${PROMPT_NAME:-$ROLE}-continue.tpl")"
    # `exec resume` inherits the original sandbox and rejects --sandbox/--color.
    run_codex exec resume "$THREAD_ID" --json --skip-git-repo-check \
        -c model="$MODEL" -c model_reasoning_effort="$EFFORT" -c service_tier="$TIER" \
        -o "$OUT" "$PROMPT" </dev/null >"$EVENTS" 2>"$EVENTS.stderr" || {
            rc=$?; echo "error: codex exec resume failed (rc=$rc)" >&2
            tail -20 "$EVENTS.stderr" >&2; exit 1; }
    report resumed ;;

*) usage ;;
esac
