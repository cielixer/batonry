//! Inside the hexagon: domain types plus the port traits they declare.
//!
//! This crate must not depend on a UI toolkit or on I/O. Inheritance
//! resolution and the domain rules have to be testable as pure functions.
//! If `iced`, `tokio` or `rusqlite` shows up in Cargo.toml, that is a bug.
//!
//! [`PaneId`] is the first domain type to land; the rest arrive through
//! stages 1 and 2.
//!
//! # Vocabulary
//!
//! - **[`PaneId`]** -- one pane on screen, minted by the shell. Not a session
//!   id (A5) and never persisted.
//! - **Router ([`route_input`])** -- every byte heading for a terminal goes
//!   through this one pure function: keystrokes, paste, snippet execution,
//!   palette sends (A11). No pane owns its input, which is what keeps
//!   broadcast insertable later, and the bytes pass through exactly as given
//!   -- nothing on this path may append a newline. The routing state
//!   (targets, focus, which panes are live) is the application's; the write
//!   is the adapter's; this function is only the decision between them.
//! - **[`TargetSet`]** -- which panes receive a dispatch: the focused pane,
//!   or an explicit set (the broadcast shape, deliberately unused in M1).
//! - **[`Store`]** -- the persistence port for app-wide convenience state;
//!   SQLite and in-memory test doubles implement it outside this crate.
//! - **Delivery** -- one router decision handed onward as a pane and an
//!   unchanged byte slice: [`route_input`]'s `deliver` callback. Routing chooses
//!   which live panes receive input; delivery performs the write -- that
//!   half lives in the application's Delivery adapter (#14 landed it as
//!   `baton-ui`'s `terminal_event.rs`), never here. Stage 2 puts sessions behind
//!   the `Substrate` port (A1) and its Delivery resolves a pane to its
//!   session through `Substrate::send`.

/// Identifies one pane on screen.
///
/// The shell mints these. A pane is a place input can be routed to and a
/// session can be shown in -- it is **not** a session: session identifiers
/// are extensible strings with their own contract (A5), never persisted
/// through this type, and a pane outlives reconnects that replace its
/// session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct PaneId(u64);

impl PaneId {
    /// Creates an identifier for one pane on screen.
    pub const fn new(n: u64) -> PaneId {
        PaneId(n)
    }
}

mod router;
mod store;

pub use router::{TargetSet, route_input};
pub use store::Store;
