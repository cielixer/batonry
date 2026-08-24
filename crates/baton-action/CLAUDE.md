# baton-action -- the implementation contract

**This file supplements the repository's [`CLAUDE.md`](../../CLAUDE.md).** The root contract governs the whole project; this one holds rules that apply to this crate alone. On a conflict, the root wins. It is what the root's **A10** (every behaviour goes through the action registry) and **A11** (input goes through the router) expand into.

**Paths are written relative to the repository root.** `DECISIONS.md` and `evidence/*` are under `docs/milestones/01-ssh-client/`.

---

## 1. The action registry -- every behaviour is an action

**A UI element publishes an action rather than calling a function directly.** iced's `Message` plays that role, so the framework enforces it.

**There are two tables, because what an action *is* and how it is *reached* are different things** (#63).
**Both live inside this crate and both are compile-time constants** (#77).

```rust
// baton-action/src/action.rs -- what it is. Permanent, and independent of any keymap
pub struct Action {
    pub id: Cow<'static, str>,       // "host.connect", "pane.split.vertical"
    pub label: Cow<'static, str>,    // the name shown in the palette. Canonical, not duplicated
    pub channels: Channels,          // PALETTE|CLICK|MENU|DRAG. ux §4's "publish" column, minus K
    pub arg: ArgShape,               // None|Host|Pane|TabIndex|Snippet|HostTab{..}|...
}
pub const KEY_ONLY: Channels;        // the empty set. Build a row with it; never query with it
pub const fn reachable_from(set: Channels, surface: Channels) -> bool;
pub const fn union(set: Channels, extra: Channels) -> Channels;   // a fn, not `|`, because const

pub struct Binding {                          // how it is reached. Replaceable, and there may be several
    pub action: Cow<'static, str>,            // joins on Action::id
    pub key: Cow<'static, str>,               // "meta+shift+KeyD"
    pub when: Option<Cow<'static, str>>,      // #11 parses this
}

// baton-action/src/catalog.rs -- the compile-time tables
pub const ACTIONS: &[Action];
pub const DEFAULT_KEYMAP: &[Binding];
pub const BUILT_IN: Source;

// baton-action/src/registry.rs
pub struct Source { pub name: Cow<'static, str>, pub rows: Cow<'static, [Action]> }
pub fn try_merge(sources: &[Source]) -> Result<Registry, MergeError>;
pub struct ActionId(u16);          // issued by the registry. An index, so lookup is O(1)
impl Registry {
    pub fn resolve(&self, name: &str) -> Option<ActionId>;   // permanent name -> issued index
    pub fn get(&self, id: ActionId) -> Option<&Action>;
    pub fn rows(&self) -> &[Action];
}
// The accessor that extracts the index is a free function, and **private** (#79). A caller
// that can compute the integer can build an index the registry never issued.
```

**`KEY_ONLY` is the empty set, so it cannot be used as a query.** `reachable_from(anything, KEY_ONLY)` is always `true`. "Does this reach no surface at all?" is asked as `channels == KEY_ONLY`.

**An `ActionId` means something only inside the registry that issued it.** The scope is the registry, not the run: with two registries the indices overlap. If a reason to build a second one ever appears, that is the point to design against.

**There is no `KEY` bit in the channel set.** An `Action` is permanent, but **whether a key reaches it changes the moment someone rebinds.** That information is derived from the binding table (a binding naming the action means a key reaches it), so a bit would duplicate it -- and would leave it ambiguous whether the bit means default reachability, effective reachability, or permission to bind at all.

**Use `Cow<'static, str>`, not `&'static str`.** The strings in a user keymap (TOML) are owned at runtime, so `&'static` would force a leak or interning. `Cow::Borrowed` is const-constructible, so the built-in keymap stays a constant.

**M1 includes user keymaps** (`ux/interactions.md` §8-4). **Anything outside the compile-time tables arrives as TOML, and merging only adds: it never redefines** (#78). A rebind is one more `Binding` row and a new action is one more `Action` row. **A duplicate id is an error, not last-one-wins.** If a file could quietly change what a built-in action does, the palette would still be showing the built-in label.

**The merge returns a `Result`. This is not a test condition.** Taking `Source { name, rows }` is what lets a duplicate-id error **name both sources and both positions.** An anonymous slice cannot say that.

**`when` guards the binding, not the action.** The order of judgement is *"if it is in the keymap and `when` passes, publish the action; everything else goes through the input router to the PTY"*, so `when` decides **whether the keystroke is ours in the first place.** It is conditional interception, not a disabled action. `term.copy` is the example: with a selection it copies, and without one `⌘C` **goes to the PTY.** One action may carry several bindings, or none.

- **Every action is registered.** The palette is a UI that searches this registry for rows whose `channels` include `PALETTE`. **If the mouse can do it and the palette cannot, that is a bug.**
  **Being in the registry and being in the palette are still different things.** A host-key modal's confirm and cancel go through the registry (A10: a click does not call a function directly) but do not appear in the palette. They are not the kind of thing a person hunts for, and greying them out fills the list with noise. The dividing line is **"should a user be able to invoke this from anywhere?"**
- **Keep `when` clauses minimal.** Context identifiers and **`!`, `&&`, `||`, `==`** only. Warp does this in 124 lines and VSCode in 2,183; **we are on Warp's side.** The target is under 200 lines.
  **The operators are symbols, not words (`and`, `or`, `not`).** This is a file a user writes by hand, and the keymap syntax people already know (VSCode and Zed) uses `!` and `&&`.
- Sources worth reading: Zed's `crates/gpui/src/keymap/context.rs` (891 lines, Apache-2.0) and Warp's `crates/warpui_core/src/keymap/context.rs` (124 lines, **MIT**).
- Use `nucleo` for fuzzy search. Do not write one.

**A key conflict is compile-error grade.** If two actions share a default key and their `when` clauses **can be true at the same time, CI fails.** Do not resolve it with precedence. `⌘D` had been assigned to both favourite and split, and `host_selected` and `pane_focused` really are true together. The check is in the pull request gate, `TESTPLAN.md` §8.

## 2. The input router -- a pane does not own input

```rust
pub enum TargetSet { Focused, Set(Vec<PaneId>) }
pub struct InputRouter { targets: TargetSet }
// dispatch(bytes) -> send to each sink in the target set
```

- **Keystrokes, pastes, snippet execution, and anything the palette publishes as a send all go through this router.**
- Do not build `pane.on_key() -> pty.write()`. **Build it once that way and broadcast can never be inserted afterwards.**
- **M1 has no broadcast feature.** `TargetSet::Set` is defined and no UI is built for it.
- **Do not break the ability to fill a pane with bytes without running them.** M1 has no UI for it, but the router has to be able to send bytes without a newline: M2 uses it to stage the `tmux` install command and M3 the `baton-agent` one, **leaving the user to press `⏎`.** Do not put code on the send path that appends a newline unconditionally.
