//! The two tables: what an action is, and how one is reached.
//!
//! They live together because the split between them *is* the design, and
//! putting them in separate files hides the one thing worth noticing about it.
//! [`Action`] is permanent and identical under every keymap; [`Binding`] is
//! replaceable and there may be several or none. The condition belongs to the
//! binding, not the action, because what it decides is whether a keystroke
//! becomes an action at all.

use std::borrow::Cow;

/// Where an action can be invoked from, other than by a key.
///
/// **Key reachability is deliberately absent.** An action is reachable by key
/// exactly when some binding names it, so a `KEY` bit would duplicate the keymap
/// -- and be ambiguous the moment someone rebinds: default reachability,
/// effective reachability, or permission to bind at all?
///
/// The bits that remain are what makes "in the registry" and "in the palette"
/// different things. A modal's confirm button goes through the registry, so a
/// click never calls a function directly, but nobody hunts for it in a palette
/// -- it carries `CLICK` without `PALETTE`.
///
/// The empty set is [`KEY_ONLY`]; read its note before using it in a comparison.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Channels(u8);

/// The command palette.
pub const PALETTE: Channels = Channels(1 << 0);
/// A direct click on something.
pub const CLICK: Channels = Channels(1 << 1);
/// A context or overflow menu.
pub const MENU: Channels = Channels(1 << 2);
/// A drag gesture.
pub const DRAG: Channels = Channels(1 << 3);
/// No non-key surface. Every action carrying this is reached by a key instead,
/// which a test in the keymap suite asserts row by row.
///
/// **A value to build a row with, never one to ask about.** It is the empty set,
/// so [`reachable_from`]`(anything, KEY_ONLY)` is `true`, `PALETTE` included.
/// "Does this reach nothing?" is `set == KEY_ONLY`.
pub const KEY_ONLY: Channels = Channels(0);

/// Whether `set` includes every bit of `surface`.
pub const fn reachable_from(set: Channels, surface: Channels) -> bool {
    set.0 & surface.0 == surface.0
}

/// Set union. A `const fn` and not `BitOr`, because operator traits are not
/// callable in the `const` context the table is built in.
pub const fn union(set: Channels, extra: Channels) -> Channels {
    Channels(set.0 | extra.0)
}

/// The shape of the argument an action expects.
///
/// A shape and not a type, because this crate does not depend on `baton-core`
/// and so cannot name what a host or a pane actually is. The value travels
/// beside the id when the action is issued; the table only says what to expect.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ArgShape {
    /// Takes no argument. Everything stage 1 implements is this.
    #[default]
    None,
    /// A host.
    Host,
    /// A pane.
    Pane,
    /// A tab position.
    TabIndex,
    /// A snippet.
    Snippet,
    /// A group.
    Group,
    /// A workspace.
    Workspace,
    /// A help topic.
    Topic,
    /// A host **and an optional tab**, which is `host.edit{id,tab?}`.
    ///
    /// Here because a flat enum of single ids cannot express it, and an action
    /// taking a host alone would be [`ArgShape::Host`].
    HostTab,
}

/// One thing the app can do.
///
/// Strings are `Cow` so one table has two origins: the built-in rows borrow
/// literals and stay a constant, a row loaded at runtime owns its strings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Action {
    /// The permanent name, `<domain>.<verb>[.<variant>]` as in `host.connect`.
    /// Keymaps and saved settings join on it, so **it never changes**; labels
    /// are free to.
    pub id: Cow<'static, str>,
    /// The canonical user-visible text. Not duplicated anywhere else.
    pub label: Cow<'static, str>,
    /// Which non-key surfaces can invoke this.
    pub channels: Channels,
    /// The argument shape this expects.
    pub arg: ArgShape,
}

/// One way to reach one action.
///
/// `when` stays opaque here. It is a guard on the *binding*, so what it decides
/// is whether a keystroke becomes an action at all or falls through to whatever
/// the input router is pointed at -- not whether an action is greyed out.
/// Parsing and evaluating it belongs to whoever owns the context it names.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Binding {
    /// The id of the action this reaches. Joins to a registry.
    pub action: Cow<'static, str>,
    /// The chord's canonical ASCII spelling, parseable by [`crate::Keystroke`].
    pub key: Cow<'static, str>,
    /// An opaque condition. Empty means the binding always applies.
    pub when: Option<Cow<'static, str>>,
}
