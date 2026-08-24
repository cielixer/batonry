//! The actions this application has, and the keys that reach them.
//!
//! Ten of each, and deliberately not the whole ~82-row specification. The
//! contract runs one way: **every implemented behaviour has a row here**, not
//! every row has an implementation. Registering an action nothing executes puts
//! an entry in the palette that does nothing, which is worse than transcribing
//! late.
//!
//! Both tables are `const`. Anything that has to change without a rebuild --
//! a rebound key, a row from something loaded -- arrives as a second source and
//! is merged on top; see [`crate::try_merge`].

use std::borrow::Cow;

use crate::action::{
    Action, ArgKind, Binding, Channels, KEY_ONLY, MENU, PALETTE, union,
};

use crate::registry::Source;

/// Actions stage 1 can actually perform.
pub const ACTIONS: &[Action] = &[
    act("app.quit", "Quit", PALETTE),
    act("palette.open", "Command Palette", KEY_ONLY),
    act("palette.close", "Close Palette", KEY_ONLY),
    act("palette.confirm", "Run", KEY_ONLY),
    act("palette.next", "Next Result", KEY_ONLY),
    act("palette.prev", "Previous Result", KEY_ONLY),
    act("term.copy", "Copy", union(PALETTE, MENU)),
    act("term.paste", "Paste", union(PALETTE, MENU)),
    act("term.select_all", "Select All", KEY_ONLY),
    act("term.clear", "Clear Screen", PALETTE),
];

/// Every stage-1 action takes no argument, so the shape is not repeated ten
/// times. An action that needs one spells it out instead of using this.
const fn act(
    id: &'static str,
    label: &'static str,
    channels: Channels,
) -> Action {
    Action {
        id: Cow::Borrowed(id),
        label: Cow::Borrowed(label),
        channels,
        arg: ArgKind::None,
    }
}

/// The built-in keymap.
///
/// **Every unmodified key here is guarded.** A key not in this table reaches the
/// terminal, so this table *is* the interception set -- an unguarded bare key
/// silently stops reaching the shell, which is the first thing a terminal user
/// notices.
pub const DEFAULT_KEYMAP: &[Binding] = &[
    bind("palette.open", "meta+KeyK", Some("!palette_open")),
    bind("palette.close", "Escape", Some("palette_open")),
    bind("palette.confirm", "Enter", Some("palette_open")),
    bind("palette.next", "ArrowDown", Some("palette_open")),
    bind("palette.prev", "ArrowUp", Some("palette_open")),
    bind("app.quit", "meta+KeyQ", None),
    bind("term.copy", "meta+KeyC", Some("has_selection")),
    bind("term.paste", "meta+KeyV", Some("pane_live")),
    bind("term.select_all", "meta+KeyA", Some("pane_focused")),
    bind("term.clear", "meta+shift+KeyK", Some("pane_live")),
];

const fn bind(
    action: &'static str,
    key: &'static str,
    when: Option<&'static str>,
) -> Binding {
    Binding {
        action: Cow::Borrowed(action),
        key: Cow::Borrowed(key),
        when: match when {
            Some(w) => Some(Cow::Borrowed(w)),
            None => None,
        },
    }
}

/// This crate's contribution to the boot-time merge.
pub const BUILT_IN: Source = Source {
    name: Cow::Borrowed("baton-action"),
    rows: Cow::Borrowed(ACTIONS),
};
