//! The keystroke syntax, and the property conflict detection depends on.
//!
//! **Matching is on the physical key.** Measured on macOS: with the Korean
//! 2-Set input source active, the physical `A` key still arrives as
//! `Code::KeyA` while the logical key becomes `ㅁ` -- and stays `ㅁ` with
//! Command held, since macOS does not special-case command chords. Matching on
//! the produced character would lose `⌘A` the moment someone types Korean.
//! These tests pin the physical reading.

use std::borrow::Cow;
use std::collections::HashSet;

use baton_action::{Binding, Keystroke, KeystrokeError};
use keyboard_types::{Code, Modifiers};

fn ks(s: &str) -> Keystroke {
    s.parse().unwrap_or_else(|e| panic!("{s:?} rejected: {e}"))
}

/// Pins the modifier table: each of the four names maps to the flag it claims,
/// and the flag prints back as the same name.
#[test]
fn each_modifier_parses_to_itself_and_prints_back() {
    for (text, flag) in [
        ("meta", Modifiers::META),
        ("shift", Modifiers::SHIFT),
        ("alt", Modifiers::ALT),
        ("control", Modifiers::CONTROL),
    ] {
        let k = ks(&format!("{text}+KeyA"));
        assert_eq!(k.modifiers, flag, "{text} parsed to the wrong flag");
        assert_eq!(
            k.to_string(),
            format!("{text}+KeyA"),
            "{text} printed back wrongly"
        );
    }
}

#[test]
fn the_physical_key_is_what_is_stored() {
    assert_eq!(ks("meta+a").code, Code::KeyA);
    assert_eq!(ks("meta+KeyA").code, Code::KeyA);
    assert_eq!(ks("1").code, Code::Digit1);
    assert_eq!(ks("Digit1").code, Code::Digit1);
    assert_eq!(ks("Escape").code, Code::Escape);
    assert_eq!(ks("F2").code, Code::F2);

    // Shorthand and long form are the same value, so a keymap may mix them.
    assert_eq!(ks("meta+a"), ks("meta+KeyA"));
}

/// The keys the specification's own keymap needs, none of which the previous
/// hand-rolled enum could express.
#[test]
fn the_keys_the_keymap_actually_needs_all_exist() {
    for (spelling, expected) in [
        ("meta+BracketRight", Code::BracketRight),
        ("meta+shift+BracketLeft", Code::BracketLeft),
        ("meta+Equal", Code::Equal),
        ("meta+Minus", Code::Minus),
        ("shift+PageUp", Code::PageUp),
        ("shift+PageDown", Code::PageDown),
        ("Backspace", Code::Backspace),
        ("Tab", Code::Tab),
        ("Space", Code::Space),
        ("meta+Comma", Code::Comma),
        ("Enter", Code::Enter),
        ("ArrowUp", Code::ArrowUp),
        ("meta+alt+ArrowLeft", Code::ArrowLeft),
    ] {
        assert_eq!(ks(spelling).code, expected, "{spelling}");
    }
}

#[test]
fn a_keystroke_round_trips_through_its_own_spelling() {
    for s in [
        "meta+KeyK",
        "meta+shift+KeyD",
        "meta+alt+ArrowLeft",
        "meta+shift+alt+control+KeyA",
        "Escape",
        "F12",
        "meta+Comma",
        "shift+PageDown",
    ] {
        let k = ks(s);
        assert_eq!(k.to_string(), s, "{s} did not round-trip");
        assert_eq!(ks(&k.to_string()), k);
    }
}

#[test]
fn everything_that_is_not_canonical_is_rejected() {
    use KeystrokeError as E;
    /// A spelling, and the error it has to produce. Pairing them means a case
    /// that fails for the wrong reason is a failure, not a pass.
    type Case = (&'static str, fn(&KeystrokeError) -> bool);
    let cases: &[Case] = &[
        ("", |e| matches!(e, E::MissingKey)),
        ("meta+", |e| matches!(e, E::MissingKey)),
        ("meta+shift", |e| matches!(e, E::MissingKey)),
        ("+KeyA", |e| matches!(e, E::EmptyComponent)),
        ("meta++KeyA", |e| matches!(e, E::EmptyComponent)),
        ("cmd+KeyA", |e| matches!(e, E::UnknownModifier { .. })),
        ("super+KeyA", |e| matches!(e, E::UnknownModifier { .. })),
        ("Meta+KeyA", |e| matches!(e, E::UnknownModifier { .. })),
        ("meta+meta+KeyA", |e| {
            matches!(e, E::RepeatedModifier { .. })
        }),
        ("shift+meta+KeyA", |e| {
            matches!(e, E::ModifierOutOfOrder { .. })
        }),
        ("control+alt+KeyA", |e| {
            matches!(e, E::ModifierOutOfOrder { .. })
        }),
        ("meta+A", |e| matches!(e, E::UnknownKey { .. })),
        ("meta+keya", |e| matches!(e, E::UnknownKey { .. })),
        ("meta+nonsense", |e| matches!(e, E::UnknownKey { .. })),
        ("meta+ㅁ", |e| matches!(e, E::NonAscii)),
    ];
    for (bad, expected) in cases {
        let err = bad
            .parse::<Keystroke>()
            .map(|_| ())
            .expect_err(&format!("{bad:?} should not parse"));
        assert!(expected(&err), "{bad:?} rejected wrongly: {err:?}");
        assert!(!err.to_string().is_empty());
    }
}

/// `cmd` is rejected on purpose: it names one platform's key, and this crate
/// makes no platform choice. `META` is Command on macOS and the Windows key
/// elsewhere; which one an application treats as primary is the application's
/// decision.
#[test]
fn the_syntax_names_no_platform() {
    assert!("cmd+KeyA".parse::<Keystroke>().is_err());
    assert!("win+KeyA".parse::<Keystroke>().is_err());
    assert!("option+KeyA".parse::<Keystroke>().is_err());
    assert!("meta+KeyA".parse::<Keystroke>().is_ok());
}

#[test]
fn one_physical_keystroke_is_one_value() {
    assert_eq!(ks("meta+shift+KeyK"), ks("meta+shift+KeyK"));
    assert_ne!(ks("meta+KeyK"), ks("meta+shift+KeyK"));
    assert_ne!(ks("meta+KeyK"), ks("control+KeyK"));
    assert_ne!(ks("F1"), ks("F2"));
    // The letter F is not the function key F1.
    assert_ne!(ks("KeyF"), ks("F1"));

    // Usable as a map key, which is how conflict detection groups.
    let mut set = HashSet::new();
    assert!(set.insert(ks("meta+KeyK")));
    assert!(!set.insert(ks("meta+KeyK")));
}

/// The property that makes a keymap file possible: a binding loaded at runtime
/// owns its strings, a built-in one borrows literals, and they are one type. If
/// this needed a leak or an intern, "a user's file is more rows in the same
/// table" would be false -- and that claim is why bindings are separate from
/// whatever describes an action.
#[test]
fn a_binding_can_be_built_from_owned_strings() {
    let owned = Binding {
        action: Cow::Owned(String::from("term.paste")),
        key: Cow::Owned(String::from("meta+shift+KeyV")),
        when: Some(Cow::Owned(String::from("pane_live"))),
    };
    assert!(matches!(owned.action, Cow::Owned(_)));
    ks(&owned.key);

    // `Cow::Borrowed` is const-constructible, so a default keymap is a constant.
    const BORROWED: Binding = Binding {
        action: Cow::Borrowed("term.paste"),
        key: Cow::Borrowed("meta+shift+KeyV"),
        when: Some(Cow::Borrowed("pane_live")),
    };
    assert_eq!(owned, BORROWED);
}

/// The crate does not interpret a condition.
#[test]
fn a_condition_is_opaque() {
    let b = Binding {
        action: Cow::Borrowed("app.quit"),
        key: Cow::Borrowed("meta+KeyQ"),
        when: None,
    };
    assert!(b.when.is_none());
    let guarded = Binding {
        when: Some(Cow::Borrowed("!palette_open && x==1")),
        ..b
    };
    assert_eq!(guarded.when.as_deref(), Some("!palette_open && x==1"));
}

/// `Modifiers` defines fourteen flags; this syntax names four, and so does
/// `Display`. A hand-built value carrying anything else prints without it. Not
/// reachable from an event, since winit reports exactly four modifier states.
#[test]
fn a_hand_built_keystroke_with_an_exotic_modifier_does_not_round_trip() {
    let exotic = Keystroke {
        modifiers: Modifiers::META | Modifiers::FN,
        code: Code::KeyA,
    };
    assert_eq!(exotic.to_string(), "meta+KeyA", "FN is not in the syntax");

    // So it does not survive a round-trip, and the parse is not wrong -- it is
    // the only value the syntax can name.
    let reparsed: Keystroke = exotic.to_string().parse().unwrap();
    assert_ne!(reparsed, exotic);
    assert_eq!(reparsed.modifiers, Modifiers::META);

    // Everything the syntax *can* name still round-trips, which is the property
    // that matters and is asserted separately above.
    let ordinary = Keystroke {
        modifiers: Modifiers::META,
        code: Code::KeyA,
    };
    assert_eq!(ordinary.to_string().parse::<Keystroke>().unwrap(), ordinary);
}

/// The shorthand reaches the same value as the long form, which is the only
/// reason it is allowed to exist: it is an abbreviation, not a second key.
///
/// Case is **not** part of it: `A` is rejected alongside `Meta` and `keya` in
/// `everything_that_is_not_canonical_is_rejected`.
#[test]
fn the_shorthand_is_an_abbreviation_of_the_long_form() {
    assert_eq!(ks("a"), ks("KeyA"));
    assert_eq!(ks("meta+a"), ks("meta+KeyA"));
    assert_eq!(ks("meta+1"), ks("meta+Digit1"));
    // And the canonical spelling it prints back is the long one.
    assert_eq!(ks("meta+a").to_string(), "meta+KeyA");
}

/// The reviewer asked whether function keys are reachable. They are, and by the
/// long form only -- there is no shorthand to collide with.
#[test]
fn function_keys_need_no_table_of_ours() {
    for (spelling, expected) in
        [("F1", Code::F1), ("F12", Code::F12), ("F24", Code::F24)]
    {
        assert_eq!(ks(spelling).code, expected);
    }
    assert_eq!(ks("meta+F2").to_string(), "meta+F2");
}
