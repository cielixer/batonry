//! Bindings and the chord parser.
//!
//! Two properties carry real weight here:
//!
//! - **A binding can be built from owned strings.** A user keymap file ships in
//!   M1 and TOML strings are owned at runtime. If that needed a leak or an
//!   intern, then "the user's file is more rows in the same table" -- the reason
//!   the action and binding tables are separate at all -- would be false.
//! - **One spelling per chord.** The conflict check groups bindings by chord, so
//!   two spellings of one physical chord would be two map keys and the check
//!   would miss the collision it exists to find.

use std::borrow::Cow;
use std::collections::HashSet;

use baton_action::{ACTIONS, Binding, Chord, DEFAULT_KEYMAP, ParseError};

#[test]
fn every_default_binding_names_an_action_that_exists() {
    let ids: HashSet<&str> = ACTIONS.iter().map(|a| a.id).collect();
    for b in DEFAULT_KEYMAP {
        assert!(
            ids.contains(b.action.as_ref()),
            "{:?} is bound to {:?}, which is not a registered action. A \
             dangling binding is a key that silently does nothing",
            b.key,
            b.action
        );
    }
}

/// The parse has to happen somewhere, and a test is strictly better than
/// discovering it as a dead key at runtime.
#[test]
fn every_default_key_parses() {
    for b in DEFAULT_KEYMAP {
        b.key.parse::<Chord>().unwrap_or_else(|e| {
            panic!("{} is bound to unparseable key {:?}: {e}", b.action, b.key)
        });
    }
}

#[test]
fn default_keymap_has_no_duplicate_chords() {
    // A chord may legitimately repeat when the `when` clauses are disjoint --
    // deciding that is #12's job. What must not happen is the same chord with
    // the same guard, because one of the two could never fire.
    let mut pairs = HashSet::new();
    for b in DEFAULT_KEYMAP {
        let chord = b.key.parse::<Chord>().unwrap();
        assert!(
            pairs.insert((chord, b.when.clone())),
            "{:?} appears twice with the same guard {:?}; one of them can \
             never fire",
            b.key,
            b.when
        );
    }
}

/// Stage 1 intercepts ten chords and nothing else. The specification's rule is
/// that a key not in the table goes to the terminal, so this list *is* the
/// interception set -- worth pinning so growth is deliberate.
#[test]
fn stage_one_intercepts_exactly_ten_chords() {
    assert_eq!(DEFAULT_KEYMAP.len(), 10);
    let bare = DEFAULT_KEYMAP
        .iter()
        .filter(|b| !b.key.contains('+'))
        .map(|b| b.key.as_ref())
        .collect::<HashSet<_>>();
    // Unmodified keys are the dangerous ones: they are what a person types into
    // a terminal. Only the palette's own navigation may claim any, and only
    // while the palette is open.
    assert_eq!(
        bare,
        HashSet::from(["escape", "enter", "up", "down"]),
        "an unmodified key was added to the keymap. Every one of these has to \
         be guarded by a `when`, or it stops reaching the terminal"
    );
    for b in DEFAULT_KEYMAP.iter().filter(|b| !b.key.contains('+')) {
        assert!(
            b.when.is_some(),
            "{:?} is unmodified and unguarded, so it would never reach the \
             terminal",
            b.key
        );
    }
}

#[test]
fn a_binding_can_be_built_from_owned_strings() {
    // What a TOML loader will do. No leak, no interning, no 'static promise.
    let action = String::from("term.paste");
    let key = String::from("cmd+shift+v");
    let when = String::from("pane_live");

    let b = Binding {
        action: Cow::Owned(action),
        key: Cow::Owned(key),
        when: Some(Cow::Owned(when)),
    };

    assert!(matches!(b.action, Cow::Owned(_)));
    b.key
        .parse::<Chord>()
        .expect("an owned key parses like a borrowed one");

    // And the two forms are interchangeable where it counts: a borrowed and an
    // owned binding with the same content are equal.
    let borrowed = Binding {
        action: Cow::Borrowed("term.paste"),
        key: Cow::Borrowed("cmd+shift+v"),
        when: Some(Cow::Borrowed("pane_live")),
    };
    assert_eq!(b, borrowed);
}

#[test]
fn chords_accept_the_canonical_spelling() {
    for good in [
        "cmd+k",
        "cmd+shift+d",
        "cmd+alt+left",
        "cmd+shift+alt+ctrl+a",
        "escape",
        "enter",
        "up",
        "down",
        "f2",
        "f12",
        "cmd+comma",
        "cmd+1",
        "z",
        "0",
    ] {
        good.parse::<Chord>()
            .unwrap_or_else(|e| panic!("{good:?} rejected: {e}"));
    }
}

#[test]
fn chords_reject_everything_that_is_not_canonical() {
    use ParseError as E;
    /// A spelling, and the error it has to produce. Pairing them means a case
    /// that fails for the wrong reason is a failure, not a pass.
    type Case = (&'static str, fn(&ParseError) -> bool);
    let cases: &[Case] = &[
        ("", |e| matches!(e, E::MissingKey)),
        ("cmd+", |e| matches!(e, E::MissingKey)),
        ("cmd+shift", |e| matches!(e, E::MissingKey)),
        ("+k", |e| matches!(e, E::EmptyComponent)),
        ("cmd++k", |e| matches!(e, E::EmptyComponent)),
        ("meta+k", |e| matches!(e, E::UnknownModifier { .. })),
        ("cmd+cmd+k", |e| matches!(e, E::RepeatedModifier { .. })),
        ("shift+cmd+k", |e| matches!(e, E::ModifierOutOfOrder { .. })),
        ("ctrl+alt+k", |e| matches!(e, E::ModifierOutOfOrder { .. })),
        ("cmd+K", |e| matches!(e, E::UnknownKey { .. })),
        ("cmd+space", |e| matches!(e, E::UnknownKey { .. })),
        ("f0", |e| matches!(e, E::UnknownKey { .. })),
        ("f25", |e| matches!(e, E::UnknownKey { .. })),
        ("Cmd+k", |e| matches!(e, E::UnknownModifier { .. })),
        ("cmd+é", |e| matches!(e, E::NonAscii)),
    ];
    for (bad, expected) in cases {
        let err = bad
            .parse::<Chord>()
            .map(|_| ())
            .expect_err(&format!("{bad:?} should not parse"));
        assert!(
            expected(&err),
            "{bad:?} rejected with the wrong error: {err:?}"
        );
        assert!(!err.to_string().is_empty());
    }
}

/// The property the conflict check depends on: one physical chord is one value.
/// Out-of-order spellings are rejected rather than normalised, so there is no
/// second spelling that could hash differently.
#[test]
fn one_physical_chord_is_one_value() {
    let chord = |s: &str| s.parse::<Chord>().unwrap();

    assert_eq!(chord("cmd+shift+k"), chord("cmd+shift+k"));
    assert_ne!(chord("cmd+k"), chord("cmd+shift+k"));
    assert_ne!(chord("cmd+k"), chord("ctrl+k"));
    assert_ne!(chord("f1"), chord("f2"));
    // `f` the letter is not `f1` the function key.
    assert_ne!(chord("f"), chord("f1"));

    // Usable as a hash-map key, which is how the conflict check will group.
    let mut set = HashSet::new();
    assert!(set.insert(chord("cmd+k")));
    assert!(!set.insert(chord("cmd+k")));
}
