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

// baton-action/src/context.rs -- what a `when` clause can name. Same shape as Channels (#87)
pub struct Flags(u32);               // which conditions hold, and equally one condition
pub const PANE_FOCUSED: Flags;       // seventeen of these
// Both bitsets declare their constants **and their two operations** with `bitset!`
// (#88, #90). No shift and no bit arithmetic is written by hand outside that macro,
// and a test per type pins both. The operations are named by the caller because the
// question differs -- `reachable_from` asks about a surface, `holds` about a condition.
// **`bitflags` is not used** (#90): its operations are not `const fn` and the tables
// here are `const`, and `from_name` matches the constant identifier rather than the
// spelling the specification fixes.
pub const NONE: Flags;               // the empty set. Same hazard as KEY_ONLY
pub const fn holds(set: Flags, wanted: Flags) -> bool;
pub const fn combine(set: Flags, extra: Flags) -> Flags;   // `union` is taken at the crate root
pub struct Flag(Flags);              // exactly one condition: a clause's leaf
impl FromStr for Flag;               // the only way to build one. The field is private
impl From<Flag> for Flags;           // widen to a set of one, to ask `holds`
// **A leaf holds one condition, by construction** (#89, #92). A set holds none or several,
// and a leaf holding either prints something that does not parse back to it. There is no
// checked constructor because there is nothing to check: a name yields one bit. Only
// `Flag` converts to and from a name; `Flags` has no `Display` and no `FromStr`.
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
pub struct Source { pub name: Cow<'static, str>, pub actions: Cow<'static, [Action]> }
pub fn merge(sources: &[Source]) -> Registry;   // panics on a duplicate id (#82)
pub struct ActionId(u16);          // issued by the registry. An index, so lookup is O(1)
impl Registry {
    pub fn resolve(&self, name: &str) -> Option<ActionId>;   // permanent name -> issued index
    pub fn get(&self, id: ActionId) -> Option<&Action>;
    pub fn iter(&self) -> impl Iterator<Item = (ActionId, &Action)>;
    pub fn reachable(&self, surface: Channels) -> impl Iterator<Item = (ActionId, &Action)>;
    pub fn count(&self) -> usize;
}
// The accessor that extracts the index is a free function, and **private** (#79). A caller
// that can compute the integer can build an index the registry never issued.
```

**`KEY_ONLY` is the empty set, so it cannot be used as a query.** `reachable_from(anything, KEY_ONLY)` is always `true`. "Does this reach no surface at all?" is asked as `channels == KEY_ONLY`.

**An `ActionId` means something only inside the registry that issued it.** The scope is the registry, not the run: with two registries the indices overlap. If a reason to build a second one ever appears, that is the point to design against.

**There is no `KEY` bit in the channel set.** An `Action` is permanent, but **whether a key reaches it changes the moment someone rebinds.** That information is derived from the binding table (a binding naming the action means a key reaches it), so a bit would duplicate it -- and would leave it ambiguous whether the bit means default reachability, effective reachability, or permission to bind at all.

**Use `Cow<'static, str>`, not `&'static str`.** The strings in a user keymap (TOML) are owned at runtime, so `&'static` would force a leak or interning. `Cow::Borrowed` is const-constructible, so the built-in keymap stays a constant.

**M1 includes user keymaps** (`ux/interactions.md` §8-4). **Anything outside the compile-time tables arrives as TOML, and merging only adds: it never redefines** (#78). A rebind is one more `Binding` row and a new action is one more `Action` row.

**There is no last-one-wins, and the reason is that an `Action` row is a description rather than behaviour** (#81). The behaviour is whatever matches on the resolved `ActionId`, so overwriting a row could not change what an action does -- only what it claims to do. A palette entry that copies while calling itself something else is worse than a refusal. What a user actually wants to override is a **binding**, and that is already a pure add.

**An id is global and flat; `Source::name` is not a namespace.** The merge keys on the id exactly as written, so `term.copy` from a loaded file collides with the built-in one. That is deliberate: the id is what a keymap file and a saved setting persist against, so prefixing it with whichever source supplied it would break all of them as soon as an action moved between crates. Something that wants its own space spells it into the id, as `myplugin.git.commit`.

**A duplicate id panics; it is not a `Result`** (#82). It is caught wherever it appears, across two sources or inside one, and the message names **both claimants by source position and name**, because a name is a label and two sources may share one.

**`merge` does not validate a row's content, and #25 must not fix that by growing it.** An empty id, an empty label, an id that breaks the naming convention: all merge cleanly today, and nothing unchecked can reach it because every source is a `const` the tests read directly. The keymap loader is where a loaded row gets checked, and it is the better place: it can name a line. An id is unique, and every source is a compile-time constant, so two rows claiming one name is a wrong table rather than a condition a caller could act on. Taking `Source { name, actions }` is what lets the message **name both sources and both positions**, which an anonymous slice could not. When a keymap file starts contributing rows, **its loader validates them** -- that is the better place, because it can name a line.

**A keypress is a hash, not a scan.** `Keymap` is keyed by chord and `lookup`
takes the guard from there; `Registry::resolve` is keyed by name for the same
reason (#80). The rows are not public -- `entries()` was removed once nothing
but a test used it, and #12 will decide the shape it actually needs.

**`when` guards the binding, not the action.** The order of judgement is *"if it is in the keymap and `when` passes, publish the action; everything else goes through the input router to the PTY"*, so `when` decides **whether the keystroke is ours in the first place.** It is conditional interception, not a disabled action. `term.copy` is the example: with a selection it copies, and without one `⌘C` **goes to the PTY.** One action may carry several bindings, or none.

- **The registry hands out `(ActionId, &Action)` and never its slice** (#80). Every list a surface draws becomes a message, so an accessor that returns `&Action` alone forces the caller to `resolve` an id string it just read -- the index accessor being private is what makes that unavoidable. `iter` is the whole table and `reachable(surface)` is the filter a palette *is*; there is no `rows()`, `len()` or `is_empty()`.
- **Every action is registered.** The palette is `reachable(PALETTE)` and nothing else. **If the mouse can do it and the palette cannot, that is a bug.**
  **Being in the registry and being in the palette are still different things.** A host-key modal's confirm and cancel go through the registry (A10: a click does not call a function directly) but do not appear in the palette. They are not the kind of thing a person hunts for, and greying them out fills the list with noise. The dividing line is **"should a user be able to invoke this from anywhere?"**
- **Keep `when` clauses minimal.** Context identifiers and **`!`, `&&`, `||`, `==`** only. Warp does this in 124 lines and VSCode in 2,183; **we are on Warp's side.**
  **The constraint is the grammar, not a line count** (#83). Five node kinds and four operators, and adding one is the thing to argue about. A line budget measures the wrong thing: it counts doc comments and error messages, which should grow, and it says nothing about a single operator that doubles what a clause can express.
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
