//! Mechanism for a command system: names, chords, keymap rows, and an index.
//!
//! **This crate does not know what application it serves.** It has no notion of
//! a palette, a menu, a click, a pane or a host; it does not depend on `iced`;
//! and it does not contain a list of anything's actions. Those are the
//! application's, and keeping them out is what makes this crate worth
//! extracting later.
//!
//! What it does provide:
//!
//! - [`Keystroke`] and one canonical syntax for writing one, so a configuration
//!   file and a built-in default go through the same parser. **Keys are
//!   physical positions, not the characters they produce** -- see below.
//! - [`Binding`] -- a keystroke, the name of what it reaches, and an opaque
//!   condition. The rows are yours.
//! - [`Registry`], [`Source`] and [`try_merge`] -- stable indices for a set of
//!   named things, contributed by several crates, with duplicates refused
//!   loudly.
//!
//! # Vocabulary
//!
//! - **[`Action`]** -- one thing this app can do. Not a function: a row in a
//!   table, named permanently, which is what makes a keymap file and a command
//!   palette possible at all.
//! - **[`Channels`]** -- where an action can be invoked *other than by a key*:
//!   palette, click, menu, drag. There is no `KEY` bit, because key reachability
//!   is whatever the keymap says and changes when someone rebinds.
//! - **[`ArgKind`]** -- the *shape* of the argument an action expects, not the
//!   type. The value travels beside the id when the action is issued.
//! - **Id** -- the permanent name, by convention `<domain>.<verb>[.<variant>]`
//!   as in `host.connect`. Configuration joins on it, so **it never changes**.
//! - **[`ActionId`]** -- **not** the above, despite reading like it. A name is a
//!   string that outlives releases; an `ActionId` is a position the registry
//!   hands out at boot, so resolving one is a bounds check instead of a lookup.
//!   It means nothing outside the run that issued it. Names cross process
//!   boundaries; `ActionId`s never should.
//! - **[`Binding`]** -- one way to reach one action: a chord, and optionally a
//!   condition. Several may point at one action, or none.
//! - **[`Keystroke`]** -- modifiers plus one physical key, written
//!   `meta+shift+KeyK`. One physical keystroke is exactly one value, because
//!   conflict detection groups by it.
//! - **Physical vs logical** -- the distinction the whole design turns on. A
//!   physical key is a position; a logical key is what that position produces
//!   under the layout in effect. Measured on macOS: with the Korean 2-Set input
//!   source active, the physical `A` key still arrives as `Code::KeyA` while the
//!   logical key becomes `ㅁ`, and it stays `ㅁ` with Command held. **Bindings
//!   name physical keys**, or they stop working the moment someone types in a
//!   non-Latin script. The logical key is for *showing* a binding to a person,
//!   which is a different job.
//! - **`meta`, not `cmd`** -- the syntax names no platform. `META` is Command
//!   on macOS and the Windows key elsewhere; which modifier an application
//!   treats as primary is the application's decision, made where platform
//!   decisions belong.
//! - **Condition (`when`)** -- a guard on a *binding*, deciding whether that
//!   keystroke becomes an action or **falls through** to whatever else is
//!   listening. Not "is this action greyed out". Opaque here.
//!
//! The key vocabulary itself is [`keyboard_types`]: the W3C UI Events set, 216
//! physical codes and a modifier bitflag. Not a windowing library, so this crate
//! stays free of one -- and not ours to maintain, so every key is reachable
//! without a table here going stale.
//! - **[`Registry`]** -- rows plus a name index, built once from each
//!   [`Source`].
//! - **[`Source`]** -- one table with a name, so a duplicate can say *which two*
//!   collided. Borrowed for the built-in table, owned for a loaded one.
//!
//! # Shape
//!
//! Plain data with free functions over it, and methods only where a type owns
//! state and an invariant -- which here is [`Registry`] alone. No trait objects
//! and no dynamic dispatch anywhere.

mod action;
mod catalog;
mod keystroke;
mod registry;

pub use action::{
    Action, ArgKind, Binding, CLICK, Channels, DRAG, MENU, NO_CHANNEL, PALETTE,
    also, reaches,
};
pub use catalog::{ACTIONS, BUILT_IN, DEFAULT_KEYMAP};

pub use keystroke::{Keystroke, KeystrokeError};
pub use registry::{ActionId, MergeError, Registry, Source, index, try_merge};
