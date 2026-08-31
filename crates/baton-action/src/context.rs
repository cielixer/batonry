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
/// once already with `Channels`: [`NONE`] is contained by every set, so
/// `holds(anything, NONE)` is `true`, which makes it a value to build a set
/// from and never one to ask about.
///
/// That is also why a set is not what a clause's leaf holds. A leaf names one
/// condition, and [`Flag`] is the checked type that carries it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Flags(u32);

bitset!(Flags, holds, combine, NAMES:
    /// A terminal pane has focus.
    PANE_FOCUSED = "pane_focused",
    /// That pane is in the `live` state.
    PANE_LIVE = "pane_live",
    /// That pane is in the `failed` or `reconnecting` state.
    PANE_DISCONNECTED = "pane_disconnected",
    /// The terminal has a selection.
    HAS_SELECTION = "has_selection",
    /// The terminal's search bar is open.
    SEARCH_OPEN = "search_open",
    /// The command palette is open.
    PALETTE_OPEN = "palette_open",
    /// A modal is open, such as host editing or a confirmation.
    DIALOG_OPEN = "dialog_open",
    /// A text input has focus: a dialog field, the palette's query, renaming.
    EDITING_TEXT = "editing_text",
    /// The sidebar is in hosts mode.
    SIDEBAR_HOSTS = "sidebar_hosts",
    /// The sidebar is in workspaces mode.
    SIDEBAR_WORKSPACES = "sidebar_workspaces",
    /// A host card is selected in the sidebar.
    HOST_SELECTED = "host_selected",
    /// What is on screen is a scratch.
    SCRATCH_ACTIVE = "scratch_active",
    /// What is on screen is a named workspace.
    WORKSPACE_ACTIVE = "workspace_active",
    /// That host has a jump path configured.
    HAS_JUMP = "has_jump",
    /// Keys typed while the connection was down are still queued.
    HAS_QUEUED_INPUT = "has_queued_input",
    /// The modal confirming a host key seen for the first time is open.
    DIALOG_HOSTKEY_NEW = "dialog_hostkey_new",
    /// The modal confirming a **changed** host key is open.
    DIALOG_HOSTKEY_CHANGED = "dialog_hostkey_changed",
);

/// Nothing holds. A value to start a set from, never one to ask about.
pub const NONE: Flags = Flags(0);

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
        while index < NAMES.len() {
            let (flag, spelling) = NAMES[index];
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
        for (flag, spelling) in NAMES {
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
