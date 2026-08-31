# CLAUDE.md -- the implementation contract

**This file is for agents. A person does not have to read it.**
The prose written for people is in `docs/README.md` and `docs/milestones/*/`, which git does not track. This file is what those documents decided, restated as rules that can be enforced.

- The project is **Baton**. It handles terminals and AI agent sessions across several servers in one window, and hands work from one server to the next.
- The current milestone is **M1 -- the SSH management terminal** (`docs/milestones/01-ssh-client/`).
- The language is Rust. **This app is a client, and it prefers macOS without being macOS-only.** What it connects to over SSH prefers Linux. Do not mix the two axes: for `sshd`, `tmux` and terminal compatibility, look at Linux first.

  **The dividing line is "is it complicated"** (#75). **What is complicated looks at macOS alone**: IME, the keychain, the single-instance lock, connection reuse. Connection reuse is structurally Unix-only. **What is predictable is written platform-neutral from the start**, and the keymap is the example: physical key codes are the same everywhere (#72) and modifiers use the W3C names (#74). Neutral now is cheaper than neutral later, and **only when it costs nothing**.

  **Where a platform decision is needed and there is no answer yet, block it at compile time rather than with a runtime panic.**
  ```rust
  #[cfg(target_os = "macos")]
  pub const PRIMARY: Modifiers = Modifiers::META;
  #[cfg(target_os = "windows")]
  pub const PRIMARY: Modifiers = Modifiers::CONTROL;
  #[cfg(not(any(target_os = "macos", target_os = "windows")))]
  compile_error!("primary modifier undecided for this target; decide it here");
  ```
  Do not use `todo!()` or `unimplemented!()`. Those are runtime and the build passes anyway. **Not writing `_` when matching a platform enum** is the more Rust-shaped version of the same idea: add a variant and the build breaks everywhere that has to handle it.
  **Every `#[cfg(target_os)]` still lives in `baton-platform` alone** (§2), and so does the example above.

---

## 1. The stack (settled)

```
iced 0.14          MIT, crates.io, wgpu backend, declarative (Elm)
iced_aw 0.14       MIT -- only widgets iced itself lacks: tabs, context menus, cards, badges
alacritty_terminal 0.26  Apache-2.0 -- VT parsing and the grid
```

**egui, glyphon and gpui are not used.** The reasons they were rejected are in [`DECISIONS.md` #1](docs/milestones/01-ssh-client/DECISIONS.md).

**A modal does not need `iced_aw`.** Build it from 0.14's own `Stack` and `Float`. 0.14 added `Table`, `Grid`, `rich_text`, an animation API, IME and headless testing, so **check iced itself before reaching for `iced_aw`.** The widget correspondence table, and what is missing relative to HTML, are in the [`evidence/rendering.md` appendix](docs/milestones/01-ssh-client/evidence/rendering.md).

**A reference implementation is read, not copied, and its licence is checked first.** This holds for anything outside the dependency list above -- another terminal's render loop, someone else's tooling. Adopting a design is fine; adopting the code is a licence question that gets answered before the first line is pasted.

**Accessibility, meaning screen reader support, is out of scope for M1.** iced does not support it either (iced-rs/iced#552, open since 2020) and **nothing is designed on the assumption that it lands soon.** The ban on hover-only affordances still holds.

### Crate contracts

Rules that apply to one crate live with that crate, and the harness loads them when a file in that directory is read. **This index exists because planning happens before any file is opened** -- deciding which crate owns a piece of work should not require guessing what rules it carries.

| File | What it holds |
|---|---|
| [`crates/baton-action/CLAUDE.md`](./crates/baton-action/CLAUDE.md) | the action table, the registry, the keymap, `when` clauses, the input router |
| [`crates/baton-ui/CLAUDE.md`](./crates/baton-ui/CLAUDE.md) | UI rules, the theme, view state, sidebar and pane behaviour, layout regressions |
| [`crates/baton-ssh/CLAUDE.md`](./crates/baton-ssh/CLAUDE.md) | the enforced `ssh` configuration, `ControlMaster`, `ProxyJump`, error classification |
| [`crates/baton-store/CLAUDE.md`](./crates/baton-store/CLAUDE.md) | the SQLite schema, inheritance, workspaces and scratch, export and import |
| [`crates/baton-term/CLAUDE.md`](./crates/baton-term/CLAUDE.md) | the terminal grid, IME, render performance, render verification |

`baton`, `baton-core` and `baton-platform` carry no file of their own: what governs them is §2 and §4, which are cross-cutting and stay here.

**One line stays here rather than moving with its crate. Do not remove the `[patch.crates-io]` pin on `crates/winit`; removing it breaks Korean input again** (#76). The person editing the workspace `Cargo.toml` is not inside `baton-term` and would never see the rule there.

## 2. Crate layout

Start as one cargo workspace and split a crate into its own repository once it has proven general. **The shape is Hexagonal (ports and adapters) plus Elm.** The wiring rules are in [`sys/software-design.md` §1](docs/milestones/01-ssh-client/sys/software-design.md) and the background is in [`docs/primer/elm.md`](docs/primer/elm.md).

```
batonry/
  Cargo.toml                  [workspace] members = ["crates/*"], exclude = ["crates/winit"]
  crates/
    baton/          one main.rs. Injects adapters and runs. No logic
    baton-ui/       iced screen assembly, projection, Elm wiring   <- no main, so it is headless-testable
    baton-core/     the hexagon. The domain, and port traits
    baton-action/   action table, registry, keymap, when clauses, input router
    baton-term/     the iced terminal widget (started as an iced_term hardcopy)  -> extraction candidate
    baton-ssh/      the system ssh process, ControlMaster, ProxyJump
    baton-store/    SQLite, export/import
    baton-platform/ keychain, single-instance lock, paths, clock, uuid
    winit/          one hardcopy. Substituted via [patch.crates-io] (#76)
```

**A hardcopy records its origin inside the copy.** `crates/baton-term/UPSTREAM.diff` and `crates/winit/UPSTREAM.diff`, each alongside its own `LICENSE` and `NOTICE.md`. **There is no separate `vendor/` directory.**

**Dependency direction.**

- **`baton-core` depends on no UI and no IO.** Inheritance resolution and the domain rules have to be testable as pure functions.
- `baton-action` knows only `baton-core`. It does not know iced exists, because an action is data.
- `baton-term` does not know `baton-core`. It is a terminal widget and nothing more, so that it can be extracted.
- **`baton-ui` has no `main`.** With a `main` it cannot be driven by `iced_test`'s headless `Simulator`.
- **Every `#[cfg(target_os)]` lives in `baton-platform` alone.** An OS branch appearing in any other crate is rejected.
- **`update` calls a domain method. It does not assign to a domain field.**
- **`view()` takes a projection, not a domain type.** The projection is a pure function in `baton-ui`: `fn project(&State) -> Projection`.
- **Do not add more crates.** A new boundary exists only when there is *one rule to enforce across it*. If the rule cannot be written down, a folder is enough.
- **`baton-action` is not going to be split into its own repository** (#77), which is why the action table lives inside that crate.
- **A port exists only when the thing it replaces already has a name.** That is `Substrate` (tmux, in M2), `Store`, and `Clock` and `Ids` for test determinism. "We might swap it later" is not a reason.

**No multi-process architecture and no shared-memory IPC (iceoryx2 and the like).** The seven reasons for rejecting it are in [`sys/system-design.md` §4](docs/milestones/01-ssh-client/sys/system-design.md).

## 3. Never

1. **Do not open an inbound port.**
2. **Do not enable SSH agent forwarding by default.** It is opt-in per host, and turning it on shows the risk.
3. **Do not store credential bytes.** Store paths and references; delegate secrets to the OS keychain or `ssh-agent`.
4. **Do not treat the user's `~/.ssh/config` as canonical.** Import from it, but do not let it drive connection behaviour.
5. **Do not link an SSH implementation of our own.** Use the system `ssh` as a subprocess.
6. **Do not proceed quietly when a host key has changed.**
7. **Write nothing but hostnames and usernames into logs and diagnostic files.** Mode is `0600`.
8. **No Sixel and no Kitty graphics in M1.**

## 4. Architecture contracts -- break one now and rewrite later

| # | Rule | What breaking it costs |
|---|---|---|
| A1 | A terminal session is created **only behind the `Substrate` trait**. M1 has one implementation, `SshPtySubstrate` | M2 has nowhere to insert tmux |
| A2 | **Drive `alacritty_terminal` with bytes.** `Processor::advance(&mut self, handler: &mut H, bytes: &[u8])`: the `&mut` is the handler and the bytes are an immutable slice. Do not hand the core a PTY handle | in M2 the bytes start coming from tmux's `%output` while the core demands a PTY |
| A3 | **Splits, tabs, scrollback, search and copy belong to Baton.** When tmux arrives in M2 we still do not use its windows, panes, layouts or copy-mode | layout mirroring bugs by the dozen. cmux hit twelve in one day and then rewrote from the bottom |
| A4 | `Host` keeps **`parent` (the management relation) and `jump` (the connection path) separate** | M5's tree aggregation starts with a schema migration |
| A5 | A session identifier is **one extensible string**. It has to be able to grow into `baton/<user>/<track>/<n>`. **The UI does not use a session id as a database primary key** | in M2 the tmux session name becomes a join key that cannot be changed |
| A6 | **Persist `launch_spec` completely.** M1 only has a shell command and an environment, but the schema is built now | in M3 `--resume` fails to restore `--mcp-config`, `--settings`, `--add-dir` and the permission mode, so session restore is quietly wrong |
| A7 | **Size calculation does not depend on the result of a render.** The inputs are *the window geometry, fixed chrome constants, and cell metrics*, and nothing else | a resize feedback loop. cmux grew about 19pt per pass |
| A8 | Correctness is a **settled property**. It may be wrong mid-drag and mid-resize as long as it is right once the gesture ends | synchronous sizing occupies the main thread |
| A9 | **The shell layout has two docks, left and right, from the start.** The right one may be permanently collapsed in M1 | attaching the right dock in M2 means reworking the centre's size calculation and pane splitting |
| A10 | **Every behaviour goes through the action registry.** See `crates/baton-action/CLAUDE.md` | building the palette becomes a rewiring of the whole app |
| A11 | **Input goes through the router.** See `crates/baton-action/CLAUDE.md` | broadcast cannot be inserted later |

```rust
trait Substrate {
    fn spawn(&self, place: &Place, cmd: Command) -> Result<SessionId>;
    fn attach(&self, id: SessionId) -> Result<ByteStream>;   // -> &[u8], raw
    fn send(&self, id: SessionId, bytes: &[u8]) -> Result<()>;
    fn resize(&self, id: SessionId, cols: u16, rows: u16) -> Result<()>;
    fn resync(&self, id: SessionId, lines: u32) -> Result<Snapshot>;
    fn list(&self) -> Result<Vec<SessionMeta>>;
}
```

**A local shell goes through the same trait.** Local and remote do not get separate code paths.

## 5. Language -- every character git tracks is English

**There are no exceptions.** Code, comments, doc comments, commit messages, pull requests, issues, comments in `Cargo.toml`, the templates under `.github/`, and our own comments inside a hardcopy.

- **`docs/` is not tracked** (`.gitignore`), which is why it is written in Korean: it is planning and design prose for a person, and the canonical record is GitHub issues. **Every `CLAUDE.md` *is* tracked**, so every one of them is English. A contract that gates every pull request should itself be reviewable in one.
- **Test data is not an exception; it is a different category.** Testing Hangul composition needs `한글` and testing CJK width needs `漢字`. **Prose is English and data is whatever characters it needs to be.** A comment has to make clear that it is data.
- **Quoted measured output is data too.** When `crates/winit/NOTICE.md` explains the IME defect by writing that `ㅎㅏㄴ` appears instead of `한`, that is not prose but **evidence**, and translating it destroys the point. The test is the same one: allowed **only when the characters themselves are the subject**.
- **A comment says what the code is, not what it used to be.** Change history, withdrawn designs, and what a review proposed and lost belong in `DECISIONS.md` and in the commit message. Delete a sentence that describes a design no longer present ("an earlier shape pushed the rows out") or defends a decision already made ("named `resolve` rather than `id` because…"). Length itself is not the problem: a measured number, a hazard, or why a cast would be wrong stays even when it runs long.
- **A surprising rule goes inline, beside the line that implements it.** Not a paragraph above the function. The shorthand accepting `a` and rejecting `A` is the example, and two lines cover it. That line is where someone would go to break the rule.
- **There is a reason for all of this.** This repository will be public, and our patch to `crates/winit` is a candidate to send upstream. Translating later always costs more than writing English now.
- **This is how it is checked.** The output of `git ls-files | xargs grep -l '[가-힣]'` has to be **explainable**, and the legitimate list is **exactly ten files** (verified 2026-08-31).
  `crates/baton-term/tests/{fixtures/hangul-2set.jsonl,fixtures/hangul-2set-before-winit-fix.jsonl,ime.rs,vt_conformance.rs}` is test data;
  `crates/winit/{src/keyboard.rs,src/platform_impl/macos/view.rs,UPSTREAM.diff,NOTICE.md}` is upstream text and IME evidence;
  and `.claude/skills/baton-gate/SKILL.md` plus **this file** are **the case where the check contains the characters it searches for** -- the class `[가-힣]` is itself the data, and so are the examples above it.
  **An eleventh file is a rejection, and if the list shrinks, shrink the list.** An over-permissive allowlist makes the check meaningless.

## 6. Performance floors -- these are completion criteria

| | |
|---|---|
| keystroke to screen | local p99 **under 16 ms** |
| idle (12 panes, no output) | CPU **under 2 %**, **no** GPU present |
| first screen on reattach | local **under 300 ms** / remote at 30 ms RTT **under 2 s** |
| warm connect (`ControlMaster` reuse) | **under 100 ms** |
| heavy output (`yes`) | coalesce without dropping a frame, and hold the scrollback cap |
| palette open to first result | **under 50 ms** (at 500 actions) |

## 7. Headless test rules

**The rules for verifying terminal render are in [`crates/baton-term/CLAUDE.md`](./crates/baton-term/CLAUDE.md).** What follows applies to every test that drives `iced_test`'s `Simulator`.

- **Keep `ICED_TEST_BACKEND=tiny-skia`** (`.cargo/config.toml`). The reason is not reproducibility but **that `Simulator` runs on a CI runner with no GPU**. The default, wgpu, demands one.
- **`Simulator` does capture a canvas** (measured). Pixel goldens are not skipped because they are *impossible* but because they are **worthless**.
- **Give the cursor with `point_at`.** Inject only a `CursorMoved` event and the cursor the widget receives is `Unavailable`.
- **Finish a drag inside one `Simulator`.** `pane_grid`'s drag state lives in the widget tree's state and disappears when `UserInterface` is rebuilt.
- **In a `RedrawRequested` handler, emit a message only when state changed.** iced repeats `interface.update` up to three times until no new message comes out, rebuilding the UI in between.
- **Goldens or not, do not put a time or a uuid on screen.** That is what the `Clock` and `Ids` ports are for. A view that depends on the clock makes every form of snapshot flicker.

## 8. Documentation rules

- **Do not create a new document file** when a new question comes up. Edit the existing section in that milestone.
- A reversed decision is **deleted**, not struck through. Leave one line in `DECISIONS.md`.
- Respect the `<!-- budget: N lines -->` at the top of a file.
- **Do not put field definitions, configuration values or traits in prose written for people.** That material comes here.
- **Do not commit a script that runs once.** Left behind, it diverges from the state of the repository immediately. Commit only what is used every time (the issue and pull request templates); **the specification is held by [`docs/WORKFLOW.md`](./docs/WORKFLOW.md).**
- **Do not leave a decision in an issue comment.** A GitHub issue is a unit of work; the canonical home of a decision is `DECISIONS.md`. The operating rules are in [`docs/WORKFLOW.md`](./docs/WORKFLOW.md).
- **`evidence/` is the basis as it stood at the time of the investigation, not the current decision.** On a conflict, **`DECISIONS.md` wins.** An evidence document whose premise has been reversed gets (1) a table at the top saying what died and (2) **its operational instructions removed from the body** -- a spike's scope, what to do if it failed, the schedule. Measurements stay. The basis remains valid; **the instructions do not.**
- **There are two kinds of budget.** Prose documents (`README`, `PLAN`, `sys/*`, `ux/*`) hold a fixed budget and are compressed when they exceed it. **Cumulative documents (`DECISIONS.md` and `evidence/*`) have no budget.** Their size is managed by the "reversed means deleted" rule.
- **Anything investigated is written to that milestone's `evidence/` as markdown.** Not left in a chat log. Investigating the same thing twice is the most expensive waste available in this project.
- **Add a line to `docs/primer/glossary.md` before using a new term.** Do not make a person ask what a word means twice.
- **`docs/primer/` is background knowledge only and holds no decisions.** A decision leaking into a primer means two canonical sources. A new decision goes to `DECISIONS.md` or to the relevant `sys/` document.
- **The canonical UI is `docs/milestones/*/design/`.** One HTML file per area, with `index.html` as the map. **A screen that is not there is a screen that does not exist yet.** Sketches are generated from `design/_gen/` (`dl.py` for shapes, `sk.py` for sketches, `pg.py` for the page shell, `build.py` for the body and assembly). **Do not hand-edit the HTML.**
- Do not read `docs/archive/`. It is the document set from before 2026-08-19.
