Continue the implementation session for `{{TARGET}}`.

The working tree has probably changed since your last turn: the requester
reviews each batch and fixes problems directly. Run `git status -s` and
`git diff HEAD` to resync before doing anything. **Treat the current tree as
authoritative and do not revert the requester's edits.**

## Review notes from the requester (may be empty)

{{NOTES}}

These say what was fixed after your last turn and why. Any convention stated
here is binding for the rest of this session -- do not reintroduce a corrected
pattern.

## New instructions

{{EXTRA}}

Same rules as the first turn, and they still bind: stay inside the stated
scope; never touch `crates/baton-winit`, `crates/baton-iced`, or
`crates/baton-iced-winit`; mark any `crates/baton-term` change with `// BATON:`;
everything git tracks is English; no tests; no commits, tags, versions, or
README edits; leave clippy and build green.

Same report format -- files changed, deviations, leftovers, behaviour needing a
test, clippy and build status -- ending with exactly one tag on its own line:
  IMPLEMENTATION_COMPLETE
  IMPLEMENTATION_PARTIAL
