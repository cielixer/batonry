//! Inside the hexagon: domain types plus the port traits they declare.
//!
//! This crate must not depend on a UI toolkit or on I/O. Inheritance
//! resolution and the domain rules have to be testable as pure functions.
//! If `iced`, `tokio` or `rusqlite` shows up in Cargo.toml, that is a bug.
//!
//! Skeleton only; the types land in stage 1-2.
