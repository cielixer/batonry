//! The two tables: what an action is, and how one is reached.
//!
//! They live together because the split between them *is* the design, and
//! putting them in separate files hides the one thing worth noticing about it.
//! [`Action`] is permanent and identical under every keymap; [`Binding`] is
//! replaceable and there may be several or none. The condition belongs to the
//! binding, not the action, because what it decides is whether a keystroke
//! becomes an action at all.
//!
//! Both tables live in this crate rather than in whatever drives it. That is a
//! deliberate reversal: an earlier shape pushed the rows out on the theory that
//! this crate would be extracted and published, and it will not be. Keeping them
//! here means one place to look, a compile-time constant instead of a wiring
//! step, and no vocabulary split across a boundary nobody was going to cross.

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
/// Reachable only by key, or not yet reachable at all.
pub const NO_CHANNEL: Channels = Channels(0);

/// Whether every bit in `wanted` is present.
pub const fn reaches(set: Channels, wanted: Channels) -> bool {
    set.0 & wanted.0 == wanted.0
}

/// Combines channels. `const`, because the table is a constant.
pub const fn also(set: Channels, extra: Channels) -> Channels {
    Channels(set.0 | extra.0)
}

/// The shape of the argument an action expects.
///
/// The *kind*, not the type: the value travels beside the id when the action is
/// issued, so the table only has to say what shape to expect.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ArgKind {
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
    /// taking a host alone would be [`ArgKind::Host`].
    HostTab,
}

/// One thing the app can do.
///
/// Strings are `Cow` for the same reason a binding's are: the built-in table
/// borrows literals and stays a constant, while a row that arrives at runtime --
/// from a configuration file, or one day from something loaded -- owns its
/// strings and is the same type. One table, two origins.
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
    pub arg: ArgKind,
}

/// One way to reach one action.
///
/// `Cow` rather than `&'static str` is the whole reason a binding is separate
/// from whatever describes an action: a keymap loaded from a configuration file
/// owns its strings at runtime, and if this were `&'static` the loader would
/// have to leak or intern them. Built-in bindings borrow literals and are
/// therefore still `const`-constructible; loaded ones own theirs. Both are the
/// same type, so a user's file really is more rows in the same table.
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
