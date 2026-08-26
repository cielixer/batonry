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
//! is merged on top; see [`crate::merge`].
//!
//! **The two tables stay two, and the id they join on is written once.** An
//! action may carry no binding at all (roughly twenty of the specification's
//! rows are palette- and menu-only) or several (`pane.resize.*` has a key and a
//! drag), and a user's keymap adds `Binding` rows without adding `Action` rows,
//! so a binding cannot be a field of an action. What it can stop being is a
//! second copy of the same string: the [`id`] constants below are the join key,
//! and a typo is a build failure rather than a binding that resolves to nothing.

use std::borrow::Cow;

use crate::action::{
    Action, ArgShape, Binding, Channels, KEY_ONLY, MENU, PALETTE, union,
};

use crate::registry::Source;

/// The permanent names, written once because both tables join on them.
///
/// Not public: outside this crate an action is reached through
/// [`Registry::resolve`](crate::Registry::resolve), which is what a keymap file
/// and a palette query both go through.
mod id {
    pub const APP_QUIT: &str = "app.quit";
    pub const PALETTE_OPEN: &str = "palette.open";
    pub const PALETTE_CLOSE: &str = "palette.close";
    pub const PALETTE_CONFIRM: &str = "palette.confirm";
    pub const PALETTE_NEXT: &str = "palette.next";
    pub const PALETTE_PREV: &str = "palette.prev";
    pub const TERM_COPY: &str = "term.copy";
    pub const TERM_PASTE: &str = "term.paste";
    pub const TERM_SELECT_ALL: &str = "term.select_all";
    pub const TERM_CLEAR: &str = "term.clear";
}

/// Actions stage 1 can actually perform.
pub const ACTIONS: &[Action] = &[
    act(id::APP_QUIT, "Quit", PALETTE),
    act(id::PALETTE_OPEN, "Command Palette", KEY_ONLY),
    act(id::PALETTE_CLOSE, "Close Palette", KEY_ONLY),
    act(id::PALETTE_CONFIRM, "Run", KEY_ONLY),
    act(id::PALETTE_NEXT, "Next Result", KEY_ONLY),
    act(id::PALETTE_PREV, "Previous Result", KEY_ONLY),
    act(id::TERM_COPY, "Copy", union(PALETTE, MENU)),
    act(id::TERM_PASTE, "Paste", union(PALETTE, MENU)),
    act(id::TERM_SELECT_ALL, "Select All", KEY_ONLY),
    act(id::TERM_CLEAR, "Clear Screen", PALETTE),
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
        arg: ArgShape::None,
    }
}

/// The built-in keymap.
///
/// **Every unmodified key here is guarded.** A key not in this table reaches the
/// terminal, so this table *is* the interception set -- an unguarded bare key
/// silently stops reaching the shell, which is the first thing a terminal user
/// notices.
pub const DEFAULT_KEYMAP: &[Binding] = &[
    bind(id::PALETTE_OPEN, "meta+KeyK", Some("!palette_open")),
    bind(id::PALETTE_CLOSE, "Escape", Some("palette_open")),
    bind(id::PALETTE_CONFIRM, "Enter", Some("palette_open")),
    bind(id::PALETTE_NEXT, "ArrowDown", Some("palette_open")),
    bind(id::PALETTE_PREV, "ArrowUp", Some("palette_open")),
    bind(id::APP_QUIT, "meta+KeyQ", None),
    bind(id::TERM_COPY, "meta+KeyC", Some("has_selection")),
    bind(id::TERM_PASTE, "meta+KeyV", Some("pane_live")),
    bind(id::TERM_SELECT_ALL, "meta+KeyA", Some("pane_focused")),
    bind(id::TERM_CLEAR, "meta+shift+KeyK", Some("pane_live")),
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
    actions: Cow::Borrowed(ACTIONS),
};
