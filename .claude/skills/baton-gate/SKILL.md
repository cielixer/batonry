---
name: baton-gate
description: Use before opening a batonry PR - walks the ticket's definition of done, runs the full test suite and the repository-specific checks, then a cross-model review of the whole branch, and opens the PR when all three pass
---

# Gate a batonry branch

`baton-gate #<n>` -- or with no ticket, for work done by hand. Work done by hand
still goes through the gate.

Three things, in this order, and the order matters. The ticket's own list comes
first because it is the acceptance criteria; the suite next because a review of
code that does not compile spends an expensive model's attention on findings the
compiler would have given away.

## 0. Fix the comparison base

Everything below compares against the branch point, not against `HEAD`. `HEAD`
misses committed work, and a branch with both committed and uncommitted changes
would otherwise get a partial gate.

    BASE=$(git merge-base main HEAD)

Also confirm nothing intended is untracked -- `git status --porcelain` should
show no `??` for files the change needs. An untracked file is invisible to every
check that follows.

## 1. Walk the ticket's definition of done

**`gh issue view <n>` and go through its checkboxes one at a time.** For each,
say where it was verified: a test name, a command and its output, a file and
line. Then tick it on the issue.

This is the step the whole board design exists for. `Verify` is a separate column
from `In progress` precisely so that "the code is written" and "the checkboxes
were actually checked" cannot share a cell -- and the first ticket through this
workflow reached its PR with **none** of its thirteen boxes ticked, because
nothing walked them. The work had been done; nothing recorded it, so nothing
would have caught it if it had not been.

**A definition-of-done item you cannot point at is not done.** If one turns out
to be unverifiable as written, that is a defect in the ticket: fix the wording,
say so in the ledger, and do not tick it.

With no ticket, say instead what the change was for and how you know it does it.

## 2. The suite

**Run each command unchanged, as its own top-level command, and read its own
exit status.** Piping into `grep`, filtering, truncating with `head`, an `||`
fallback, or reading `$?` after a later command **invalidates the result** --
that is not pedantry, it produced two false "clean" reports on the first ticket
through here, both times hiding a failure in freshly written code. Record the
command and its status in the ledger.

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo build --workspace
    cargo test --workspace

A failure stops the gate. Then the checks specific to this repository, which
nothing else enforces -- there is no CI yet, so this skill *is* the enforcement.

**The copied crate is frozen.** `crates/winit` is not byte-identical to
upstream -- it carries our IME patch -- it is **frozen relative to the branch
base**, with every line of our divergence recorded in its `UPSTREAM.diff`. So
the check is that this branch adds no new divergence:

    git diff --quiet "$BASE" -- crates/winit

Non-zero means stop. Either the change is wrong, or it is deliberate and
`UPSTREAM.diff` plus `NOTICE.md` are updated in the same commit.

**And the substitution is still in place.** `[patch.crates-io]` is the whole
reason one copy is enough, and **deleting it is silent** -- cargo emits no error
and no warning, resolves the published crate, and the first Hangul jamo starts
being dropped again. `crates/baton-term/tests/ime.rs` asserts it; do not let
that test be weakened.

**`crates/baton-term` may diverge, so its record has to keep up.** If
`git diff --quiet "$BASE" -- crates/baton-term` fails, regenerate
`UPSTREAM.diff` per its header and confirm no unexplained delta.

**Everything git tracks is English**, except fixtures whose subject *is* the
text. The allowlist lives in `CLAUDE.md` section 7c and **is not restated here**
-- one number in two places is how it drifts, and it already moved twice in a
day. Compare, and treat a difference in **either** direction as a failure: an
extra file is an English-only violation, a missing one means the allowlist is
now over-permissive and gets narrowed in this commit.

    git ls-files | xargs grep -l '[가-힣]'

**Every new word is defined in the crate's `//!` docs.** Most of the vocabulary
here is this project's rather than Rust's or `iced`'s -- action, binding, chord,
issue site, `when` clause -- so a reader arrives without it. #10 introduced seven
public types and defined none of them, and it surfaced as a reviewer saying the
words were unfamiliar, which is the expensive way to find out.

So: for each public type or piece of project vocabulary this branch introduces,
check the crate's `//!` block defines it, in English, with a link to the type
where the detail lives. Run
`RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links" cargo doc --no-deps` -- a
vocabulary list with dead links is worse than none.

**There is exactly one home for this, and it is the `//!` block.** It sits next
to the code, so it cannot drift from what it describes; `cargo doc` renders it;
and it is all anyone gets from a clone. A per-crate `README.md` would restate the
type docs with nothing keeping the two honest, and a second copy anywhere else is
an obligation that gets skipped -- which is how this gap appeared. A README earns
its place only when a crate is extracted and has an audience on crates.io.

**Every mirrored specification the ticket named was updated.** The ticket's
definition of done lists the canonical source and each mirror and generator for
any grammar, schema or vocabulary it touches (see `baton-plan`). Check each one
either changed or is explicitly unaffected. Generated files are rebuilt from
their generator, never hand-edited.

**Test flakiness is a failure, not a retry.** Run `cargo test --workspace`
**three consecutive times** and require three passes. A test that passes on a
later attempt is a defect report: this repository has already had a gauge
underflow surface as an assertion failure in an unrelated test, and re-running
would have buried it.

## 3. Cross-model review

`gpt-5.6-sol` at xhigh, read-only, against `CLAUDE.md` and
`.claude/skills/codex/review-checklist.md`. **Run it in the background** with
`CODEX_TIMEOUT=1800` as a circuit breaker, and put the brief in a file -- all
free-form prose goes through `--extra-file`, never argv.

    cat > /tmp/gate.txt <<'BRIEF'
    Suite: fmt/clippy/build/test all green, three consecutive test runs.
    <the definition-of-done walk, and anything it should know>
    BRIEF
    .claude/skills/codex/codex-run.sh start --role review --extra-file /tmp/gate.txt "#<n>"

Tell it the suite passed. The reviewer is told not to hunt for coverage gaps,
and knowing the suite is green keeps it from guessing.

### The loop

- `APPROVED` -- go to step 4.
- `REQUEST_CHANGES` -- **engage critically.** Open every `file:line` it cites
  before agreeing. A different model is valuable because it does not share the
  implementation's blind spots; the same property means it does not share its
  context, so some findings are it missing something. Fix the real ones. Push
  back on the wrong ones with the argument, once. Then resume with
  `--notes-file`.
- `NEEDS_REWORK` -- structural. Surface it before mass-editing; this is one of
  the four things that stops a run.

**A standing Critical or Major does not pass.** The checklist says approval
requires zero of each, and this skill does not get to be more permissive than
the checklist it points at. Three rounds is the ceiling; after that:

- **Minor and Suggestion** findings may be closed by a `Ruling:` in the ledger.
- **Critical and Major** may only be closed by a line in `DECISIONS.md`, and
  the reviewer is told in the next `--notes-file` that they were closed and why.

That is deliberately harder than a ledger note and deliberately not "the
reviewer has the last word". On the first ticket through here the one Critical
was real *and* its proposed fix was in the wrong direction -- the inconsistency
existed in four places and the reviewer wanted the three correct ones changed.
A reviewer that cannot be overruled blocks on its own misdiagnosis; an operator
who can wave a Critical away in a per-ticket, gitignored ledger is not being
held to anything. A decision entry is the middle: public, permanent, and
answerable later.

## 4. Promote what the ledger learned

**Before opening the PR, empty the ledger of anything durable.** It is
per-ticket and gitignored, so whatever stays in it dies with the ticket. Each
lesson has exactly one of three homes:

| kind | goes to |
|---|---|
| A convention the implementer should have known | `codex/prompts/implement.tpl` |
| A gap in how this workflow runs | the relevant `SKILL.md` |
| A choice someone will question later | `DECISIONS.md` |

The first ticket through here produced four durable lessons and three of them
survived only because someone asked for a retrospective afterwards. This step is
that retrospective, made routine and small.

## 5. Commit and open the PR

Tidy the branch into **one intentional commit**. The repository squash-merges
with `squash_merge_commit_message=COMMIT_MESSAGES`, so that commit's subject and
body *become* the squash message -- which makes "tidy the commits" load-bearing
rather than housekeeping, because several commits concatenate. Verify the text
as what will live in `git log`: prose, what changed for a person using the app,
and what the alternative was.

Subject carries the milestone and stage; GitHub appends the PR number.

    m1/stage1: action registry and the default keymap

Then the PR, keeping the structure of `.github/pull_request_template.md`, with
`Closes #<n>` in the body. **Only tick a checkbox you actually verified** --
that is the whole reason the list exists. Attach numbers, not adjectives, if the
ticket was on a performance floor.

Branch policy is settled at ticket setup and not re-argued here.

## Done when

The ticket's definition of done is walked and ticked, the suite is green
including the repository-specific checks, every new term is defined in the
crate's `//!` docs, the review returned `APPROVED`
with no standing Critical or Major, the ledger has been emptied of anything
durable, and the PR is open with `Closes #<n>` and an honestly-ticked checklist.
