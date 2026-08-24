# baton-term -- the implementation contract

**This file supplements the repository's [`CLAUDE.md`](../../CLAUDE.md).** The root contract governs the whole project; this one holds rules that apply to the terminal widget alone. On a conflict, the root wins.

`baton-term` started as a hardcopy of `iced_term 0.8` (MIT). **The divergence from upstream is recorded in `crates/baton-term/UPSTREAM.diff`, and there is no separate `vendor/` directory.**

**This crate does not know `baton-core`** (root §2). It is a terminal widget and nothing more, so that it stays extractable.

**Paths are written relative to the repository root.** `DECISIONS.md` and `evidence/*` are under `docs/milestones/01-ssh-client/`.

---

## 1. The terminal grid

1. **The escape hatch is `iced::widget::shader`.** It provides `Primitive::prepare(&Device, &Queue, …)`, `draw(&mut RenderPass)` and `render(&mut CommandEncoder, …)`, so both a RenderPass and a CommandEncoder are available. Drop to this if the framework's text cannot hit the performance floor.
2. **Do not open the `sugarloaf` path** (measured in S2, `DECISIONS.md` #50). The framework's text plus glyph-run batching uses 3.4 % of the 16 ms budget at twelve panes. If frames drop in real use, drop to 1 above instead.

**These are the rules S2 settled by measurement. Breaking one is a rejection.**

- **Do not wait on a channel while holding a lock.** The original `iced_term` called `blocking_send` inside a `FairMutex<Term>` and deadlocked (100 % reproducible with `yes | head -c 100000`, #49). `Wakeup` means "something changed", so **coalesce it at the point of emission and drop the overflow.** Do not let an event that must not be dropped, like `Title` or `Exit`, be pushed out by wakeups.
- **Draw a double-width character centred across both cells.** The original centres it in the first cell, so the glyph bleeds into the left cell and leaves a gap on the right.
- **We draw the underline variants (`4:3` undercurl, `4:4` dotted, `4:5` dashed, `21` double) and the underline colour (`SGR 58`) ourselves.** The original looks only at `Flags::UNDERLINE`, draws a straight line, and draws **nothing at all** for the rest. Confirmed by grid dump.
- **Procedural box drawing is not optional.** `┌──┬──┐` left to the font was confirmed not to meet at cell boundaries. Budget roughly 1,000 lines.
- **Measure performance under two kinds of load.** pty throughput on macOS is dominated by **line count**, not bytes, so `yes` and dense text differ by 47×. Measured with `yes` alone, batching looks like a loss. **The unit is source bytes, and it is never mixed with pty bytes** (ONLCR inflates by 1.43×).

**These are the performance rules. Breaking one is a rejection.**

- **Do not reshape text every frame.** Lapce calls `TextLayout::new()` every frame for every line and pays for it. **A per-row cache or run-length batching is mandatory.**
- Merge backgrounds into one quad where the colour and row match and the columns are contiguous (RLE).
- Shape glyphs in one pass where the style matches and the columns are contiguous. Force the advance to the cell width to guarantee monospace alignment.
- Skip a wide char's spacer and the blank after an emoji variation selector. Take the width information exactly as `alacritty_terminal` reports it.
- Render from damage. With no output, do not redraw.
- Coalesce reads on **1 MiB or 5 ms**, whichever comes first.

**These are reference implementations: read them, do not copy them, and check the licence.** The RLE batching in Zed's `crates/terminal_view/src/terminal_element.rs` lines 480 to 560, and the `CachedRow::TextBlobs` row cache in Freya's `crates/freya-terminal/src/rendering.rs`.

## 2. IME -- solved (G1). The rules still stand

**Korean composition on macOS was fixed in `crates/winit`** (`DECISIONS.md` #55, `evidence/rendering.md` §9.1).
The macOS Korean input source does not use a preedit model; it assembles jamo by **`insertText` with a `replacementRange`, that is, insert and then replace.** Limiting the "document" exposed to the input method **to the string being composed**, and absorbing that replacement inside winit, means the app sees **only the standard `Ime::Preedit` and `Ime::Commit`.**
**iced was not forked and this crate did not change.**

- **The `[patch.crates-io]` pin on `crates/winit` stays. Remove it and Korean breaks again.**
  **There is one hardcopy** (#76). `[patch]` *substitutes* a package but cannot *rename* one, so naming the copy `winit` makes it a one-line job. **Of the three ways to break this, deleting the `[patch]` stanza is the only quiet one** (measured); cargo hard-errors on the other two.
  `crates/baton-term/tests/ime.rs::winit_ime_fix_is_wired_up` asserts it.
- **Carry `crates/winit/UPSTREAM.diff` forward when upgrading winit.** 0.31 is still unfixed.
- **Do not test IME with `keystroke`, meaning unicode injection.** That bypasses the input method entirely. Use `key code`, meaning a virtual key code.

Four implementation rules.

- **A `preedit`'s `selection` is an empty range, that is, a caret.** Used as a highlight span, nothing appears.
- **Hand the runtime `preedit: None`.** We draw into the cell grid ourselves (on-the-spot). Given `Some(..)`, iced floats an overlay above, which does not suit a terminal.
- **No raw key leaks during composition** (measured at zero). winit suppresses the `KeyboardInput` for a key the IME consumed, so do not put a time-based rule like "within the last N ms" into the input router. The one rule to hold is **never send both `Ime::Commit` and the raw key to the pty.**
- **Do not introduce latency for Latin input.** A character that is not a composable Hangul jamo or syllable goes out immediately.

## 3. Render verification -- we do not look at pixels

**The canonical check for terminal render is the grid dump** (`Terminal::dump_grid()`, `DECISIONS.md` #57).
Assert truecolour, CJK width, box characters and OSC 8 **as text.** No font and no pixel is involved, so macOS, Linux and CI all give the same answer.

- **Do not build pixel goldens.** They were removed on measurement (2026-08-21, `evidence/rendering.md` §9.4a and §9.7). They say only "this changed" and never whether it is correct, so **ours had encoded four known defects** verbatim; the result varies by machine (one missing font weight breaks it); and fixing that means the repository carries a test font forever.
- **There is no `assets/`.** The product uses system fonts. Reintroducing a bundled font means reversing #57 first.
- **The spike crates (`s1-ime`, `s2-grid`, `s3-panegrid`) are not in the repository.** What was kept lives in `crates/baton-term/tests/`: the IME fixtures are `fixtures/hangul-2set.jsonl` and `fixtures/hangul-2set-before-winit-fix.jsonl` with `ime.rs` as the replayer, the ANSI conformance grid dumps are in `vt_conformance.rs`, and the deadlock regression is in `deadlock.rs`. **`cargo test --workspace` runs all of it.** Looking at a render with your eyes becomes possible from stage 3, when the app draws a screen.
