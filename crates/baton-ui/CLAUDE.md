# baton-ui -- the implementation contract

**This file supplements the repository's [`CLAUDE.md`](../../CLAUDE.md).** The root contract governs the whole project; this one holds rules that apply to this crate alone. On a conflict, the root wins.

**This crate has no `main`** (root §2). With one it cannot be driven by `iced_test`'s headless `Simulator`, and the headless test rules in the root's §7 apply to everything written here.

**Paths are written relative to the repository root.** `DECISIONS.md`, `evidence/*`, `ux/*` and `design/*` are under `docs/milestones/01-ssh-client/`.

---

## 1. UI rules

- **Every UI string is English.** Buttons, labels, statuses, errors, menus, empty-state copy, and every character inside a design sketch. Prose written for people (`docs/**/*.md` and the body of `design/*.html`) is Korean. The dividing line is **"does it appear on screen?"**
- **Do not scatter strings through the code.** Every user-visible string is a constant in `baton/src/main.rs`, passed into `App::new` (the owner's call on #14): editing copy is an edit there, and i18n later swaps what `main` passes. This crate renders what it is handed and writes no user-visible literal of its own. M1 has no locale switching.
- **Dark is the default theme.** Do not hardcode a colour; take all of them from one `Theme` struct, so that light and custom themes can be added later.
- **Restore view state after a restart.** What is restored is the sidebar mode, **whether a group is collapsed**, the sidebar width, the last workspace, and the palette's recent items, all in `app_pref`.
  **What is not restored** is a pane's scroll position, text mid-composition, and a search query. Reviving the scroll position of a session that no longer exists is worse than not restoring it.
  **View state does not go into an export.** It is not inventory to share.
- **The minimum pane size is `20 cols × 5 rows` and nothing shrinks below it.** A drag or a split that would is refused. **Do not create a "this pane is too small" state**: rather than showing a broken screen and apologising, make it unreachable.
  The window's minimum size derives from this: `min_w = 20*cell_w + 244 + 30 + 26` and `min_h = 5*cell_h + 34 + 32 + 18` (the chrome constants below).
  If a restored layout does not fit the window, **undo the deepest split first** and say so in the restore banner.
- **A selection always copies.** This is not a setting. macOS has no primary selection so this overwrites the system clipboard; if someone complains, separate it then.
- **The reason an action is disabled is visible on hover or keyboard focus only.** Shown always, the list fills with sentences. **It has to be visible on focus too**, or it contradicts the ban on hover-only affordances.
- **The sidebar filter (`⌘⇧F`) and the command palette (`⌘K`) are different widgets.** Do not merge them. The filter narrows a list in place; the palette finds actions, workspaces and snippets.
- **A snippet runs immediately, with no confirmation.** The palette row already says which pane it targets.
- **A banner does not carry state.** Do not put something the user needs to know into an indicator that disappears when dismissed. Approaching the session limit is signalled by **the session badge reading `9/10`**, not by a banner: no new UI element, just more meaning on a number that is already there.
- **There is no update UI in M1.**
- **Do not cover a disconnected pane.** The reason, the attempt count and the next action attach as **a bar along the bottom of the pane.** Dimming the whole thing hides the last output and blocks scrolling and copying, which makes the pane genuinely a dead end.
  **The only centred case is a pane with no content yet.** The test is not the reason for the failure but **whether there is content**.
- **`local`, the builtin local host, is always the top row of the sidebar.** It belongs to no section and is the first line regardless of how many hosts exist. **Editing, deleting, moving between groups and favouriting are all impossible.** It is never filtered out by search. It has a scratch, and it can be a pane in a workspace. **There is no "zero hosts" state.**
- **Do not hide a blocked menu item.** Removing it makes the menu a different shape for different hosts, so its positions cannot be memorised. Leave it dimmed and label it `locked`.
- **The default start mode is workspace mode.** But **with zero `workspace` rows, start in host mode.** Do not build a "you have no workspaces" empty screen.
- **A sidebar item is a card, not a tree node.** One host is one `rounded-lg` card. **The border is transparent at rest and turns on with the background only when there is a status.** Section headers use `rounded-md` to mark the hierarchy (this is Orca's sidebar rule).
- **A healthy state gets no icon.** Connected is a single dot and healthy is unmarked. **Fix the container size of the status indicator** so a change of state does not shift the layout.
- **Group depth is two at most.** Render it in the UI as **one path string**, like `medipixel / gpu`. Do not draw a nested tree.
- **This is how inheritance is shown.** An inherited value is dimmed, the placeholder carries the inherited value, and the label reads `— inherited from <group>`. The `↺` reset icon appears **only on a field set directly.** Do not use an affordance that appears only on hover: the keyboard cannot reach it.
- **Build the host edit form as a declarative field table.** A field holds no value, only `pick` and `write` functions, which is what makes "which scope did this value come from?" computable at render time. The reasoning and the comparison against the alternatives are in `evidence/host-ui.md`.

## 2. Layout regressions -- the completion criteria for layout work

These all happened in Orca. **Write the reproduction as code.**

1. A pane created by a split **does not stay blank.**
2. The grid does not tear during a drag-resize, and the final size is exact on release.
3. Closing a pane redraws its sibling at the correct size immediately.
4. A disconnected pane **is not a dead end.** The reason appears and there is a reconnect button.
5. After reconnecting, the screen repaints from the retained buffer. **An escape sequence is never replayed half way.**
6. Splitting during heavy output leaves both panes correct.
7. Resizing at the top of the scrollback keeps the position.
8. Twelve panes left open for thirty minutes still use under 2 % CPU.
