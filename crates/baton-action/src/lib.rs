//! Action registry, keymap, `when` predicates, and the input router.
//!
//! Actions are data. This crate does not know `iced` exists: a UI element
//! never calls a function, it emits an action, and the palette is just a
//! search over this registry.
//!
//! There are **two tables**, and the split is the design rather than an
//! accident of layout. [`ActionMeta`] says what an action *is* -- permanent,
//! and the same under every keymap. A binding says how it is *reached* --
//! replaceable, and there may be several or none. `when` belongs to the
//! binding, because what it really decides is whether a keypress is ours at
//! all or falls through to the terminal.

mod action;
mod actions;
mod keymap;
mod registry;

pub use action::{ActionId, ActionMeta, ArgKind, IssueSites};
pub use actions::{ACTIONS, STAGE1_SOURCE};
pub use keymap::{Binding, Chord, DEFAULT_KEYMAP, ParseError};
pub use registry::{ActionSource, Registry, RegistryError, try_merge};
