//! Korean input on macOS.
//!
//! The macOS Korean input method does not use the preedit model. It composes
//! jamo into syllables by calling `insertText:` with a `replacementRange` --
//! it inserts a character, reads it back, and replaces it. That requires the
//! client to expose its document through `selectedRange` and
//! `attributedSubstringForProposedRange:`.
//!
//! winit 0.30.13 as published answers `{NSNotFound, 0}` and `None`, so the
//! input method cannot use that model and the first jamo never joins the
//! composition: typing "한글" produces "ㅎㅏㄴ글". `vendor/winit` fixes it by
//! exposing only the in-flight composition as the document and absorbing the
//! replacement, so the app sees nothing but ordinary `Preedit` / `Commit`.
//!
//! Two things are tested here:
//!   1. The recordings still mean what we think they mean (fixtures below)
//!   2. The patch is actually wired into the build -- replaying a fixture
//!      cannot catch a missing `[patch.crates-io]`, because a fixture is just
//!      a file

/// Applies a recorded IME event stream the way a terminal must: preedit is
/// held for display only, and just the commits reach the pty.
#[derive(Default)]
struct PtyBytes {
    committed: String,
    preedit: String,
}

impl PtyBytes {
    fn apply(&mut self, line: &str) {
        match field(line, "ev").as_deref() {
            Some("commit") => {
                self.preedit.clear();
                self.committed
                    .push_str(&field(line, "text").unwrap_or_default());
            },
            Some("preedit") => {
                self.preedit = field(line, "content").unwrap_or_default();
            },
            // A raw key goes straight out, but only if it carries a single
            // printable character. Named keys were recorded as their debug
            // label (`Named(Super)`), and control characters are commands.
            Some("key") => {
                let t = field(line, "text").unwrap_or_default();
                let mut chars = t.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if !c.is_control() => {
                        self.committed.push(c)
                    },
                    _ => {},
                }
            },
            _ => {},
        }
    }
}

/// Minimal extraction of one string field. The fixtures are one flat JSON
/// object per line, so this avoids a serde dependency in a test.
fn field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\": \"");
    let i = line.find(&pat)? + pat.len();
    let mut out = String::new();
    let mut chars = line[i..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'r' => out.push('\r'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(
                        u32::from_str_radix(&hex, 16).ok()?,
                    )?);
                },
                c => out.push(c),
            },
            c => out.push(c),
        }
    }
    None
}

fn replay(path: &str) -> PtyBytes {
    let raw = std::fs::read_to_string(path).expect("fixture");
    let mut out = PtyBytes::default();
    for line in raw
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
    {
        out.apply(line);
    }
    out
}

/// With the fix, the recorded stream composes into what the human typed.
#[test]
fn recorded_hangul_composes_into_syllables() {
    let out = replay("tests/fixtures/hangul-2set.jsonl");
    assert_eq!(out.committed, "한글 ");
    assert_eq!(out.preedit, "", "no composition should be left dangling");
}

/// The pre-fix recording is kept because the difference between the two files
/// *is* the bug. If someone changes how commits are interpreted, both tests
/// have to move together.
#[test]
fn recording_from_before_the_fix_shows_the_broken_composition() {
    let out = replay("tests/fixtures/hangul-2set-before-winit-fix.jsonl");
    // Jamo committed one at a time instead of forming syllables.
    assert_eq!(out.committed, "ㅎㅏㄴ글 바도");
}

/// Guards the build configuration itself.
///
/// The replay tests above pass even with the patch removed -- fixtures are
/// files. Without this, a PR that deletes `[patch.crates-io]` goes green and
/// Korean input silently breaks again.
#[test]
fn vendored_winit_ime_patch_is_wired_up() {
    let manifest = std::fs::read_to_string("../../Cargo.toml")
        .expect("workspace manifest");
    assert!(
        manifest.contains("[patch.crates-io]")
            && manifest.contains("winit = { path = \"vendor/winit\" }"),
        "the workspace no longer patches vendor/winit, so the published winit \
         is in use and the first Hangul jamo is dropped again"
    );

    let view = std::fs::read_to_string(
        "../../vendor/winit/src/platform_impl/macos/view.rs",
    )
    .expect("vendored winit view.rs");
    for needle in [
        // absorbs insert-then-replace
        "fn handle_composing_insert",
        // decides what may be held back; keeps Latin typing at zero latency
        "fn is_composing_script",
        // the document readback the input method needs
        "attributedSubstringForProposedRange",
        "committed_utf16",
    ] {
        assert!(
            view.contains(needle),
            "{needle:?} is gone from vendor/winit -- the IME fix was reverted"
        );
    }
    // And the shape the upstream issue points at must not come back.
    assert!(
        !view.contains("NSRange::new(NSNotFound as NSUInteger, 0)\n        }"),
        "selectedRange answers NSNotFound again (upstream winit issue #4666)"
    );
}
