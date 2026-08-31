//! The closed vocabulary of conditions a `when` clause can name.

use std::fmt;
use std::str::FromStr;

use crate::bitset::bitset;

/// Which conditions hold, and equally a single condition: a set either way.
///
/// The same shape as [`Channels`](crate::Channels), and for the same reason --
/// naming one of a fixed set of booleans, and asking whether a set contains it,
/// are the same operation. Plain data, so whoever owns the UI state fills it and
/// this crate stays unaware that a UI toolkit exists.
///
/// **A set can hold anything, including what the domain does not have.** Seen
/// once already with `Channels`: [`Flags::NONE`] is contained by every set, so
/// `holds(anything, Flags::NONE)` is `true`, which makes it a value to build a set
/// from and never one to ask about.
///
/// That is also why a set is not what a clause's leaf holds. A leaf names one
/// condition, and [`Flag`] is the checked type that carries it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Flags(u32);

bitset!(Flags, holds, combine, NAMES,
    /// Nothing holds. A value to start a set from, never one to ask about.
    NONE:
    /// A terminal pane has focus.
    PANE_FOCUSED,
    /// That pane is in the `live` state.
    PANE_LIVE,
    /// That pane is in the `failed` or `reconnecting` state.
    PANE_DISCONNECTED,
    /// The terminal has a selection.
    HAS_SELECTION,
    /// The terminal's search bar is open.
    SEARCH_OPEN,
    /// The command palette is open.
    PALETTE_OPEN,
    /// A modal is open, such as host editing or a confirmation.
    DIALOG_OPEN,
    /// A text input has focus: a dialog field, the palette's query, renaming.
    EDITING_TEXT,
    /// The sidebar is in hosts mode.
    SIDEBAR_HOSTS,
    /// The sidebar is in workspaces mode.
    SIDEBAR_WORKSPACES,
    /// A host card is selected in the sidebar.
    HOST_SELECTED,
    /// What is on screen is a scratch.
    SCRATCH_ACTIVE,
    /// What is on screen is a named workspace.
    WORKSPACE_ACTIVE,
    /// That host has a jump path configured.
    HAS_JUMP,
    /// Keys typed while the connection was down are still queued.
    HAS_QUEUED_INPUT,
    /// The modal confirming a host key seen for the first time is open.
    DIALOG_HOSTKEY_NEW,
    /// The modal confirming a **changed** host key is open.
    DIALOG_HOSTKEY_CHANGED,
);

impl Flags {
    /// Groups of conditions of which **at most one** can hold at a time.
    ///
    /// The conflict checker (#12) trusts these blindly: an assignment that
    /// holds two members of one group is skipped as impossible, so a group
    /// belongs here only when the specification actually promises it -- a
    /// wrong entry hides real collisions. "At most one" says nothing about a
    /// whole group being false; that case stays possible on purpose (a
    /// collapsed sidebar, a pane in neither state).
    pub const EXCLUSIVE: &'static [Flags] = &[
        // The sidebar is in one mode.
        combine(Flags::SIDEBAR_HOSTS, Flags::SIDEBAR_WORKSPACES),
        // States of one pane: live, or failed/reconnecting.
        combine(Flags::PANE_LIVE, Flags::PANE_DISCONNECTED),
        // What is on screen is one thing: a scratch or a named workspace.
        combine(Flags::SCRATCH_ACTIVE, Flags::WORKSPACE_ACTIVE),
        // One host-key modal at a time.
        combine(Flags::DIALOG_HOSTKEY_NEW, Flags::DIALOG_HOSTKEY_CHANGED),
    ];
}

/// Whether `ctx` holds two or more members of any of `groups`.
pub(crate) const fn excludes(ctx: Flags, groups: &[Flags]) -> bool {
    let mut i = 0;
    while i < groups.len() {
        if (ctx.0 & groups[i].0).count_ones() >= 2 {
            return true;
        }
        i += 1;
    }
    false
}

/// Every assignment of the conditions at once, for the exhaustive sweep.
/// Crate-private: outside callers build sets from the named constants.
pub(crate) fn assignments() -> impl Iterator<Item = Flags> {
    (0..(1u32 << Flags::NAMES.len())).map(Flags)
}

/// Exactly one condition: what a clause's leaf names.
///
/// [`Flags`] is a set, and a set holds none or several as readily as one. A leaf
/// can hold neither: the empty set is a clause that is always true, several bits
/// is a conjunction, and printing either would not parse back to what it was.
///
/// **[`FromStr`] is the only way to build one**, and it reads a name out of the
/// table, so the value is one condition by construction rather than by a check
/// that could be skipped. The field is private, which is what closes the other
/// route. Only this type converts to and from a name.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Flag(Flags);

impl From<Flag> for Flags {
    /// One condition widened to a set of one, which is what [`holds`] asks with.
    fn from(flag: Flag) -> Flags {
        flag.0
    }
}

/// A name that is not in the closed vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownFlag {
    /// What was written.
    pub name: String,
}

impl fmt::Display for UnknownFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown context flag {:?}", self.name)
    }
}

impl std::error::Error for UnknownFlag {}

impl FromStr for Flag {
    type Err = UnknownFlag;

    /// **Closed on purpose:** a misspelling has to fail rather than become a
    /// clause that is quietly false, which would disable a key with no
    /// diagnosis. The set is complete before its features exist for the same
    /// reason -- a flag that landed with its feature would let a clause
    /// referring to it parse today and mean nothing.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let mut index = 0;
        while index < Flags::NAMES.len() {
            let (flag, spelling) = Flags::NAMES[index];
            if spelling.as_bytes() == name.as_bytes() {
                return Ok(Flag(flag));
            }
            index += 1;
        }
        Err(UnknownFlag {
            name: name.to_owned(),
        })
    }
}

impl fmt::Display for Flag {
    /// The one name, always. Total because the value holds exactly one bit.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (flag, spelling) in Flags::NAMES {
            if holds(self.0, *flag) {
                return f.write_str(spelling);
            }
        }
        // Unreachable: the field is private and `FromStr` sets it from a NAMES
        // entry, so every value matches one of them. Writing nothing is still
        // better than a panic on a formatter.
        Ok(())
    }
}
