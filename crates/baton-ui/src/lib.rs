//! Screen assembly, projection, and the Elm wiring for `iced`.
//!
//! Deliberately a library with no `main`: a crate with a `main` cannot be
//! driven by `iced_test`'s headless `Simulator`, and the UI tests depend on
//! that.
//!
//! `view()` takes a projection -- a flat, display-only value -- not a domain
//! type. The projection is built by a pure function in this crate.
