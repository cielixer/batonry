---
name: baton-plan
description: Use when starting new work on batonry that has no ticket yet, or when an existing ticket is too vague or too large to implement - designs the approach, then writes it into a GitHub ticket reviewed by a second model
---

# Plan work on batonry

Two phases, in this order, and the order is the point: **design, then plan.**
Design decides *what shape* the change is; planning decides *what order* it gets
built in. Doing them together produces a task list that is really a set of
undeclared decisions.

The output is **a GitHub issue**, not a document. This repository decided that
the ticket is the plan: one ticket, one PR, one working session, with four
required sections that a fresh session can start from. There is no `plans/`
directory and adding one would create a second source of truth.

## Before anything

Read `CLAUDE.md`. Not skimmed -- it is the implementation contract, and its
section 4 lists eleven architecture rules whose violation means the work gets
redone. Design that ignores it produces plans that fail review before they fail
tests.

Then read the crate contract for whichever crate is likely to own this work.
`baton-action`, `baton-ui`, `baton-ssh`, `baton-store` and `baton-term` each have
a `CLAUDE.md` of their own, indexed in the root's section 1. Deciding which crate
owns a piece of work is phase 1's job, and it cannot be done well without the
rules that crate carries.

Then check what already exists:

    gh issue list --milestone "M1 - SSH client" --state open
    gh issue view <n>          # if a ticket already exists for this

## Phase 1 -- Design

**Do not open an editor.** The question here is not "what files change" but
"what is the shape of this, and what does it cost".

1. **State the problem in one paragraph**, in terms of what a person can see or
   do afterwards. If that paragraph is hard to write, the work is not understood
   yet and no amount of task breakdown will fix it.
2. **Find the constraints that already exist.** Which of A1-A11 does this touch?
   Which crate owns it, given that `baton-core` has no UI or I/O, `baton-action`
   does not know `iced` exists, and `baton-term` does not know `baton-core`
   exists? Which performance floor in section 6 applies?
3. **Consider at least two shapes**, and write down why the loser lost. This is
   the single highest-value artifact of the phase: in six weeks the question
   will be "why is it like this", and the answer needs to exist somewhere.
4. **Inventory the specification sites.** For any grammar, schema, registry
   table, or contract vocabulary this touches: name **one canonical source**,
   then every mirror and every generator of a mirror. Put that list in the
   ticket's definition of done so the gate can check each site changed or is
   explicitly unaffected.

   This is the failure this project keeps hitting. The `when` grammar lived in
   four places and they disagreed, which cost a Critical at gate; a fifth site
   was a **generated** HTML page whose source was a Python generator, and a grep
   did not find it. Enumerating by hand is cheaper than building a scanner, and
   what it gives up is discovery of mirrors nobody knew about.

5. **Name the new words.** Any term this introduces that a reader would have to
   ask about -- a public type, a concept, a piece of project vocabulary -- gets
   listed here and lands in the ticket's definition of done, so the gate can
   check each one is defined in the owning crate's `//!` docs. The first ticket
   through this workflow shipped seven new public types and defined none, and it
   surfaced as a reviewer saying the words were unfamiliar.

6. **Name what you are not doing** and why it is safe to defer.

Ask the user when a choice is theirs -- a product behaviour, a tradeoff between
things they value differently, anything touching what the app *is*. Decide
yourself when it is a technical consequence of constraints already written down.
Guessing at the first kind wastes their week; asking about the second wastes
their afternoon.

**A decision made here goes in `DECISIONS.md` as one line, not in the ticket.**
Issues are work units; decisions have their own canonical home, and a decision
buried in an issue comment is one nobody finds when it needs reversing.

## Phase 2 -- Plan

Write or update the ticket. Four sections, all required:

1. **What** -- one paragraph. What is visible when this is done.
2. **Read first** -- three to five pointers. More than five means the ticket is
   too big. **Put the substance inline, not just links.** The planning notes are
   not in the repository, so a ticket that only points at them cannot be read by
   anyone else, or by a fresh session, or by the reviewer.
3. **Architecture contract** -- which of A1-A11 this is on, and the failure each
   prevents. "None" is an acceptable answer, written explicitly.
4. **Definition of done** -- checkboxes. Each one verifiable, each one something
   a person could disagree about. Copy the specific performance floor or
   regression item by number. Always include clippy and test, **list the
   specification sites from phase 1 step 4**, and **list the new words from step
   5**, so the gate can check both.

**Every checkbox here gets walked and ticked at the gate**, so write them as
things someone can point at. "Correct" and "clean" are not gates; a named test
or a command with an exit status is.

Size it so it is one PR. If it is not, split it and say where the seam is:

    gh issue create --template ticket.yml
    gh issue edit <n> --add-label "stage:N,area:<crate>,size:S|M|L" \
                      --milestone "M1 - SSH client"

Sub-issues attach to the stage epic:

    gh api repos/cielixer/batonry/issues/<epic>/sub_issues -F sub_issue_id=<db-id>

## Phase 3 -- Have a second model review the plan

Cheaper than finding the same problem in code. **sol opens and closes the
phase; Fable carries the revision loop between them** -- an xhigh run per
round is what made review the dominant Codex cost, and the loop only needs a
fresh pair of eyes, which Fable has (Opus when Fable is unavailable; their
usage is metered separately from Codex).

**First review: `gpt-5.6-sol`** at xhigh, in a read-only sandbox, against
`CLAUDE.md` and the checklist. **Run it in the background** -- xhigh reviews
routinely outlast a foreground command timeout:

    cat > /tmp/plan.txt <<'BRIEF'
    <what the reviewer should know that the ticket does not say>
    BRIEF
    .claude/skills/codex/codex-run.sh start --role review --prompt plan-review \
        --extra-file /tmp/plan.txt "#<n>"

All free-form prose goes through `--extra-file`, never argv: one line is enough
to contain a backtick or a `!`, and a real run lost instructions that way.

Read the trailing tag:

- `APPROVED` with no revisions -- start implementing with `baton-ticket`.
- `REQUEST_CHANGES` -- **engage, do not comply.** Read the code it cites before
  agreeing. A cross-model review is valuable exactly because it does not share
  the planning model's blind spots, which also means it does not share its
  context: some findings are it missing something. Fix the real ones, push back
  on the wrong ones with the reason.
- `NEEDS_REWORK` -- the approach is wrong. Go back to phase 1 rather than
  patching the ticket.

**The revision loop runs on Fable, not sol**: each revised ticket goes to a
fresh-context agent of the session's model, read-only, against the same
contract and checklist, until it returns `APPROVED`.

**Then sol confirms the final ticket once.** Resume its thread carrying what
changed and why -- it reads a delta, not the ticket twice -- and its
`APPROVED` closes the phase:

    .claude/skills/codex/codex-run.sh resume --role review --prompt plan-review \
        --notes-file /tmp/notes.txt "#<n>"

**Three rounds is the ceiling**, as at the gate. After that, Minor and
Suggestion findings may be closed by a `Ruling:` in the ledger, and a **standing
Critical or Major may only be closed by a line in `DECISIONS.md`** with the
reviewer told why in the next notes. A review that will not converge is being
argued with rather than used -- but a reviewer that cannot be overruled blocks on
its own misdiagnosis, which has already happened here once, so the escape hatch
stays and is made public instead of private.

Skip this phase only for a ticket that is a few mechanical lines. Anything
touching an architecture contract goes through it.

## Done when

The ticket exists, its four sections are filled, it lists the specification
sites it touches, its labels and milestone are set, the plan review returned
**`APPROVED`** -- with no standing Critical or Major, since this skill does not
get to be more permissive than the checklist it points at -- and any decision it
produced is a line in `DECISIONS.md`.

Then: `baton-ticket #<n>`.
