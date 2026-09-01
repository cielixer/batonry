//! The crate boundaries, enforced instead of documented (#16).
//!
//! Each rule below was prose in a CLAUDE.md or a `//!` block until this file;
//! now violating one fails `cargo test`, which CI runs on every pull request.
//! The dependency questions are asked of `cargo tree` -- the real resolved
//! graph, so a violation arriving transitively is caught the same as a direct
//! one -- and the source questions are asked of the files on disk.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace root, from this crate's own manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// The resolved dependency tree of one workspace crate, one package name per
/// line, resolved for **every target** and with **every feature on** -- a
/// violation that only Windows resolves, or that hides behind an optional
/// feature another crate enables, is still a violation. `edges` is `-e`'s
/// argument: `normal` where the rule is about what the built library carries
/// (dev-dependencies never leak into a consumer), `normal,dev` where it is
/// about extraction, which takes the tests along.
fn dependency_names(package: &str, edges: &str) -> Vec<String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        // The parse below reads plain text; a CI runner that turns cargo's
        // colour on wraps the " (*)" dedup marker in ANSI codes and every
        // deduplicated line stops matching (seen live on the first run).
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(workspace_root())
        .args(["tree", "-p", package, "-e", edges, "--target", "all"])
        .args(["--all-features", "--prefix", "none"])
        .args(["--format", "{p}"])
        .output()
        .expect("cargo tree runs");
    assert!(
        out.status.success(),
        "cargo tree -p {package} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect()
}

/// Panics if any resolved dependency of `package` starts with one of the
/// forbidden crate-name prefixes.
fn assert_tree_free_of(
    package: &str,
    edges: &str,
    forbidden: &[&str],
    rule: &str,
) {
    let names = dependency_names(package, edges);
    assert!(
        names.len() > 1,
        "cargo tree returned only {package} itself; the check saw nothing"
    );
    for name in &names {
        for prefix in forbidden {
            assert!(
                !(name == prefix
                    || name.starts_with(&format!("{prefix}-"))
                    || name.starts_with(&format!("{prefix}_"))),
                "{package} resolves {name}, which breaks the rule: {rule}"
            );
        }
    }
}

/// `baton-core` is the hexagon: inheritance resolution and the domain rules
/// have to be testable as pure functions, which no crate dragging in a UI
/// toolkit or an IO runtime can promise.
#[test]
fn core_depends_on_no_ui_and_no_io() {
    assert_tree_free_of(
        "baton-core",
        "normal",
        &[
            "iced",
            "winit",
            "wgpu",
            "tokio",
            "rusqlite",
            "smol",
            "async-std",
        ],
        "baton-core depends on no UI and no IO (root CLAUDE.md section 2); \
         the domain must stay testable as pure functions",
    );
}

/// An action is data. The moment this crate sees `iced`, actions stop being
/// publishable from anything that is not the UI.
#[test]
fn action_does_not_know_iced_exists() {
    assert_tree_free_of(
        "baton-action",
        "normal",
        &["iced", "winit", "wgpu"],
        "baton-action does not know iced exists (root CLAUDE.md section 2); \
         an action is data",
    );
}

/// `baton-term` is a terminal widget and nothing more, so that it can be
/// extracted into its own repository. A `baton-core` edge would tie the
/// extraction to our domain forever -- and dev and build edges count,
/// because extraction takes the tests and the build script along.
#[test]
fn term_does_not_know_the_domain() {
    assert_tree_free_of(
        "baton-term",
        "normal,build,dev",
        &["baton-core"],
        "baton-term does not know baton-core (root CLAUDE.md section 2); \
         it must stay extractable",
    );
}

/// Every platform branch lives in `baton-platform` alone (root CLAUDE.md).
/// An OS branch anywhere else is a decision made in a place nothing audits.
/// The scan covers **every Rust file in the crate and its manifest** -- a
/// platform-specific `[target.'cfg(...)'.dependencies]` table and a build
/// script reading `CARGO_CFG_TARGET_OS` are the standard routes around a
/// src-only grep -- and flags the family and vendor axes too. The two
/// hardcopies are upstream code and exempt, `baton-platform` is the rule's
/// owner, and this file exempts itself: like the english-only check script
/// (#95), the scanner necessarily contains the tokens it hunts.
#[test]
fn cfg_target_os_lives_in_baton_platform_alone() {
    let crates_dir = workspace_root().join("crates");
    let exempt_crates = ["baton-platform", "winit", "baton-term"];
    let exempt_files = ["baton-core/tests/boundaries.rs"];
    let needles = [
        "target_os",
        "TARGET_OS",
        "target_family",
        "TARGET_FAMILY",
        "target_vendor",
        "TARGET_VENDOR",
    ];

    let mut hits = Vec::new();
    for entry in fs::read_dir(&crates_dir).expect("crates/") {
        let dir = entry.expect("dir entry").path();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        if !dir.is_dir() || exempt_crates.contains(&name.as_str()) {
            continue;
        }
        let mut files = vec![dir.join("Cargo.toml")];
        rust_sources_everywhere(&dir, &mut files);
        for path in files {
            let relative = path
                .strip_prefix(&crates_dir)
                .expect("under crates/")
                .to_string_lossy()
                .into_owned();
            if exempt_files.contains(&relative.as_str()) {
                continue;
            }
            let text = fs::read_to_string(&path).expect("readable file");
            for (n, line) in text.lines().enumerate() {
                // The tokens themselves, not `cfg(target_os`: `any(...)`,
                // `not(any(...))`, `cfg_attr`, a manifest target table and a
                // build script's env var all bury the token deeper, and the
                // root contract's own example is the `not(any(...))` form.
                // A doc sentence naming a token costs one glance; a missed
                // branch is a silent platform decision.
                if needles.iter().any(|needle| line.contains(needle)) {
                    hits.push(format!("{relative}:{}", n + 1));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "a platform branch outside baton-platform; every one lives there \
         alone (root CLAUDE.md section 2), so a platform decision is never \
         made where nothing audits it:\n{}",
        hits.join("\n")
    );
}

/// `baton-ui` must stay headless-drivable: with a `main` it cannot be driven
/// by `iced_test`'s `Simulator`, and it is easy to add one by accident.
#[test]
fn ui_has_no_main() {
    let ui = workspace_root().join("crates/baton-ui");
    assert!(
        !ui.join("src/main.rs").exists(),
        "baton-ui grew src/main.rs; it must stay a library so iced_test's \
         Simulator can drive it (root CLAUDE.md section 2)"
    );
    let manifest = fs::read_to_string(ui.join("Cargo.toml")).expect("manifest");
    assert!(
        !manifest.contains("[[bin]]"),
        "baton-ui declares a [[bin]] target; it must stay a library so \
         iced_test's Simulator can drive it"
    );
    let mut files = Vec::new();
    rust_sources(&ui.join("src"), &mut files);
    for path in files {
        let text = fs::read_to_string(&path).expect("readable source");
        assert!(
            !text.contains("fn main("),
            "{} defines fn main; baton-ui must stay a library so iced_test's \
             Simulator can drive it",
            path.display()
        );
    }
}

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

/// Every `.rs` under the crate directory: src, tests, benches, examples, and
/// `build.rs` at the top. `target/` is skipped -- build output, not source.
fn rust_sources_everywhere(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable crate dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rust_sources_everywhere(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
