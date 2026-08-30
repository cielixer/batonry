//! The built-in keymap, checked against the table it points at.
//!
//! The specification's rule is that a key **not** in this table reaches the
//! terminal. So this table *is* the interception set, and every assertion here
//! is really about what the app does and does not take away from the shell.

use std::collections::HashSet;

use baton_action::{ACTIONS, BUILT_IN, DEFAULT_KEYMAP, Keystroke, merge};

#[test]
fn every_binding_names_an_action_that_exists() {
    let registry = merge(&[BUILT_IN]);
    for b in DEFAULT_KEYMAP {
        assert!(
            registry.resolve(&b.action).is_some(),
            "{:?} is bound to {:?}, which is not registered. A dangling \
             binding is a key that silently does nothing",
            b.key,
            b.action
        );
    }
}

/// The parse has to happen somewhere, and a test is strictly better than
/// discovering it as a dead key at runtime.
#[test]
fn every_key_parses() {
    for b in DEFAULT_KEYMAP {
        b.key.parse::<Keystroke>().unwrap_or_else(|e| {
            panic!("{} is bound to unparseable key {:?}: {e}", b.action, b.key)
        });
    }
}

#[test]
fn every_action_stage_one_implements_is_reachable_somehow() {
    let bound: HashSet<&str> =
        DEFAULT_KEYMAP.iter().map(|b| b.action.as_ref()).collect();
    for a in ACTIONS {
        assert!(
            bound.contains(a.id.as_ref()),
            "{} is registered but nothing reaches it -- neither a key here nor \
             a channel would make it usable",
            a.id
        );
    }
}

/// A chord may repeat when the conditions are disjoint -- deciding that is the
/// conflict check's job. What must not happen is the same chord with the same
/// guard, because one of the two could never fire.
#[test]
fn no_two_bindings_share_a_chord_and_a_guard() {
    let mut seen = HashSet::new();
    for b in DEFAULT_KEYMAP {
        let chord = b.key.parse::<Keystroke>().unwrap();
        assert!(
            seen.insert((chord, b.when.clone())),
            "{:?} appears twice with the same guard {:?}; one of them can \
             never fire",
            b.key,
            b.when
        );
    }
}

/// Unmodified keys are the dangerous ones: they are what a person types into a
/// terminal. Only the palette's own navigation may claim any, and only while the
/// palette is open.
#[test]
fn every_unmodified_key_is_guarded() {
    let bare: HashSet<&str> = DEFAULT_KEYMAP
        .iter()
        .filter(|b| !b.key.contains('+'))
        .map(|b| b.key.as_ref())
        .collect();
    assert_eq!(
        bare,
        HashSet::from(["Escape", "Enter", "ArrowUp", "ArrowDown"]),
        "an unmodified key was added to the keymap. Each one stops reaching the \
         terminal, so the set is pinned deliberately"
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

/// `Ctrl` is never intercepted. That is the single biggest thing the macOS-only
/// keymap buys: every `Ctrl+<letter>` a shell or an editor wants goes straight
/// through, with no exception list to maintain.
#[test]
fn control_is_never_intercepted() {
    for b in DEFAULT_KEYMAP {
        assert!(
            !b.key.contains("control+"),
            "{:?} intercepts a Ctrl chord; those belong to the terminal",
            b.key
        );
    }
}
