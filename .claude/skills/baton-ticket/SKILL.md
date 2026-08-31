---
name: baton-ticket
description: Use when implementing a batonry GitHub ticket - runs the whole loop from branch to gate, delegating implementation to Codex in reviewed batches and keeping a ledger of decisions
---

# Implement one batonry ticket

`baton-ticket #<n>`

One ticket, one branch, one PR. Implementation is delegated to `gpt-5.6-luna` in
batches; **you review every batch and fix problems yourself** rather than
arguing with the implementer. Tests are yours, not the implementer's.

## Run continuously

**Do not check in between batches.** The user asked for the ticket to be
implemented; implementing it is the answer, and "should I continue?" costs their
attention and buys nothing.

**Rule, do not stall.** Ambiguity in the ticket, a conflict between the ticket
and `CLAUDE.md`, a batch that turns out bigger than planned -- decide it. The
contract is the authority, the ticket is its application, your judgement settles
what neither covers. Record each one in the ledger as

    Ruling: <what you decided> -- <why> -- <what it costs if wrong>

A wrong ruling costs rework the user can see and undo. A session parked on a
question costs their day.

**Four things stop you, and only these:** something irreversible or destructive;
something security-sensitive; a change outside this repository; or a decision
that is genuinely the user's -- a product behaviour, not a technical
consequence.

## Setup

1. Read `CLAUDE.md` in full. It is the contract every batch is judged against.
   Then read the `CLAUDE.md` of every crate this ticket touches -- the root's
   section 1 lists which crates have one. Those rules are as binding as the
   root's, and a batch is judged against both.
2. `gh issue view <n>` -- the ticket is the plan. Read its "Read first" list and
   actually read those files. If the four sections are missing or vague, stop and
   use `baton-plan` instead; implementing an unclear ticket produces work that
   fails review.
3. Branch off **`main`**, always, no need to ask:

       git switch -c m1/stage<N>-<slug> main

4. Open the ledger at `.claude/skills/codex/state/ticket-<n>.ledger.md` --
   gitignored, one per ticket, and deliberately **not** keyed like the Codex
   threads: resetting a thread rebuilds context, it does not erase what
   happened. If a ledger already exists, read it; you may be resuming someone's
   afternoon.

## Never touch

`crates/winit` is **frozen** -- not byte-identical to upstream, since it carries
our macOS IME patch, but frozen, with every line of our divergence recorded in
its `UPSTREAM.diff`, which stops making sense the moment it drifts further. The
implementer is told this too; **verify it in every batch review**, because a
plausible-looking edit there is the most expensive mistake available here.

`crates/baton-term` may change: mark each line `// BATON:` and regenerate
`UPSTREAM.diff` (its header documents how) before the gate.

## The batch loop

### 1. Decide the batches

**Batches run one at a time, in one thread.** Both sources this workflow came
from say so explicitly and for the same reason -- see `DECISIONS.md` #68 for the
argument and the measurement. The short version: corrections from reviewing
batch N are what batch N+1 needs, and they cannot exist before the review does.

**Concurrency belongs across tickets**, in separate worktrees, decided before
starting rather than mid-loop.

Then split the ticket's checkboxes:

- A batch is **the smallest set that leaves the tree green** -- compiles, clippy
  clean. Never split an interface from its implementation and its wiring.
- Aim for a reviewable diff, roughly 300 changed lines. A checkbox bigger than
  that is its own batch.
- Size by risk: anything novel or on an architecture contract goes down to one
  checkbox. Mechanical work batches larger.
- A ticket of three or four low-risk checkboxes goes in one shot. No ceremony.
- **Filter out what is not the implementer's**: anything needing a decision from
  the user, a credential, or an action outside the repo. Those are yours.

### 2. Delegate

**Put the brief in a file.** All free-form prose goes through `--extra-file` and
`--notes-file`, never argv -- one line is enough to contain a backtick, a `$` or
a `!`, and argv hands the text to the shell first. A real run lost part of its
instructions to `command not found: when`. A quoted heredoc interpolates nothing:

    cat > /tmp/batch1.txt <<'BRIEF'
    Implement only: <checkbox text>
    BRIEF
    .claude/skills/codex/codex-run.sh start --role implement \
        --extra-file /tmp/batch1.txt "#<n>"

Each later batch resumes the same thread, carrying your corrections. Context
compounds across turns -- that is why it is one thread and not one call per
batch:

    .claude/skills/codex/codex-run.sh resume --role implement \
        --notes-file /tmp/notes1.txt --extra-file /tmp/batch2.txt "#<n>"

**Run these in the background** with `CODEX_TIMEOUT=1800` as a circuit breaker
against a hung run -- generous, not a target. They routinely outlast a
foreground timeout.

Read the trailing tag: `IMPLEMENTATION_COMPLETE` means review the batch;
`IMPLEMENTATION_PARTIAL` means read the report and either resume for the
remainder or finish the leftovers yourself during review.

### 3. Review the batch

In this order. The tests come **before** the micro-gate, because writing them
changes the tree and a gate run from before that is evidence about code that no
longer exists.

1. **Review the delta only.** `git status -s && git diff` shows just this batch,
   because earlier batches are staged. Check it against the ticket, `CLAUDE.md`,
   and the patterns already in the code.
2. **Confirm no frozen crate was touched.** Non-negotiable, every batch.
3. **Fix problems yourself.** Do not send fixes back -- a round trip costs more
   than the edit. What you fixed and why becomes the next batch's notes.
4. **Read a test file before writing to it.** A whole-file write destroys what
   is there: #11 overwrote `tests/keymap.rs` and lost six tests from #10, and one
   of the replacements contradicted a position that file had recorded. If the new
   tests are about a different subject they belong in a different file, which is
   usually the better split anyway.
5. **Write the tests this batch needs.** They are yours. The user does not read
   test code, so the standard is on you: a regression test is verified by
   reintroducing the bug and watching it fail. A test that passes both ways is
   worse than none, because it is believed.

   **Move code by line ranges, and check the move before believing it.** Cutting
   two blocks by byte offset shifts the second one's indices when the first is
   removed; on #11 that silently truncated a function's closing brace, and the
   same command's `cargo build` still reported success. Sorting both versions
   and diffing shows exactly what a reordering added and removed -- if it is a
   pure move, the diff is empty.

   **Confirm the mutation landed before believing the result.** A string
   replacement that matches nothing reports a clean pass and proves nothing --
   #11 reported three of them, because rustfmt had split the target across lines
   and a later attempt matched the same identifier in a different table. Print
   the mutated line, or assert the replacement happened.

   The cross-model review checks test *validity* at the gate -- vacuous assertions, dead branches, fixtures that
   prove nothing -- but not until then, so do not lean on it here.
6. **Micro-gate.** Each command as its own top-level command, reading its own
   exit status. **No pipe, no `head`, no `||` fallback, and never `$?` after a
   later command** -- that produced two false "clean" reports on the first
   ticket through here, both hiding a failure in code written minutes earlier.

       cargo fmt --all --check
       cargo clippy --workspace --all-targets -- -D warnings
       cargo build --workspace

   The full suite waits for the gate. Fix failures now.
7. **Stage the paths this ticket owns**, not everything:

       git add -A -- crates/

   `git add -A` picks up whatever else you touched. That is how a tooling fix
   got entangled with the first ticket and cost a detour to separate.
8. Verify the checkboxes the implementer ticked match what the diff contains.

**Adapt.** Clean batch, grow the next. Heavy corrections, shrink it and spell out
the pattern in the notes. If the implementer keeps reintroducing something you
corrected, reset the thread at the next batch boundary -- the ticket plus a
summary note rebuilds context faster than arguing.

**A tooling defect found mid-loop does not get fixed mid-loop.** Note it in the
ledger and fix it after this ticket's PR is open, on its own branch. Interleaving
the two costs more than the delay, and a squash merge would put two unrelated
things in one commit.

The same holds for a workflow lesson the gate wants promoted: `baton-gate` step
4 writes the `DECISIONS.md` entry in the ticket's branch, because `docs/` is
untracked, and leaves anything tracked as a patch for the follow-up branch. If
the two ever read as though they disagree, this is the one that is right --
**a ticket's commit contains that ticket's code.**

### 4. Final pass

After the last batch, read the **whole feature diff** once:
`git diff $(git merge-base main HEAD)`. Batch reviews catch local problems; this
catches drift across batches -- duplicated helpers, names that diverged, dead
code left by a course correction, a free function that should have been a
standard trait impl. Fix directly.

Then regenerate `crates/baton-term/UPSTREAM.diff` if that crate changed.

## Then the gate

Do not open a PR from here. Run `baton-gate #<n>`, which walks the ticket's
definition of done, runs the full suite and the repository-specific checks, and
gets a cross-model review of the whole branch.

## Ledger

Append as you go. It is the record of what happened and why, and what lets a
later session resume without re-deriving:

    ## Batch 3 -- registry merge
    Implemented: id validation, duplicate detection
    Fixed by hand: used a HashMap where the ticket said linear scan (A10 wants
      lookup off the hot path)
    Ruling: ids validated at build time, not boot -- the ticket said "a test",
      but a build failure is strictly better and costs nothing. If wrong, it
      moves to a test in one commit.
    Tests: duplicate-id detection, verified by adding a duplicate

**The ledger is gitignored and per-ticket, so anything durable in it has to be
moved out before the ticket closes.** That is a step in `baton-gate`; write
entries knowing they will be triaged rather than kept.

## Done when

Every checkbox is implemented and verified against the diff, the frozen crates
are untouched, tests exist for new behaviour, fmt and clippy and build are green,
the final pass is done, and the ledger has an entry per batch.
