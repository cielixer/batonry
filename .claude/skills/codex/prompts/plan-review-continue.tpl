The plan `{{TARGET}}` has been revised since your last review. Its current
text has been re-fetched to `{{TICKET}}` -- read that, then produce an
incremental review. **No network: do not try `gh`.**

1. For each prior finding: quote it briefly, then say **addressed**,
   **not addressed**, or **partially addressed**, pointing at the text that
   resolved it.
2. Flag anything **new** the revision introduced -- a revision that fixes one
   problem by creating another is common, and this is the turn that catches it.

## Notes from the requester

{{NOTES}}

Findings marked here as intentional, or deferred with a reason, should not be
raised again. Disagreeing once, with the argument, is fine; repeating the
original finding is not.

Same severities and the same gate as the first turn. `CLAUDE.md` remains the
authority.

End with exactly one tag on its own line:
  APPROVED
  REQUEST_CHANGES
  NEEDS_REWORK

## Additional context

{{EXTRA}}
