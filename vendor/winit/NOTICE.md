# `vendor/winit` -- why this is here

A copy of `winit` 0.30.13 (Apache-2.0) with macOS Korean input fixed.
`[patch.crates-io]` points the whole workspace, including `iced`, at this copy.
It is a **path** dependency, not a git dependency, so builds stay hermetic.

- Upstream: crates.io `winit 0.30.13`
- Licence: Apache-2.0 (upstream `LICENSE` preserved verbatim)
- Our delta: **`BATON.diff`**. One file is touched:
  `src/platform_impl/macos/view.rs`
- `examples/`, `tests/`, `docs/` and `benches/` were removed along with their
  manifest entries, to keep the tree small. A dependency only builds its lib,
  so this does not affect the build.

## What is fixed

The macOS Korean input method does not use `setMarkedText` (preedit). It
composes jamo into syllables with `insertText:` plus a `replacementRange`:

```
insertText("ㅎ")                 start of a composition
attributedSubstring(0,1) -> "ㅎ"  it reads back what it inserted
insertText("하", repl=(0,1))     replace that with the composed syllable
insertText("한", repl=(0,1))     and again
insertText(" ")                  outside the composition; the previous commits
```

Upstream winit answers `selectedRange` -> `{NSNotFound, 0}`,
`attributedSubstringForProposedRange:` -> `None`, and ignores
`replacementRange`. The input method therefore cannot read the document, cannot
use that model, and drops the first jamo on the floor: typing 한글 produces
ㅎㅏㄴ글.

| change | what |
|---|---|
| upstream [#4650](https://github.com/rust-windowing/winit/pull/4650), applied as-is | non-empty `validAttributesForMarkedText`; `insertText`'s commit gate split out into `pending_commit` |
| upstream [#4666](https://github.com/rust-windowing/winit/issues/4666) | `selectedRange` stops answering `NSNotFound` (dictation breaks for the same reason) |
| **ours** | a `composing` document plus `committed_utf16`, which **absorbs the insert-then-replace protocol** and surfaces only standard `Ime::Preedit` / `Ime::Commit` |

**The design in one line:** the document we expose to the input method is *only
the in-flight composition*. A terminal grid is not the input method's document,
so there is no reason to show anything else -- and this keeps every replacement
request inside the buffer we hold. That is why **no fork of `iced` is needed and
`baton-term` did not change**: the app sees ordinary preedit and commit events.

`composing` is **not cleared on commit.** Clearing it desyncs our offsets from
the input method's, and the replacement for the next syllable lands in the wrong
place (measured). `committed_utf16` separately records how much already went to
the app.

## Why typing English has no lag

`is_composing_script()` holds back **only Hangul jamo and syllables**. Latin,
digits and punctuation are never revised by the Korean input method, so they go
out immediately. With nothing in flight and a non-composable character, the code
returns early and takes exactly the upstream path -- verified with the ABC
layout, where `a s d SPACE ENTER` all arrive as raw key events and the
composition path is never entered.

## Sending this upstream

#4650 is already open, so only the `composing` / `committed_utf16` part is new
material for a PR. winit 0.31 added
`Ime::DeleteSurrounding { before_bytes, after_bytes }`, but **its AppKit backend
never emits it** (checked in 0.31.0-beta.2), so upgrading does not fix this on
its own. Because our approach never exposes that event to the application, it
ports to 0.31 in the same place.
