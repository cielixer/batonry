//! Mechanism for a command system: names, chords, keymap rows, and an index.
//!
//! The catalog lives here ([`ACTIONS`], [`DEFAULT_KEYMAP`]): one place to look,
//! and a compile-time constant rather than a wiring step. Nothing in the crate
//! depends on a UI toolkit, knows what a pane or a host is, or names a platform.
//!
//! What it provides:
//!
//! - [`Keystroke`] and one canonical syntax for writing one, so a configuration
//!   file and a built-in default go through the same parser. **Keys are
//!   physical positions, not the characters they produce** -- see below.
//! - [`Binding`] -- a keystroke, the name of what it reaches, and an opaque
//!   condition. The rows are yours.
//! - [`Registry`], [`Source`] and [`try_merge`] -- stable indices for a set of
//!   named things, contributed by several sources, with duplicates refused
//!   loudly.
//!
//! # Vocabulary
//!
//! - **[`Action`]** -- one thing this app can do. Not a function: a row in a
//!   table, named permanently, which is what makes a keymap file and a command
//!   palette possible at all.
//! - **[`Channels`]** -- where an action can be invoked *other than by a key*:
//!   palette, click, menu, drag. There is no `KEY` bit, because key reachability
//!   is whatever the keymap says and changes when someone rebinds. The empty set
//!   is [`KEY_ONLY`], and it is a value to build a row with rather than one to
//!   ask about: every set contains it.
//! - **[`ArgShape`]** -- what an action expects beside its id: a host, a pane, a
//!   tab position. A shape and not a type, because this crate cannot name
//!   `baton-core`'s types without depending on it.
//! - **Id** -- the permanent name, by convention `<domain>.<verb>[.<variant>]`
//!   as in `host.connect`. Configuration joins on it, so **it never changes**.
//! - **[`ActionId`]** -- **not** the above, despite reading like it. A name is a
//!   string that outlives releases; an `ActionId` is a position the registry
//!   hands out at boot, so resolving one is a bounds check instead of a lookup.
//!   It means nothing outside the **registry** that issued it -- two registries
//!   in one process hand out overlapping indices. Names cross process
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
//! - **[`Registry`]** -- the merged actions plus a name index, built once from
//!   each [`Source`]. It hands out `(ActionId, &Action)` and never the slice, so
//!   a caller always has something it can dispatch.
//! - **[`Source`]** -- one contribution with a name, so a duplicate can say
//!   *which two* collided. Borrowed for the built-in table, owned for a loaded
//!   one.
//!
//! The key vocabulary itself is [`keyboard_types`]: the W3C UI Events set, 216
//! physical codes and a modifier bitflag. Not a windowing library, so this crate
//! stays free of one -- and not ours to maintain, so every key is reachable
//! without a table here going stale.
//!
//! # Style
//!
//! Plain data with free functions over it, and methods only where a type owns
//! state and an invariant -- which here is [`Registry`] alone. No trait objects
//! and no dynamic dispatch anywhere.

mod action;
mod catalog;
mod keystroke;
mod registry;

pub use action::{
    Action, ArgShape, Binding, CLICK, Channels, DRAG, KEY_ONLY, MENU, PALETTE,
    reachable_from, union,
};
pub use catalog::{ACTIONS, BUILT_IN, DEFAULT_KEYMAP};

pub use keystroke::{Keystroke, KeystrokeError};
pub use registry::{ActionId, MergeError, Registry, Source, try_merge};
