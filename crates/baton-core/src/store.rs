//! The persistence port for convenience state.

/// The persistence port for app-wide convenience state.
///
/// Export/import and an in-memory test double are named consumers. Setters are
/// infallible from the caller's view: a store that cannot write logs nothing
/// user-visible in stage 1, because this cache of convenience state may be
/// lost without breaking the app.
pub trait Store {
    /// Reads one app-wide preference, or returns `None` when it is unavailable.
    fn app_pref(&self, key: &str) -> Option<String>;

    /// Stores one app-wide preference without surfacing persistence failures.
    fn set_app_pref(&mut self, key: &str, value: &str);
}
