---
name: baton-gate
description: Use before opening a batonry PR - runs the full test suite and the repository-specific checks, then a cross-model review of the whole branch, and opens the PR when both pass
---

# Gate a batonry branch

`baton-gate #<n>` -- or with no ticket, for work done by hand.

Two things, in this order, and the order matters: **the suite runs first.** A
review of code that does not compile spends an expensive model's attention on
findings the compiler would have given away.

Callable on its own. Work done by hand still goes through the gate.

## 1. The suite

All of it. Nothing here is optional, and a failure stops the gate.

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo build --workspace

Then the checks that are specific to this repository, and that nothing else
enforces -- there is no CI yet, so this skill *is* the enforcement:

**Copied crates are still byte-identical to upstream.** The three verbatim
copies exist to be verbatim; a diff there is a defect even if it compiles.

    for c in baton-winit baton-iced baton-iced-winit; do
      echo "== $c: $(grep -c '^+' crates/$c/UPSTREAM.diff) added lines on record"
    done
    git diff --stat HEAD -- crates/baton-winit crates/baton-iced crates/baton-iced-winit

If that last command prints anything, stop. Either the change is wrong, or it is
deliberate and `UPSTREAM.diff` plus `NOTICE.md` need updating in the same commit.

**`crates/baton-term`'s diff is current.** It is the copy that may diverge, so
its record has to keep up. Regenerate per the header of
`crates/baton-term/UPSTREAM.diff` and confirm no unexplained delta.

**Everything git tracks is English.** The exceptions are fixtures whose subject
*is* the text, and the list is exact:

    git ls-files | xargs grep -l '[가-힣]'

Nine files are legitimate: four `baton-term` test fixtures, four `baton-winit`
files carrying upstream text and IME evidence, and **this file** -- a check for
those characters necessarily contains them, which is the same data exception the
fixtures rely on. **A tenth is a failure**, and so is a list of eight: if a file
dropped off, the allowlist in `CLAUDE.md` section 7c is now over-permissive and
gets narrowed in this commit.

**Test flakiness is a failure, not a retry.** Run `cargo test --workspace` more
than once. A test that passes on the second attempt is a defect report: this
repository has already had a gauge underflow surface as an assertion failure in
an unrelated test, and re-running would have hidden it. Find the cause.

## 2. Cross-model review

`gpt-5.6-sol` at xhigh, read-only, against `CLAUDE.md` and
`.claude/skills/codex/review-checklist.md`. **Run it in the background** and set
`CODEX_TIMEOUT=1800` as a circuit breaker.

    .claude/skills/codex/codex-run.sh start --role review "#<n>" \
        "Suite: fmt/clippy/test/build all green. <anything it should know>"

Pass the suite result in the extra context. The reviewer is told not to hunt for
coverage gaps, and knowing the suite passed keeps it from guessing.

### The loop

- `APPROVED` -- go to step 3.
- `REQUEST_CHANGES` -- **engage critically.** Open every `file:line` it cites
  before agreeing. A different model is valuable because it does not share the
  implementation's blind spots; the same property means it does not share its
  context either, so some findings are it missing something. Fix the real ones.
  Push back on the wrong ones with the argument, once. Then resume:

      .claude/skills/codex/codex-run.sh resume --role review \
          --notes "fixed A and B at <file:line>; C is intentional because <reason>" \
          "#<n>"

- `NEEDS_REWORK` -- structural. Surface it to the user before mass-editing;
  this is one of the four things that stops a run.

**Three rounds, then adjudicate.** If findings still stand after three, decide
each one yourself: fix it, or record why it stands, in the ledger with a
`Ruling:` line. A review loop that never converges is a review loop being
argued with rather than used.

## 3. Commit and open the PR

Squash the batch checkpoints into commits that each say why, not what. The
message is prose someone reads in a year -- what changed for a person using the
app, and what the alternative was.

    Refs #<n>          # in the message of any commit that is not the last

Then the PR, whose body is the template at `.github/pull_request_template.md`:

    gh pr create --fill --body-file <(...)   # keep the template's structure

`Closes #<n>` in the body. **Only tick a checkbox you actually verified** --
that is the whole reason the list exists. Attach the measurement if the ticket
was on a performance floor; numbers, not adjectives.

## Done when

The suite is green including the repository-specific checks, the review returned
`APPROVED` or every finding has a fix or a recorded ruling, and the PR is open
with `Closes #<n>` and an honestly-ticked checklist.
