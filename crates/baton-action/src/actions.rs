//! The stage-1 action rows.
//!
//! Ten, and deliberately not the whole ~82-row table. The contract runs one
//! way: **every implemented behaviour has a row here**, not every row has an
//! implementation. Registering an action nothing executes would put an entry
//! in the palette that does nothing, which is worse than transcribing late.
//!
//! Labels are the canonical user-visible strings, and `issue` mirrors the
//! non-key half of the specification's issue column.

use crate::action::{ActionMeta, ArgKind, IssueSites};
use crate::registry::ActionSource;

/// Actions stage 1 can actually perform.
pub const ACTIONS: &[ActionMeta] = &[
    meta("app.quit", "Quit", IssueSites::PALETTE),
    meta("palette.open", "Command Palette", IssueSites::empty()),
    meta("palette.close", "Close Palette", IssueSites::empty()),
    meta("palette.confirm", "Run", IssueSites::empty()),
    meta("palette.next", "Next Result", IssueSites::empty()),
    meta("palette.prev", "Previous Result", IssueSites::empty()),
    meta(
        "term.copy",
        "Copy",
        IssueSites::PALETTE.union(IssueSites::MENU),
    ),
    meta(
        "term.paste",
        "Paste",
        IssueSites::PALETTE.union(IssueSites::MENU),
    ),
    meta("term.select_all", "Select All", IssueSites::empty()),
    meta("term.clear", "Clear Screen", IssueSites::PALETTE),
];

/// Every stage-1 action takes no argument, so the shape is not repeated ten
/// times. An action that needs one spells it out instead of using this.
const fn meta(
    id: &'static str,
    label: &'static str,
    issue: IssueSites,
) -> ActionMeta {
    ActionMeta {
        id,
        label,
        issue,
        arg: ArgKind::None,
    }
}

/// This crate's contribution to the boot-time merge.
pub const STAGE1_SOURCE: ActionSource = ActionSource {
    name: "baton-action",
    actions: ACTIONS,
};
