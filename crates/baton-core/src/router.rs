//! Routes every byte heading for a terminal through one dispatch point (A11):
//! keystrokes, paste, snippet execution, and palette sends.
//!
//! No pane owns its input: a `pane.on_key -> pty.write` path must never exist,
//! because broadcast cannot be retrofitted past it. Dispatch also preserves
//! the raw byte path without adding a newline, so a command can be prefilled
//! while the user remains responsible for pressing Enter.
//!
//! **Routing is a pure function, not an object.** The state it reads -- the
//! target set, the focused pane, which panes are live -- belongs to the
//! application's one state tree, and the write itself belongs to the adapter
//! that owns sessions (A1). [`dispatch`] stands between the two: it decides
//! *which* panes receive *exactly which* bytes, and hands each decision to a
//! caller-supplied `deliver`. This crate never holds a pane map, a session,
//! or a file descriptor.
//!
//! [`TargetSet::Set`] is deliberately unused in M1 because no broadcast UI
//! ships yet. The variant makes enabling it later a change to one dispatch
//! site rather than to every input path.

use crate::PaneId;

/// Selects the pane or panes that receive dispatched input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TargetSet {
    /// Input goes to the focused pane, when there is one and it is live.
    Focused,
    /// Input goes to each listed pane that is still live.
    Set(Vec<PaneId>),
}

/// Sends `bytes`, exactly as given, to every targeted pane that is still
/// live; a pane that closed between targeting and dispatch is skipped
/// quietly, never panicked on.
///
/// `is_live` answers whether a pane can still receive input -- the caller
/// asks its own pane table. `deliver` performs one delivery; the adapter
/// behind it resolves the pane to a live session through `Substrate::send`
/// (A1). Nothing here may append or strip a byte: M2 stages the tmux install
/// command and M3 the agent one, both leaving the user to press Enter.
pub fn dispatch(
    targets: &TargetSet,
    focused: Option<PaneId>,
    is_live: impl Fn(PaneId) -> bool,
    bytes: &[u8],
    mut deliver: impl FnMut(PaneId, &[u8]),
) {
    match targets {
        TargetSet::Focused => {
            if let Some(id) = focused
                && is_live(id)
            {
                deliver(id, bytes);
            }
        },
        TargetSet::Set(ids) => {
            for &id in ids {
                if is_live(id) {
                    deliver(id, bytes);
                }
            }
        },
    }
}
