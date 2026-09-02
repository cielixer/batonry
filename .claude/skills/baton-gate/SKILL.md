---
name: baton-gate
description: Use when the owner asks to gate a batonry branch whose draft PR baton-ticket already opened - walks the ticket's definition of done, runs the full test suite and the repository-specific checks, reviews the branch on two parallel fresh contexts, and updates the draft PR when all three pass
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
text. Since #17 this check and the two hardcopy checks above are scripts that
CI runs on every pull request -- run the same ones rather than restating them,
so a hand gate and CI cannot drift:

    ./.github/ci/check-english-only.sh
    ./.github/ci/check-hardcopy-winit.sh
    ./.github/ci/check-hardcopy-term.sh

The allowlist is `.github/korean-allowlist.txt` (via `CLAUDE.md` section 5); a
difference in **either** direction fails -- an extra file is an English-only
violation, a missing one means the allowlist is over-permissive and gets
narrowed in this commit. The branch-relative freeze checks above stay: they
answer "did THIS branch touch the copies", which the upstream-anchored scripts
do not.

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

**A shared abstraction needs its guarantees tested everywhere it is shared.**
Factoring two things into one macro or one helper moves what used to be visible
in each into a place nothing checks. On #11 the shared `bitset!` computed bit
positions that had previously been written out and reviewable by eye; breaking
the macro failed one type's tests and passed the other's, because only one had a
test for the property. Check that each user of a new shared thing pins what it
now relies on.

**Test flakiness is a failure, not a retry.** Run `cargo test --workspace`
**three consecutive times** and require three passes. A test that passes on a
later attempt is a defect report: this repository has already had a gauge
underflow surface as an assertion failure in an unrelated test, and re-running
would have buried it.

## 3. The review -- two fresh contexts in parallel, then Fable absorbs the churn

**Launch both at once, read-only, against `CLAUDE.md`, the `CLAUDE.md` of every
crate the diff touches, and `.claude/skills/codex/review-checklist.md`. Both
must return `APPROVED`.**

The sol lane, in the background with `CODEX_TIMEOUT=1800` as a circuit breaker,
the brief in a file -- all free-form prose goes through `--extra-file`, never
argv:

    cat > /tmp/gate.txt <<'BRIEF'
    Suite: fmt/clippy/build/test all green, three consecutive test runs.
    <the definition-of-done walk, and anything it should know>
    BRIEF
    .claude/skills/codex/codex-run.sh start --role review --extra-file /tmp/gate.txt "#<n>"

The Fable lane: a fresh-context agent of the session's model -- **Fable first,
Opus if Fable is unavailable**; their usage is metered separately from Codex --
holding the same empirical standard: demonstrate what it claims broken, say how
it confirmed what it calls load-bearing. Fresh context is the property doing
the work, and wherever luna implemented, it reviews that code cross-model
by construction.

Tell both that the suite passed. A reviewer told the suite is green does not
hunt coverage gaps or guess.

**The lanes split by round, and that split is the cost model** (sol at xhigh
dominated Codex usage; #13 spent five runs). sol sees the branch **first and
last**: the parallel pass above, and -- only if the branch moved after its
verdict -- one final delta confirmation before the gate closes (resume its
thread). Every re-review round in between runs on the Fable lane alone, by
resuming the same agent with what landed since.

**If sol is unavailable** -- quota exhausted, outage -- the Fable lane alone
gates, as #17 and #12 did. If Fable is unavailable, the fallback is Opus, not
skipping the lane.


### The loop

**An approval covers the branch as it stood.** Anything committed after it has
not been reviewed, so if the branch moves -- and on #11 it moved three times,
twice changing the public API -- the gate runs again from step 0, with the
re-review on the Fable lane (see above). Say in the notes what landed since,
so the reviewer reads a delta rather than the whole branch a second time. That
re-run found a Critical the first pass could not have: the change that caused
it did not exist yet.

- `APPROVED` -- go to step 4.
- `REQUEST_CHANGES` -- **engage critically.** Read the lane's output **in
  full from its out-file first** -- on #14 a `tail` ate the finding at the
  top of sol's list, the disposition covered 7 of 8, and the omission cost a
  whole extra round. Then open every `file:line` it cites
  before agreeing. A different model is valuable because it does not share the
  implementation's blind spots; the same property means it does not share its
  context, so some findings are it missing something. Fix the real ones. Push
  back on the wrong ones with the argument, once. Then resume the lane that
  raised them -- the Fable agent directly, sol via `--notes-file`.
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

**Before the gate closes -- the draft PR is already open and waiting --
empty the ledger of anything durable.** It is
per-ticket and gitignored, so whatever stays in it dies with the ticket. Each
lesson has exactly one of three homes:

| kind | goes to | lands |
|---|---|---|
| A choice someone will question later | `DECISIONS.md` | **in this branch** |
| A convention the implementer should have known | `codex/prompts/implement.tpl` | on the follow-up branch |
| A gap in how this workflow runs | the relevant `SKILL.md` | on the follow-up branch |

**Only the first lands here, and the reason is what it is tracked by.** `docs/`
is gitignored, so a `DECISIONS.md` entry never enters a commit and cannot widen
one. The other two are tracked files, and this repository squash-merges: putting
them in the ticket's branch gives `main` a single commit holding both the
feature and workflow documentation its message does not mention. `baton-ticket`
says the same thing about a tooling defect found mid-loop, and for the same
reason.

So: write the entry, and for anything tracked, **capture the diff and then put
the tree back** -- the capture alone is not enough, because the edits it was
made from are still in the working tree and step 5 would commit them, which is
the exact failure this split exists to prevent:

    git diff -- .claude > .claude/skills/codex/state/ticket-<n>.lessons.patch
    git restore .claude

The pathspec matters too: a bare `git diff` would sweep any unstaged ticket work
into the lessons patch. Name the patch in the ledger; it lands on a branch off
`main` after this PR is open, with the other tooling work that has accumulated.

This step exists because the first ticket through here produced four durable
lessons and three survived only because someone asked for a retrospective
afterwards. The split exists because #11 shipped 63 lines of workflow
documentation inside a feature branch, which the cross-model review caught as a
Major -- these two skills were giving opposite instructions, and this table is
the half that was wrong.

## 5. Update the draft PR

**The draft PR already exists** -- `baton-ticket` opens it before the owner's
review, which is what this gate follows. What this step does is bring it up to
what the gate established:

- Fold everything the gate changed into the branch's **one intentional
  commit** (`git commit --amend`, `git push --force-with-lease`). The
  repository squash-merges with `squash_merge_commit_message=COMMIT_MESSAGES`,
  so that commit's subject and body *become* the squash message; verify the
  text as what will live in `git log` -- prose, what changed for a person
  using the app, what the alternative was, and what the review rounds found.
- Update the PR body: **tick the definition-of-done boxes the walk actually
  verified** -- only those; that is the whole reason the list exists -- and
  record the review outcome. Attach numbers, not adjectives, if the ticket
  was on a performance floor.
- **Leave it a draft.** CI deliberately skips drafts; the run happens once,
  when the owner marks it ready. Do not mark it ready yourself unless they
  say so.

For work with no ticket and no PR yet (the hand-done case), create the draft
PR here instead, same template, same rules.

Branch policy is settled at ticket setup and not re-argued here.

## Done when

The ticket's definition of done is walked and ticked, the suite is green
including the repository-specific checks, every new term is defined in the
crate's `//!` docs, both review lanes returned `APPROVED`
with no standing Critical or Major, the ledger has been emptied of anything
durable, and the draft PR carries the amended commit and an honestly-ticked
checklist.
