//! Screen assembly, projection, and the Elm wiring for `iced`.
//!
//! Deliberately a library with no `main`: a crate with a `main` cannot be
//! driven by `iced_test`'s headless `Simulator`, and the UI tests depend on
//! that.
//!
//! `view()` takes a projection -- a flat, display-only value -- not a domain
//! type. The projection is built by a pure function in this crate.
//! The Elm shell is assembled by [`App`], [`Message`], and `update`;
//! `terminal_event` handles the terminal arm and is the one pty-write site
//! behind the input router. Every user-visible string is injected through
//! `App::new` by `main` -- this crate writes no copy of its own.

mod app;
mod chrome;
mod keys;
mod palette_view;
mod project;
pub(crate) mod search;
mod terminal_event;
mod theme;
mod view;

pub use app::{App, Message};

/// The palette's ranking pass, exposed for the performance-floor test: the
/// measurement is a consumer too, and measuring through the public surface
/// keeps it honest about what the palette actually computes per keystroke.
pub fn palette_rank_for_measure<'a>(
    query: &str,
    labels: impl Iterator<Item = &'a str>,
) -> Vec<usize> {
    search::rank(query, labels)
        .into_iter()
        .map(|hit| hit.index)
        .collect()
}
