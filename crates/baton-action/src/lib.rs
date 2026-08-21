//! Action registry, keymap, `when` predicates, and the input router.
//!
//! Actions are data. This crate does not know `iced` exists: a UI element
//! never calls a function, it emits an action, and the palette is just a
//! search over this registry.
