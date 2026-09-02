//! Grep-style guard: no pty write envelope outside the sanctioned files.
//!
//! The ticket's rule (#13, A11): never build `pane.on_key() -> pty.write()`,
//! because broadcast cannot be retrofitted past it. The compiler cannot see
//! that rule, so this test greps for it: the application crates' sources must
//! not reach the pty write envelope directly -- every byte goes through
//! `route_input`, and only the Delivery half (#14's
//! `baton-ui/src/terminal_event.rs`) translates a routing decision into a write.

use std::fs;
use std::path::{Path, PathBuf};

/// The crates whose sources may not touch the write envelope. `baton-term`
/// is deliberately absent: its backend IS the pty side, below the router,
/// and its pane-bound pointer telemetry is exempt from routing by ruling
/// (#107) -- this guard polices the application crates' side of that line.
const GUARDED: [&str; 4] = ["baton", "baton-action", "baton-core", "baton-ui"];

/// What reaching the envelope looks like in this codebase today: the term
/// backend's write command, the notifier that feeds the pty, and a direct
/// write call. Loose on purpose -- a false positive costs a glance, a false
/// negative ships the unroutable input path. (A bare `pty` token was tried
/// and rejected: it matches the word "empty" and every doc sentence that
/// explains the rule.)
const FORBIDDEN: [&str; 3] = ["BackendCommand::Write", ".notify(", ".write("];

/// Files allowed to name the envelope: exactly one, the Delivery adapter --
/// the file that turns a routing decision into a write (#14). Anything else
/// wanting on this list is the unroutable-input path this guard exists for.
const ALLOWED: [&str; 1] = ["baton-ui/src/terminal_event.rs"];

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_pty_write_reachable_outside_the_router_path() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut sources = Vec::new();
    for krate in GUARDED {
        // src/ only: tests (like this one) may name the tokens they hunt.
        rust_sources(&root.join(krate).join("src"), &mut sources);
    }
    assert!(
        sources.len() >= 10,
        "the walk found too few files to be looking at the right tree"
    );

    let mut hits = Vec::new();
    for path in sources {
        let relative = path
            .strip_prefix(&root)
            .expect("under crates/")
            .to_string_lossy()
            .into_owned();
        if ALLOWED.contains(&relative.as_str()) {
            continue;
        }
        let text = fs::read_to_string(&path).expect("readable source");
        for (number, line) in text.lines().enumerate() {
            for token in FORBIDDEN {
                if line.contains(token) {
                    hits.push(format!("{relative}:{}: {token}", number + 1));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "pty write envelope reached outside the router path:\n{}",
        hits.join("\n")
    );
}
