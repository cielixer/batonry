//! Looking a keystroke up in an assembled keymap.
//!
//! `keymap.rs` beside this file tests `DEFAULT_KEYMAP` as data -- that the rows
//! are well formed. This one tests the `Keymap` those rows assemble into: what
//! a keystroke resolves to, and what it deliberately does not.

use std::borrow::Cow;

use baton_action::{
    BUILT_IN, Binding, Flags, Keymap, Keystroke, assemble, combine, merge,
};

/// Every flag except `Flags::PANE_FOCUSED`. Named for what it leaves out, because a
/// test that wants all of them has to say so.
const EVERY_FLAG_BUT_FOCUS: [Flags; 16] = [
    Flags::PANE_LIVE,
    Flags::PANE_DISCONNECTED,
    Flags::HAS_SELECTION,
    Flags::SEARCH_OPEN,
    Flags::PALETTE_OPEN,
    Flags::DIALOG_OPEN,
    Flags::EDITING_TEXT,
    Flags::SIDEBAR_HOSTS,
    Flags::SIDEBAR_WORKSPACES,
    Flags::HOST_SELECTED,
    Flags::SCRATCH_ACTIVE,
    Flags::WORKSPACE_ACTIVE,
    Flags::HAS_JUMP,
    Flags::HAS_QUEUED_INPUT,
    Flags::DIALOG_HOSTKEY_NEW,
    Flags::DIALOG_HOSTKEY_CHANGED,
];

fn built_in() -> Keymap {
    assemble(baton_action::DEFAULT_KEYMAP, &merge(&[BUILT_IN]))
}

fn ctx(flags: &[Flags]) -> Flags {
    flags
        .iter()
        .fold(Flags::NONE, |set, flag| combine(set, *flag))
}

fn chord(s: &str) -> Keystroke {
    s.parse()
        .unwrap_or_else(|e| panic!("{s:?} is not a chord: {e}"))
}

fn bind(
    action: &'static str,
    key: &'static str,
    when: Option<&'static str>,
) -> Binding {
    Binding {
        action: Cow::Borrowed(action),
        key: Cow::Borrowed(key),
        when: when.map(Cow::Borrowed),
    }
}

/// Assembling the real table parses every chord and every condition, so an
/// unparseable one fails here rather than at a keypress. And nothing is
/// dropped: every binding is reachable through `lookup`.
///
/// Each guard in the built-in table is satisfied either by everything being
/// true or by nothing being -- `!palette_open` needs the latter and the rest
/// need the former -- so two contexts cover the table without a solver.
#[test]
fn every_binding_is_reachable_after_assembly() {
    let registry = merge(&[BUILT_IN]);
    let keymap = built_in();
    let all = combine(
        EVERY_FLAG_BUT_FOCUS
            .iter()
            .fold(Flags::NONE, |set, f| combine(set, *f)),
        Flags::PANE_FOCUSED,
    );

    for binding in baton_action::DEFAULT_KEYMAP {
        let chord = chord(&binding.key);
        let want = registry
            .resolve(&binding.action)
            .unwrap_or_else(|| panic!("{} is not registered", binding.action));

        let reached = keymap.lookup(chord, all) == Some(want)
            || keymap.lookup(chord, Flags::NONE) == Some(want);
        assert!(
            reached,
            "{} on {} is in the table but no context reaches it; its guard is \
             {:?}",
            binding.action, binding.key, binding.when
        );
    }
}

/// A condition that fails means the keystroke is not ours, which is the whole
/// point: `⌘C` without a selection has to reach the PTY as an interrupt.
#[test]
fn a_failing_condition_hands_the_keystroke_back() {
    let keymap = built_in();
    let registry = merge(&[BUILT_IN]);
    let copy = registry
        .resolve("term.copy")
        .expect("term.copy is registered");

    assert_eq!(
        keymap.lookup(
            chord("meta+KeyC"),
            ctx(&[Flags::PANE_FOCUSED, Flags::HAS_SELECTION])
        ),
        Some(copy)
    );
    assert_eq!(
        keymap.lookup(chord("meta+KeyC"), ctx(&[Flags::PANE_FOCUSED])),
        None,
        "without a selection this must fall through to the PTY"
    );
    assert_eq!(
        keymap.lookup(chord("meta+KeyC"), ctx(&[Flags::HAS_SELECTION])),
        None,
        "a selection the keyboard is not pointed at is not this copy's"
    );
}

/// A binding with no condition always matches.
#[test]
fn an_unconditional_binding_always_matches() {
    let keymap = built_in();
    let registry = merge(&[BUILT_IN]);
    let quit = registry
        .resolve("app.quit")
        .expect("app.quit is registered");

    for context in [
        Flags::NONE,
        ctx(&[Flags::EDITING_TEXT, Flags::PALETTE_OPEN]),
    ] {
        assert_eq!(keymap.lookup(chord("meta+KeyQ"), context), Some(quit));
    }
}

/// A chord nobody bound is not ours.
#[test]
fn an_unbound_chord_resolves_to_nothing() {
    let keymap = built_in();
    for unbound in ["meta+KeyZ", "control+KeyC", "F5", "KeyA"] {
        assert_eq!(
            keymap.lookup(chord(unbound), Flags::NONE),
            None,
            "{unbound} is not in the table and must not resolve"
        );
    }
}

/// **The global rule.** While a text field has focus, a bare key that inserts a
/// character is suppressed -- typing `q` into a field must not quit the app.
///
/// No such binding exists in the real table, so this builds one. Asserting
/// against `DEFAULT_KEYMAP` would prove nothing.
#[test]
fn editing_text_kills_a_bare_key_that_produces_text() {
    let registry = merge(&[BUILT_IN]);
    let keymap = assemble(
        &[
            bind("app.quit", "KeyQ", None),
            bind("term.clear", "Digit1", None),
            bind("term.paste", "Space", None),
        ],
        &registry,
    );

    for (key, id) in [
        ("KeyQ", "app.quit"),
        ("Digit1", "term.clear"),
        ("Space", "term.paste"),
    ] {
        let action = registry.resolve(id).unwrap();
        assert_eq!(
            keymap.lookup(chord(key), Flags::NONE),
            Some(action),
            "{key} must work when no field has focus"
        );
        assert_eq!(
            keymap.lookup(chord(key), ctx(&[Flags::EDITING_TEXT])),
            None,
            "{key} must reach the field instead of firing {id}"
        );
    }
}

/// The same rule leaves the keys a palette needs alone, which is why it is
/// about producing text rather than about being a single key.
///
/// The palette's own search field sets `editing_text`, and the built-in table
/// binds `Escape`, `Enter` and the arrows bare. Suppressing every bare key
/// would make the palette impossible to close.
#[test]
fn editing_text_leaves_the_keys_a_palette_needs_alive() {
    let keymap = built_in();
    let registry = merge(&[BUILT_IN]);
    let editing = ctx(&[Flags::EDITING_TEXT, Flags::PALETTE_OPEN]);

    for (key, id) in [
        ("Escape", "palette.close"),
        ("Enter", "palette.confirm"),
        ("ArrowDown", "palette.next"),
        ("ArrowUp", "palette.prev"),
    ] {
        let action = registry.resolve(id).unwrap();
        assert_eq!(
            keymap.lookup(chord(key), editing),
            Some(action),
            "{key} died while the palette's own field had focus"
        );
    }
}

/// Pasting while the palette's field has focus goes to the field, not the pane.
///
/// This was a real defect. `term.paste` was guarded on `pane_live` alone, which
/// is a property of the pane's connection and not of where the keyboard is
/// pointed, so `⌘V` typed into the palette resolved to `term.paste` and would
/// have pasted into the terminal behind it. The global rule does not cover it
/// -- it is about bare keys -- so the guard is what had to be right.
#[test]
fn pasting_into_the_palette_does_not_reach_the_pane() {
    let keymap = built_in();
    let registry = merge(&[BUILT_IN]);
    let paste = registry.resolve("term.paste").unwrap();

    // Typing in the palette, with a live pane behind it.
    let typing =
        ctx(&[Flags::EDITING_TEXT, Flags::PALETTE_OPEN, Flags::PANE_LIVE]);
    assert_eq!(
        keymap.lookup(chord("meta+KeyV"), typing),
        None,
        "the paste belongs to the field that has focus"
    );

    // With the pane focused instead, it is the pane's paste again.
    let at_the_pane = ctx(&[Flags::PANE_FOCUSED, Flags::PANE_LIVE]);
    assert_eq!(keymap.lookup(chord("meta+KeyV"), at_the_pane), Some(paste));
}

/// The keys a text field consumes are suppressed, not just the ones that insert
/// a character: Backspace has to reach the field, and so does the caret.
#[test]
fn editing_text_also_yields_the_editing_keys() {
    let registry = merge(&[BUILT_IN]);
    let keymap = assemble(
        &[
            bind("app.quit", "Backspace", None),
            bind("term.clear", "ArrowLeft", None),
            bind("term.paste", "Home", None),
        ],
        &registry,
    );

    for key in ["Backspace", "ArrowLeft", "Home"] {
        assert!(
            keymap.lookup(chord(key), Flags::NONE).is_some(),
            "{key} must work when no field has focus"
        );
        assert_eq!(
            keymap.lookup(chord(key), ctx(&[Flags::EDITING_TEXT])),
            None,
            "{key} must reach the field"
        );
    }
}

/// **No terminal action fires without the pane focused.** The whole table, not
/// one binding at a time.
///
/// This is the invariant the review found broken twice, once for paste and once
/// for copy, because both were guarded on a property of the *pane* rather than
/// on where the keyboard is pointed: `pane_live` says the connection is up and
/// `has_selection` says the terminal has one, and neither is false while
/// somebody types in the palette. Checking bindings one at a time is how the
/// second one survived the fix for the first.
#[test]
fn no_terminal_action_fires_without_the_pane_focused() {
    let registry = merge(&[BUILT_IN]);
    let keymap = built_in();

    // Everything true except focus. A guard that lets a terminal action through
    // here is a guard that does not mention focus.
    let everything_but_focus = EVERY_FLAG_BUT_FOCUS
        .iter()
        .fold(Flags::NONE, |set, f| combine(set, *f));

    for binding in baton_action::DEFAULT_KEYMAP {
        let key = chord(&binding.key);
        let Some(id) = keymap.lookup(key, everything_but_focus) else {
            continue;
        };
        let action = registry.get(id).expect("the id came from this registry");
        assert!(
            !action.id.starts_with("term."),
            "{} resolves to {} with no pane focused; its guard {:?} is about \
             the pane's state rather than about where the keyboard is",
            binding.key,
            action.id,
            binding.when
        );
    }
}

/// A modifier takes the key back out of the rule's reach: `⌘Q` quits while
/// typing, because it was never going to reach the field.
#[test]
fn a_modifier_exempts_a_key_from_the_rule() {
    let keymap = built_in();
    let registry = merge(&[BUILT_IN]);
    let quit = registry.resolve("app.quit").unwrap();

    assert_eq!(
        keymap.lookup(chord("meta+KeyQ"), ctx(&[Flags::EDITING_TEXT])),
        Some(quit)
    );
}

/// A wrong table is a panic, not a `Result`: every source is a compile-time
/// constant, so there is nothing a caller could do with the news.
#[test]
#[should_panic(expected = "action is not registered")]
fn a_binding_naming_a_missing_action_panics() {
    assemble(
        &[bind("nope.missing", "meta+KeyZ", None)],
        &merge(&[BUILT_IN]),
    );
}

#[test]
#[should_panic(expected = "invalid chord")]
fn an_unparseable_chord_panics() {
    assemble(&[bind("app.quit", "cmd+Q", None)], &merge(&[BUILT_IN]));
}

#[test]
#[should_panic(expected = "invalid when")]
fn an_unparseable_condition_panics() {
    assemble(
        &[bind("app.quit", "meta+KeyQ", Some("pane_focussed"))],
        &merge(&[BUILT_IN]),
    );
}
