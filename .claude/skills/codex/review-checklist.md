# Review checklist

The single source of truth for what a review checks, how findings are graded,
and when a change passes. `CLAUDE.md` is the authority for the rules; this file
says how to apply them.

## Severity

| | means | consequence |
|---|---|---|
| **Critical** | Wrong results, data loss, a panic reachable from normal use, or a violated architecture contract | Blocks. `NEEDS_REWORK` if the approach caused it |
| **Major** | A real defect under conditions that will occur -- a race, an unbounded queue, a leak, an error path that swallows its cause | Blocks |
| **Minor** | Correct but avoidably fragile: a magic number, a duplicated helper, an error message nobody can act on | Does not block; fix if cheap |
| **Suggestion** | A better way that is not clearly worth the churn | Never blocks |

`APPROVED` requires zero Critical and zero Major.

## 1. Architecture contracts

`CLAUDE.md` section 6 numbers eleven contracts A1-A11 and states, for each, what
breaks later if it is ignored. They exist because each one is cheap to honour now
and a rewrite to retrofit. Check the ones the change touches, and check the
ticket's stated contract in particular.

The ones most often violated by plausible-looking code:

- **A2** -- the terminal is driven by *bytes*. Handing the core a PTY handle
  works today and blocks the second byte source entirely.
- **A7** -- size calculation takes window geometry, chrome constants, and font
  metrics. Never a render result. A single measurement read back from layout is
  a resize feedback loop.
- **A10 / A11** -- a UI element emits an action id and input goes through the
  router. A direct call from a widget to a PTY write is the failure that cannot
  be retrofitted.

Also structural, from sections 2 and 7c:

- `baton-core` depends on no UI and no I/O; `baton-action` does not know `iced`
  exists; `baton-term` does not know `baton-core` exists.
- `#[cfg(target_os)]` appears only in `baton-platform`.
- `baton-ui` has no `main` -- a crate with one cannot be driven headlessly.
- Everything git tracks is English, except fixtures whose subject *is* the text.

## 2. Correctness

- Can this produce a wrong result rather than an error? Silent wrongness is the
  worst outcome available and the hardest to notice later.
- Is any arithmetic able to wrap or overflow? A gauge that can go below zero
  panicked a background thread in this codebase already.
- Does an error path lose the cause, or report something that is not what
  happened?
- Is there an index or a lookup that assumes an entry still exists? Messages for
  a closed pane are normal, and `state.panes[id]` is how one message kills the
  app.

## 3. Concurrency

- **Is a lock held across an `await` or a channel send?** This is the single
  highest-value question in this repository -- `blocking_send` under
  `FairMutex<Term>` deadlocked it 100% of the time, and the code read as
  obviously fine.
- Can this panic on a thread whose death nobody notices? Instrumentation and
  reader threads count.
- Is anything unbounded that is fed by a producer we do not control?
- Is a dropped or coalesced message carrying information? Wakeups may be
  dropped. `Exit`, `Title`, and PTY replies may not.

## 4. Cost

- Per-frame work: is text reshaped, is a grid cloned, is a cache cleared
  unconditionally? The performance floors in `CLAUDE.md` section 9 are
  completion criteria, not aspirations.
- Allocation inside a loop over cells. There are hundreds of thousands of them.
- Does an idle pane cause work? Idle across twelve panes has a 2% CPU ceiling.

## 5. Fit

- Does the change do what the ticket's "Definition of done" says, and only that?
- Is `crates/winit` untouched, and is any `crates/baton-term` change marked
  `// BATON:` with `UPSTREAM.diff` regenerated?
- Are colours and user-visible strings in one place rather than scattered?
- Does a new module or crate exist because a rule needs enforcing at that
  boundary, or only because it felt tidy? A folder is enough when there is no
  rule.

## Out of scope

**Coverage.** Whether a behaviour needs a test is the requester's call. Do not
hunt for gaps; say in prose if one is glaring.

**Test validity is not out of scope.** The requester is the only author and the
only reviewer of this suite, which is a single point of failure, and a first pass
through this workflow shipped a dead loop and a pointless fixture in
requester-written tests. Flag an assertion that cannot fail, a branch nothing
reaches, a fixture that proves nothing, and any claim that a regression was
"verified by reintroducing the bug" where the test would obviously still pass.
Grade these like any other finding.

**Formatting and lint.** `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets -- -D warnings` ran and passed before
this review started.

**Upstream code.** `crates/winit` is **frozen**, not identical to upstream -- it
carries a deliberate IME patch, recorded line by line in its `UPSTREAM.diff`.
Code there that this branch did not touch is not this change's problem. A change
this branch *makes* there is, because there should be none.
