# `crates/baton-term` -- why this is here

A copy of [`iced_term`](https://github.com/Harzu/iced_term) 0.8.0 (MIT), the
terminal widget for `iced`, with four things fixed.

- Upstream: crates.io `iced_term 0.8.0`, git
  `e0e90be2c160bd68f1bf544a51bf772d75b98ffc`
- Licence: **MIT, Copyright (c) 2024 Ilia Shvyrialkin.** The upstream notice is
  preserved verbatim in this crate's [`LICENSE`](LICENSE), which is the file
  that governs this directory. The repository's root `LICENSE` covers our own
  code
- Our delta: **[`UPSTREAM.diff`](UPSTREAM.diff)**, which also documents how to
  regenerate it from the published crate. Every changed line is additionally
  marked `// BATON:` in the source

## Why it is a copy and not a dependency

Unlike the three `baton-{winit,iced-winit,iced}` hardcopies -- which exist only
because Cargo cannot rename a patched package, and whose `src/` is byte-identical
to upstream -- **this crate is expected to diverge.** It is the terminal grid,
which is the part of Baton that has to be ours: procedural box drawing, the
underline variants, wide-character centring and the damage model are all work
upstream does not do. Starting from a working VT widget was worth more than
starting from nothing, but the destination is our own code.

That is also why the manifest says `edition = "2021"` with a note to raise it
later, and why the `// BATON:` markers exist: while the delta is still small
enough to read, it should stay easy to see what is upstream's and what is ours.

## What is fixed

| | what |
|---|---|
| **deadlock** | `blocking_send` was called from the PTY read path while `FairMutex<Term>` was held, and the only task that can drain that channel is the UI thread -- which is waiting for the same lock. 100% reproducible with `yes \| head -c 100000`. The channel is now split: `Wakeup` is coalesced at the source through a capacity-1 channel and dropped when it would block, everything else takes an unbounded path so it can never be lost. Regression: [`tests/deadlock.rs`](tests/deadlock.rs) |
| **damage** | The original cleared the render cache on *every* command. Now `Term::damage()` decides, so panes with no output do not re-tessellate |
| **glyph batching** | Runs of cells sharing a style are shaped once instead of per cell, and same-colour background spans merge into one quad. `fill_text` calls dropped 115x |
| **no panics** | Two `panic!`s on channel close were removed. One message must not be able to kill the app |

Not yet done, and tracked separately: the widget still owns its PTY, which the
architecture forbids -- byte sources have to be swappable. That moves behind a
trait in stage 2.

## Test data contains Hangul and CJK

`tests/fixtures/*.jsonl`, `tests/ime.rs` and `tests/vt_conformance.rs` contain
Korean and Han characters on purpose: they are recorded input-method event
streams and a wide-character conformance page. Everything git tracks is English
except data whose whole point is that it is not.
