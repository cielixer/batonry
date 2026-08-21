//! Keychain, single-instance lock, paths, clock, uuid.
//!
//! Every `#[cfg(target_os)]` in the workspace lives here. If an OS branch
//! appears in another crate, that is a bug -- this is the one place to look
//! when adding a platform.
