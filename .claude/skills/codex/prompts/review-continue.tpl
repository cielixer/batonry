The change for `{{TARGET}}` has been updated since your last review. Re-run
`git status -s` and `git diff HEAD` for the current state, then produce an
incremental review.

1. For each of your prior findings: quote it briefly, then say **addressed**,
   **not addressed**, or **partially addressed**, with the `file:line` that
   resolved it or failed to.
2. Flag anything **new** the edits introduced, checking against every section of
   `.claude/skills/codex/review-checklist.md` again.

## Notes from the requester

{{NOTES}}

Findings marked here as intentional, as an environment limitation, or as
deferred with a reason should **not** be raised again. Disagreeing is fine --
say so once, with the argument -- but do not simply repeat the original finding.

Same severities and the same approval gate as the first turn. `CLAUDE.md`
remains the authority. Do not review test code.

End with exactly one tag on its own line:
  APPROVED
  REQUEST_CHANGES
  NEEDS_REWORK

## Additional context

{{EXTRA}}
