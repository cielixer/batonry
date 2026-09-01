//! Keychain, single-instance lock, paths, clock, uuid.
//!
//! Every `#[cfg(target_os)]` in the workspace lives here. If an OS branch
//! appears in another crate, that is a bug -- this is the one place to look
//! when adding a platform.

/// The shell a new local terminal runs when nothing chose one: `$SHELL` when
/// the environment says, otherwise the platform's own default.
///
/// Lives here and not in the UI because which shell a platform defaults to
/// is an OS decision, and `baton-ui` does not know the OS -- `main` asks
/// this crate and injects the answer.
pub fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        // A relative value cannot be trusted from an app's environment (no
        // login shell's PATH); the fallback is at least a real path.
        .filter(|shell| shell.starts_with('/'))
        .unwrap_or_else(|| FALLBACK.to_owned())
}

#[cfg(target_os = "macos")]
const FALLBACK: &str = "/bin/zsh";
#[cfg(all(unix, not(target_os = "macos")))]
const FALLBACK: &str = "/bin/sh";
#[cfg(not(unix))]
compile_error!("default shell undecided for this target; decide it here");

#[cfg(test)]
mod tests {
    /// Whatever the environment holds, the answer is an absolute path --
    /// the one property every caller relies on.
    #[test]
    fn the_default_shell_is_an_absolute_path() {
        assert!(super::default_shell().starts_with('/'));
    }
}
