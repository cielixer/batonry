//! What an action *is*: metadata that no keymap can change.

use std::ops::BitOr;

/// The surfaces from which an action can be issued, apart from a key binding.
///
/// **Key reachability is deliberately absent.** An action is reachable by key
/// exactly when some binding names it, so a `KEY` bit here would duplicate the
/// binding table -- and worse, it would be ambiguous the moment a user rebinds
/// something: default reachability, effective reachability, or permission to
/// bind at all?
///
/// The bits that remain are what makes "in the registry" and "in the palette"
/// different things. A modal's confirm button goes through the registry, so a
/// click never calls a function directly, but it is not something a person
/// hunts for in a palette -- so it carries `CLICK` without `PALETTE`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct IssueSites(u8);

impl IssueSites {
    /// The command palette can issue the action.
    pub const PALETTE: Self = Self(1 << 0);
    /// A direct UI click can issue the action.
    pub const CLICK: Self = Self(1 << 1);
    /// A context or overflow menu can issue the action.
    pub const MENU: Self = Self(1 << 2);
    /// A drag gesture can issue the action.
    pub const DRAG: Self = Self(1 << 3);

    /// No issue surface other than a key binding.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Whether every bit in `other` is present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Combines sites in a `const` context, which `BitOr` cannot do. The
    /// action table is a constant, so this is the form it uses.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOr for IssueSites {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// The shape of the argument an action expects.
///
/// These name the *kind* of argument, not the domain type. Referring to real
/// id types would mean inventing them in `baton-core` from here, which is the
/// wrong place; and the registry only has to describe the shape, because the
/// value travels beside the id at dispatch time rather than living in this
/// table.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ArgKind {
    /// Takes no argument. Every stage-1 action is this.
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
    /// This variant exists because a flat enum of single ids cannot express
    /// it, and an action that took a host alone would be [`ArgKind::Host`].
    /// Nothing in stage 1 uses it; it is here so the shape is settled before
    /// there are consumers to migrate.
    HostTab,
}

/// A registry-issued action identifier.
///
/// The value is the action's position in the registry's contiguous slice, so
/// resolving one is a bounds check rather than a second lookup. There is no
/// public constructor: an id that the registry did not issue could not be
/// resolved anyway.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ActionId(u16);

impl ActionId {
    /// Only the registry issues ids, and only from a bounds-checked index.
    pub(crate) const fn from_index(index: u16) -> Self {
        Self(index)
    }

    /// The contiguous registry position this id stands for.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// What an action is: permanent metadata, identical under every keymap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionMeta {
    /// The stable id. Bindings and saved settings join on this, so **it never
    /// changes**; labels are free to.
    pub id: &'static str,
    /// The canonical user-visible text. Not duplicated anywhere else.
    pub label: &'static str,
    /// Which non-key surfaces can issue this.
    pub issue: IssueSites,
    /// The argument shape this expects.
    pub arg: ArgKind,
}
