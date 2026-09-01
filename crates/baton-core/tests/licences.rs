//! Licence gate: a new dependency cannot arrive under an unlisted licence.
//!
//! The repository is public and MIT, and it vendors code under MIT and
//! Apache-2.0. This asks `cargo tree` for every resolved package's SPDX
//! expression -- every target and every edge kind, because a build script's
//! dependency or a Windows-only one ships all the same -- and refuses
//! anything not on the allowlist below -- the
//! in-tree equivalent of a `cargo deny` licence check (#16), with nothing
//! to install and a failure that names the package.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Every SPDX expression the tree carries today, listed exactly.
///
/// An expression, not a licence: `Apache-2.0 OR GPL-2.0-only` is here
/// because `OR` lets us choose the Apache side (self_cell is the carrier),
/// while a bare `GPL-2.0-only` would rightly fail. Growing this list is a
/// reviewed decision, which is the point.
const ALLOWED: &[&str] = &[
    "(MIT OR Apache-2.0) AND Unicode-3.0",
    "Apache-2.0",
    "Apache-2.0 AND MIT",
    "Apache-2.0 OR GPL-2.0-only",
    "Apache-2.0 OR MIT",
    "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
    "Apache-2.0/MIT",
    "BSD-2-Clause",
    "BSD-2-Clause OR Apache-2.0 OR MIT",
    "BSD-3-Clause",
    "CC0-1.0",
    "MIT",
    "MIT OR Apache-2.0",
    "MIT OR Apache-2.0 OR Zlib",
    "MIT/Apache-2.0",
    "Unlicense OR MIT",
    "Zlib",
    "Zlib OR Apache-2.0 OR MIT",
    // The seven below only resolve off-host or on build/dev edges.
    "0BSD OR MIT OR Apache-2.0",
    "BSD-3-Clause OR MIT OR Apache-2.0",
    "BSL-1.0",
    "ISC",
    // OR lets us take the MIT side; a bare LGPL would rightly fail.
    "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
    "MIT OR Zlib OR Apache-2.0",
    "Unlicense/MIT",
];

#[test]
fn every_dependency_licence_is_allowlisted() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(cargo)
        // The parse below reads plain text; a CI runner that turns cargo's
        // colour on wraps the " (*)" dedup marker in ANSI codes and every
        // deduplicated line stops matching (seen live on the first run).
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(root)
        .args(["tree", "--workspace", "-e", "normal,build,dev"])
        .args(["--target", "all", "--prefix", "none"])
        .args(["--format", "{p}\t{l}"])
        .output()
        .expect("cargo tree runs");
    assert!(
        out.status.success(),
        "cargo tree failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = String::from_utf8(out.stdout).expect("utf-8");
    let mut offenders = BTreeSet::new();
    let mut seen = 0usize;
    for line in text.lines() {
        let Some((package, licence)) = line.split_once('\t') else {
            continue;
        };
        // cargo tree marks a deduplicated subtree with a trailing "(*)".
        let licence = licence.trim_end_matches(" (*)").trim();
        seen += 1;
        // Our own workspace crates carry the workspace licence field; they
        // are checked like everything else, not skipped.
        if licence.is_empty() {
            offenders.insert(format!("{package}: no licence declared"));
        } else if !ALLOWED.contains(&licence) {
            offenders.insert(format!("{package}: {licence}"));
        }
    }
    assert!(
        seen > 300,
        "only {seen} packages seen; the workspace tree is larger than that, \
         so the parse went wrong"
    );
    assert!(
        offenders.is_empty(),
        "dependencies carry licences outside the allowlist; a public MIT \
         repository cannot quietly absorb them -- either the licence is \
         compatible and gets an allowlist line in a reviewed commit, or the \
         dependency goes:\n{}",
        offenders.into_iter().collect::<Vec<_>>().join("\n")
    );
}
