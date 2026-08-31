//! Shared, hand-transcribed spec data for the integration tests.
//!
//! Transcribed by hand from `ux/interactions.md` section 2 rather than read
//! from the crate, because a test that iterates the same table the code
//! iterates asserts nothing. One transcription, shared by the binaries that
//! need it via `mod common`.

use baton_action::{Flags, holds};

/// Every flag with its canonical spelling, in declaration order.
pub const EVERY_FLAG: [(Flags, &str); 17] = [
    (Flags::PANE_FOCUSED, "pane_focused"),
    (Flags::PANE_LIVE, "pane_live"),
    (Flags::PANE_DISCONNECTED, "pane_disconnected"),
    (Flags::HAS_SELECTION, "has_selection"),
    (Flags::SEARCH_OPEN, "search_open"),
    (Flags::PALETTE_OPEN, "palette_open"),
    (Flags::DIALOG_OPEN, "dialog_open"),
    (Flags::EDITING_TEXT, "editing_text"),
    (Flags::SIDEBAR_HOSTS, "sidebar_hosts"),
    (Flags::SIDEBAR_WORKSPACES, "sidebar_workspaces"),
    (Flags::HOST_SELECTED, "host_selected"),
    (Flags::SCRATCH_ACTIVE, "scratch_active"),
    (Flags::WORKSPACE_ACTIVE, "workspace_active"),
    (Flags::HAS_JUMP, "has_jump"),
    (Flags::HAS_QUEUED_INPUT, "has_queued_input"),
    (Flags::DIALOG_HOSTKEY_NEW, "dialog_hostkey_new"),
    (Flags::DIALOG_HOSTKEY_CHANGED, "dialog_hostkey_changed"),
];

/// The spellings a context assignment holds, for a failure message a person
/// can act on.
// Compiled once per test binary; not every binary uses every item.
#[allow(dead_code)]
pub fn spellings(ctx: Flags) -> impl Iterator<Item = &'static str> {
    EVERY_FLAG
        .iter()
        .filter(move |(flag, _)| holds(ctx, *flag))
        .map(|(_, spelling)| *spelling)
}
