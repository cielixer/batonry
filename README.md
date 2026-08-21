# Baton

**One window for terminals spread across many servers — and a way to hand a task from one server to the next.**

> **Status: pre-alpha. There is nothing to install yet.**
> The window arrives in stage 1. What exists today is the terminal grid, a
> patched input stack, and the measurements that decided the architecture.
> Watch the [M1 milestone](https://github.com/cielixer/batonry/milestone/1) if
> you want to know when it becomes usable.

macOS client, Rust, [iced](https://github.com/iced-rs/iced). MIT.

---

## Why

The first version of this idea was abandoned for a boring reason: **the terminal
kept breaking.** Splitting a pane left it blank. A dropped connection turned a
pane into a dead end with no way back. Resizing tore the grid. No amount of
clever session management on top of that is worth using every day.

So the order is inverted. **Build a terminal good enough to delete the others
first, then put the interesting parts on top.** Milestone 1 is that terminal
plus host management — enough to replace an SSH client outright. Session
persistence, agent sessions, and handoff come after, and only after.

---

## Two things here that might be worth your time

Even at this stage, two problems were interesting enough to write down
carefully. Both are upstream problems, not ours, and both are reproducible.

### A deadlock in a terminal widget, and how it was pinned down

The starting point was a hardcopy of `iced_term` 0.8. Under sustained output it
wedges permanently — 100% reproducible with `yes | head -c 100000` in a single
pane.

The cause is one line: `blocking_send` on a bounded channel, called from the PTY
read path **while the `FairMutex<Term>` grid lock is held**. The only task that
can drain that channel is the UI thread, and the UI thread is blocked waiting
for the same lock. Neither side can move. It was found by reading the stacks out
of `sample(1)` rather than by guessing.

The fix is not a bigger channel. A wakeup means *"something changed"*, so it is
**coalesced at the source and dropped when it would block** — but events that
carry information (`Title`, `Exit`, PTY replies) must never be dropped, so they
take a separate unbounded path. Two channels, drained with a `biased` select.

The part worth copying is the test. Sustained output alone does not reproduce
it: a consumer that never stalls almost never opens the window. The regression
test injects 4 ms of consumer delay to stand in for render cost, brackets 2 MB
of output with OSC title markers, and asserts the closing marker arrives — the
exact event the deadlock destroyed. It was verified by putting `blocking_send`
back and watching the test fail, which the first version of the test did not do.
There is also an assertion that scans the source so the call cannot come back.

→ [`crates/baton-term/tests/deadlock.rs`](crates/baton-term/tests/deadlock.rs)

### Korean input on macOS, which no amount of application code could fix

Typing Hangul into any `winit`-based application on macOS produces `ㅎㅏㄴ`
where it should produce `한`. The jamo never combine.

The usual assumption is that this is a preedit problem — the application is
mishandling marked text. It is not. **The macOS Korean input method does not use
marked text at all.** It commits each jamo immediately with
`insertText:replacementRange:`, then reaches *back* into the document to replace
what it already wrote. That requires the application to answer
`selectedRange`, `markedRange`, and `attributedSubstringForProposedRange:`
truthfully. `winit` 0.30.13 answers the last one with `nil` and reports no
selection, so the input method concludes there is nothing to revise and gives up
on combining.

Three wrong theories died on the way, each to a control run: it is not caused by
switching input sources (it reproduces with zero switches), it is not the
input-router rule that was originally blamed, and `osascript keystroke` cannot
test it at all because it injects Unicode directly and bypasses the IME — the
whole first round of experiments measured nothing. TextEdit, given byte-identical
synthetic keystrokes, produces `한`. Upstream's own proposed patches still
produce `ㅎㅏㄴ`.

The fix exposes a document to the input method that is **only the text currently
being composed**, so every replacement range the IME asks for falls inside it.
That is 207 added lines in a single file, and it removed the need to fork
anything above it — no `iced` patch, no change to the terminal widget. It is
covered by two recorded event streams, one from before the fix and one from
after, replayed in a test.

→ [`crates/baton-winit/`](crates/baton-winit/) · the delta is in
[`UPSTREAM.diff`](crates/baton-winit/UPSTREAM.diff), all of it in
`src/platform_impl/macos/view.rs`

---

## What is measured, and what it decided

Three questions were answered by experiment before any application code was
written, because each one could have invalidated the whole stack.

| Question | Answer | What it changed |
|---|---|---|
| Can the framework's own text rendering carry a terminal grid? | Yes, with glyph-run batching and damage-based caching: `fill_text` calls down 115×, per-pane geometry p99 **47.8 µs**, twelve panes together **574 µs = 3.4%** of a 16 ms frame. 60 fps under heavy output, idle CPU **≤0.4%** | Closed the planned escape hatch to a custom `wgpu` shader renderer. It is not needed |
| Is the framework's split-pane widget usable, or does resizing flood it? | Yes — exactly **one message per pointer move**, and rebuilding the view for twelve panes costs p99 **25 µs** (0.16% of a frame) | Deleted the plan to write a split layout by hand |
| Does Korean input work? | **No** — and there was no way around it, since the client is macOS-only | Became the fix above |

One measurement was surprising enough to change how benchmarks are written:
**macOS PTY throughput is governed by line count, not byte count.** `yes` (2-byte
lines) moves 1.9 MiB/s where dense 150-byte lines move 89 MiB/s — a 47× spread on
identical hardware. Benchmark with `yes` alone and output batching looks like a
regression.

Pixel-comparison tests were tried and **removed**. They were not reproducible
across machines even with a software rasteriser and bundled fonts, and they
encoded known-wrong behaviour as if it were correct. Correctness is asserted
against a text dump of the terminal grid instead.

---

## How it is put together

Hexagonal, with the Elm architecture as the driving adapter.

```
crates/
  baton            main() only: inject adapters, run
  baton-ui         screen assembly, projection, Elm wiring   (no main, so it is headless-testable)
  baton-core       the hexagon: domain types and port traits  (no UI, no I/O)
  baton-action     action registry, keymap, when-clauses, input router
  baton-term       the terminal grid widget
  baton-ssh        system ssh as a subprocess: ControlMaster, ProxyJump
  baton-store      SQLite, export/import
  baton-platform   the only crate allowed to contain #[cfg(target_os)]
```

Three rules do most of the work:

- **Every action goes through a registry.** A UI element never calls a
  function — it emits an action id. Keys, clicks, menus and the command palette
  all converge on the same id, which is why the palette is a search over the
  registry rather than a parallel wiring of the app. Anything missing from the
  palette is a bug, not a gap.
- **Input goes through a router.** A pane is a sink, not an owner. Writing
  `pane.on_key() → pty.write()` once makes multi-pane broadcast impossible to
  retrofit, so the router exists from the start even though M1 ships no
  broadcast UI.
- **Terminal sessions are driven by bytes, behind one trait.** The core is never
  handed a PTY handle, because the second implementation gets its bytes from
  somewhere else entirely.

Three crates are hardcopies of upstream, kept verbatim except for a recorded
delta so the diff stays reviewable: `baton-winit` (the IME fix, 207 lines),
plus `baton-iced-winit` and `baton-iced`, whose sources are byte-identical to
upstream and differ only in one dependency line each — Cargo can rename a
dependency but cannot rename a patched package, and that is the only reason they
are here.

---

## Deliberately not doing

Not "later" — these are structural.

| | Why |
|---|---|
| **No server, no account, no relay** | The only authentication surface is your own SSH. Neither code nor credentials pass through a third party |
| **No credential bytes stored** | Names and how to obtain them, nothing more. Passphrases go to the OS keychain or `ssh-agent` |
| **No inbound ports** | |
| **No agent forwarding by default** | Anyone with root on the remote can impersonate you. Jump hosts instead |
| **Sync, team vaults, session recording, SSO** | All require a server. Share configuration as text files in git |
| **Mobile** | The SSH key would have to live on the phone |
| **Windows** | Connection reuse is Unix-only by construction |

---

## Building

Requires a recent stable Rust and macOS.

```sh
cargo build --workspace
cargo test  --workspace
```

`cargo run` currently prints a line and exits — see the status note at the top.

The three hardcopied crates carry `disable_all_formatting` so `cargo fmt` cannot
quietly rewrite them, which would destroy the verbatim property their diffs
depend on. They are excluded from the workspace and reached through path
dependencies.

## License

MIT. See [`LICENSE`](LICENSE). Hardcopied crates keep their upstream notices in
each crate's `NOTICE.md`.
