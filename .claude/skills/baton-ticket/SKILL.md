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
2. `gh issue view <n>` -- the ticket is the plan. Read its "Read first" list and
   actually read those files. If the four sections are missing or vague, stop and
   use `baton-plan` instead; implementing an unclear ticket produces work that
   fails review.
3. Branch, always, no need to ask:

       git switch -c m1/stage<N>-<slug>

4. Open the ledger at
   `.claude/skills/codex/state/<key>.ledger.md` (gitignored, survives across
   sessions for this ticket). If one exists, read it -- you may be resuming.

## Never touch

`crates/baton-winit`, `crates/baton-iced`, `crates/baton-iced-winit` are
verbatim copies of upstream. Their whole value is being byte-identical, and each
has an `UPSTREAM.diff` that stops making sense the moment they drift. The
implementer is told this too; **verify it in every batch review** with
`git status -s`, because a plausible-looking edit there is the most expensive
mistake available in this repository.

`crates/baton-term` may change: mark each line `// BATON:` and regenerate
`UPSTREAM.diff` (its header documents how) before the gate.

## The batch loop

### 1. Decide the batches

Read the ticket's checkboxes and split them.

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

First batch:

    .claude/skills/codex/codex-run.sh start --role implement "#<n>" \
        "Implement only: <checkbox text>"

Each later batch resumes the same thread, carrying your corrections. Context
compounds across turns -- this is why it is one thread and not one call per
batch:

    .claude/skills/codex/codex-run.sh resume --role implement \
        --notes "<what you fixed after the last batch and why; conventions now binding>" \
        "#<n>" "Now implement: <next checkboxes>"

**Run these in the background.** They routinely outlast a foreground timeout.
Set `CODEX_TIMEOUT=1800` as a circuit breaker against a hung run -- generous, not
a target.

Read the trailing tag: `IMPLEMENTATION_COMPLETE` means review the batch;
`IMPLEMENTATION_PARTIAL` means read the report and either resume for the
remainder or finish the leftovers yourself during review.

### 3. Review the batch

Before requesting the next one:

1. **Review the delta only.** `git status -s && git diff` shows just this batch,
   because earlier batches are staged. Check it against the ticket, `CLAUDE.md`,
   and the patterns already in the code.
2. **Confirm no copied crate was touched.** Non-negotiable, every batch.
3. **Fix problems yourself.** Do not send fixes back -- a round trip costs more
   than the edit. What you fixed and why becomes the next `--notes`.
4. **Micro-gate:** `cargo clippy --workspace --all-targets -- -D warnings` and
   `cargo build --workspace`. Fast checks only; the full suite waits for the
   gate. Fix failures now.
5. **Write the tests this batch needs.** They are yours. The user does not review
   test code, so the standard is on you: a regression test is verified by
   reintroducing the bug and watching it fail. A test that passes both ways is
   worse than no test, because it is believed.
6. **Stage:** `git add -A`. No commits yet -- the next delta review needs a clean
   worktree diff.
7. Verify the checkboxes the implementer ticked match what the diff contains.

**Adapt.** Clean batch, grow the next. Heavy corrections, shrink it and spell out
the pattern in the notes. If the implementer keeps reintroducing something you
corrected, reset the thread at the next batch boundary -- the ticket plus a
summary note rebuilds context faster than arguing.

### 4. Final pass

After the last batch, read the **whole feature diff** once: `git diff HEAD`.
Batch reviews catch local problems; this catches drift across batches --
duplicated helpers, names that diverged, dead code left by a course correction.
Fix directly.

Then regenerate `crates/baton-term/UPSTREAM.diff` if that crate changed.

## Then the gate

Do not open a PR from here. Run `baton-gate #<n>` -- the full test suite and a
cross-model review of the whole branch.

## Ledger

Append as you go. It is the record of what happened and why, and the thing that
lets a later session resume without re-deriving:

    ## Batch 3 -- registry merge
    Implemented: id validation, duplicate detection
    Fixed by hand: used a HashMap where the ticket said linear scan (A10 wants
      lookup off the hot path)
    Ruling: ids validated at build time, not boot -- the ticket said "a test",
      but a build failure is strictly better and costs nothing. If wrong, it
      moves to a test in one commit.
    Tests: duplicate-id detection, verified by adding a duplicate

## Done when

Every checkbox is implemented and verified against the diff, the copied crates
are untouched, tests exist for new behaviour, clippy and build are green, the
final pass is done, and the ledger has an entry per batch.
