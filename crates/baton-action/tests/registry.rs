//! The action table, and the merge that assembles it.
//!
//! What these pin, and why each one exists:
//!
//! - The stage-1 inventory is **exactly** ten named ids. "About ten" was
//!   unverifiable and let any count claim completion.
//! - Ids obey the rules that can actually be checked. The specification also
//!   says the middle segment is a verb; that is **not** checkable and is not
//!   checked -- see `id_grammar_is_only_what_can_be_checked`.
//! - A duplicate names **both** sources and positions, which is the whole reason
//!   a source carries a name.
//! - **A runtime table is the same kind of thing as the built-in one**, which is
//!   what makes a rebind or a loaded row possible without a redesign.
//! - Name lookup does not scan, asserted at the source because a test cannot
//!   observe an asymptote.

use std::borrow::Cow;
use std::collections::HashSet;

use baton_action::{
    ACTIONS, Action, ArgShape, BUILT_IN, KEY_ONLY, PALETTE, Source, merge,
    reachable_from,
};

/// The ten ids stage 1 implements. Written out rather than derived from
/// `ACTIONS`, because a test that reads the value it is checking asserts nothing.
const STAGE1_IDS: [&str; 10] = [
    "app.quit",
    "palette.open",
    "palette.close",
    "palette.confirm",
    "palette.next",
    "palette.prev",
    "term.copy",
    "term.paste",
    "term.select_all",
    "term.clear",
];

/// Every domain the specification's table uses. Sixteen, not the eleven the
/// document listed before this was checked.
const DOMAINS: [&str; 16] = [
    "app",
    "palette",
    "sidebar",
    "host",
    "group",
    "workspace",
    "tab",
    "pane",
    "term",
    "snippet",
    "conn",
    "hostkey",
    "key",
    "dialog",
    "help",
    "restore",
];

#[test]
fn stage_one_registers_exactly_ten_actions() {
    let got: Vec<&str> = ACTIONS.iter().map(|a| a.id.as_ref()).collect();
    assert_eq!(
        got, STAGE1_IDS,
        "the stage-1 inventory is fixed at ten ids in a fixed order. Adding one \
         means it is implemented and reachable -- an action nothing executes is \
         a palette entry that does nothing"
    );
}

#[test]
fn every_action_has_a_label_and_stage_one_takes_no_arguments() {
    for a in ACTIONS {
        assert!(!a.label.is_empty(), "{} has no label", a.id);
        assert_eq!(
            a.arg,
            ArgShape::None,
            "{} takes an argument, but nothing in stage 1 can supply one",
            a.id
        );
    }
}

/// `reachable` is the palette, and what it hands back is dispatchable.
///
/// The id matters as much as the row. A palette entry has to turn into a message,
/// and the index accessor is private, so an iterator yielding `&Action` alone
/// would force a caller to `resolve` the id string it had just read.
#[test]
fn reachable_carries_ids_a_caller_can_dispatch() {
    let r = merge(&[BUILT_IN]);

    let by_hand: Vec<&str> = ACTIONS
        .iter()
        .filter(|a| reachable_from(a.channels, PALETTE))
        .map(|a| a.id.as_ref())
        .collect();
    let by_registry: Vec<&str> =
        r.reachable(PALETTE).map(|(_, a)| a.id.as_ref()).collect();
    assert_eq!(by_registry, by_hand, "reachable() is not the same filter");
    assert!(!by_hand.is_empty(), "the fixture would prove nothing");

    // Every id it yields addresses the row it came with.
    for (id, action) in r.reachable(PALETTE) {
        assert_eq!(r.get(id), Some(action), "{} yielded a stale id", action.id);
        assert_eq!(r.resolve(&action.id), Some(id));
    }
}

/// `iter` walks every action once, in contribution order, ids included.
#[test]
fn iter_is_the_whole_table_in_order() {
    let r = merge(&[BUILT_IN]);
    assert_eq!(r.iter().count(), ACTIONS.len());
    for ((id, action), expected) in r.iter().zip(ACTIONS) {
        assert_eq!(action, expected);
        assert_eq!(r.get(id), Some(expected));
    }
}

/// The palette shows what carries `PALETTE`. If every action carried it, or none
/// did, the field would be decoration.
#[test]
fn channels_distinguish_registry_membership_from_palette_visibility() {
    let visible = ACTIONS
        .iter()
        .filter(|a| reachable_from(a.channels, PALETTE))
        .count();
    assert!(
        visible > 0 && visible < ACTIONS.len(),
        "{visible} of {} actions are palette-visible; all or none would mean \
         Channels carries no information",
        ACTIONS.len()
    );
    // The action that opens the palette must never be palette-visible: reaching
    // it from the palette would need the palette already open.
    let open = ACTIONS.iter().find(|a| a.id == "palette.open").unwrap();
    assert!(!reachable_from(open.channels, PALETTE));
}

/// The documented grammar says the middle segment is a verb and noun forms are
/// forbidden. **That cannot be checked:** the canonical table breaks it in about
/// eleven rows -- `sidebar.mode.hosts` has no verb at all, `term.search.open`
/// and `term.font.increase` and `conn.input.flush` put a noun in the middle, and
/// `term.scroll.line.up` has four segments. The rule stays naming guidance;
/// these are the parts that are true.
#[test]
fn id_grammar_is_only_what_can_be_checked() {
    let domains: HashSet<&str> = DOMAINS.into_iter().collect();
    let mut seen = HashSet::new();

    for a in ACTIONS {
        let segments: Vec<&str> = a.id.split('.').collect();
        assert!(
            (2..=4).contains(&segments.len()),
            "{}: ids have two to four segments, got {}",
            a.id,
            segments.len()
        );
        assert!(
            domains.contains(segments[0]),
            "{}: unknown domain {:?}. A new domain is a decision, not a typo",
            a.id,
            segments[0]
        );
        for s in &segments {
            assert!(
                !s.is_empty()
                    && s.starts_with(|c: char| c.is_ascii_lowercase())
                    && s.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{}: segment {:?} must match [a-z][a-z_]*",
                a.id,
                s
            );
        }
        assert!(seen.insert(a.id.as_ref()), "{} is registered twice", a.id);
    }
}

#[test]
fn the_built_in_source_registers_and_indexes_every_row() {
    let r = merge(&[BUILT_IN]);
    assert_eq!(r.count(), ACTIONS.len());
    assert!(r.count() > 0);

    // Ids are handed out in contribution order. Asserting that through the
    // public surface rather than through the integer: the id a name resolves to
    // has to address the row at that name's position, and the ten names are
    // distinct, so nothing but a dense in-order assignment satisfies it.
    for (position, row) in ACTIONS.iter().enumerate() {
        let id = r
            .resolve(&row.id)
            .unwrap_or_else(|| panic!("{} did not resolve", row.id));
        assert_eq!(r.get(id).unwrap(), row);
        assert_eq!(r.iter().nth(position).map(|(_, a)| a), Some(row));
    }
    assert!(r.resolve("nope.missing").is_none());
}

/// A table that arrives at runtime is the same kind of thing as the built-in
/// one. This is the property the whole `Cow` arrangement exists for: if it
/// needed a leak, an intern or a second type, then "a loaded table is more rows
/// in the same table" would be false.
#[test]
fn a_runtime_table_merges_beside_the_built_in_one() {
    let loaded = Source {
        name: Cow::Owned(String::from("keymap.toml")),
        actions: Cow::Owned(vec![
            Action {
                id: Cow::Owned(String::from("plugin.greet")),
                label: Cow::Owned(String::from("Say Hello")),
                channels: PALETTE,
                arg: ArgShape::None,
            },
            Action {
                id: Cow::Owned(String::from("host.edit")),
                label: Cow::Owned(String::from("Edit Host…")),
                channels: PALETTE,
                arg: ArgShape::HostTab,
            },
        ]),
    };

    let r = merge(&[BUILT_IN, loaded]);
    assert_eq!(r.count(), ACTIONS.len() + 2);

    // Built-in rows keep their positions, so an index issued before the loaded
    // table existed still means the same thing.
    assert_eq!(r.iter().next().unwrap().1.id, ACTIONS[0].id);

    let greet = r.resolve("plugin.greet").expect("the loaded row resolves");
    assert_eq!(r.get(greet).unwrap().label, "Say Hello");
    let edit = r.resolve("host.edit").unwrap();
    assert_eq!(r.get(edit).unwrap().arg, ArgShape::HostTab);
}

/// **Merging adds; it never redefines.** A loaded table cannot take over a
/// built-in name -- if it could, a file would silently change what an action
/// does while the palette still showed the built-in label.
#[test]
#[should_panic(expected = "duplicate action id \"term.copy\"")]
fn a_runtime_table_cannot_redefine_a_built_in_action() {
    let hostile = Source {
        name: Cow::Owned(String::from("keymap.toml")),
        actions: Cow::Owned(vec![Action {
            id: Cow::Borrowed("term.copy"),
            label: Cow::Owned(String::from("Copy, but different")),
            channels: PALETTE,
            arg: ArgShape::None,
        }]),
    };
    merge(&[BUILT_IN, hostile]);
}

#[test]
fn a_duplicate_names_both_sources_and_both_positions() {
    // `term.copy` is at index 6 of the built-in table; put it at index 1 of a
    // colliding source so the positions differ and cannot be confused.
    let other = Source {
        name: Cow::Borrowed("other-crate"),
        actions: Cow::Owned(vec![
            Action {
                id: Cow::Borrowed("pane.close"),
                label: Cow::Borrowed("Close Pane"),
                channels: PALETTE,
                arg: ArgShape::Pane,
            },
            Action {
                id: Cow::Borrowed("term.copy"),
                label: Cow::Borrowed("Copy (again)"),
                channels: PALETTE,
                arg: ArgShape::None,
            },
        ]),
    };

    // The message is the whole diagnostic, and it is what `Source::name` exists
    // for: it has to name both claimants and both positions, or someone reading
    // a crash has nowhere to go.
    let text = std::panic::catch_unwind(|| merge(&[BUILT_IN, other]))
        .expect_err("a duplicate must not merge");
    let text = text
        .downcast_ref::<String>()
        .expect("the panic carries a formatted message")
        .clone();
    for needle in [
        "term.copy",
        "source 0 (baton-action) index 6",
        "source 1 (other-crate) index 1",
    ] {
        assert!(text.contains(needle), "message omits {needle:?}: {text}");
    }
}

/// A source's name is a label, not a key, so two of them may be identical. The
/// message has to stay readable when they are -- otherwise a collision between
/// two files both called `keymap.toml` reads exactly like one file colliding
/// with itself.
#[test]
fn two_sources_sharing_a_name_are_still_told_apart() {
    let one = Source {
        name: Cow::Borrowed("keymap.toml"),
        actions: Cow::Owned(vec![Action {
            id: Cow::Borrowed("plugin.greet"),
            label: Cow::Borrowed("Greet"),
            channels: PALETTE,
            arg: ArgShape::None,
        }]),
    };
    let two = Source {
        name: Cow::Borrowed("keymap.toml"),
        actions: Cow::Owned(vec![Action {
            id: Cow::Borrowed("plugin.greet"),
            label: Cow::Borrowed("Greet, again"),
            channels: PALETTE,
            arg: ArgShape::None,
        }]),
    };

    let payload = std::panic::catch_unwind(|| merge(&[one, two]))
        .expect_err("two sources claiming one id must not merge");
    let text = payload
        .downcast_ref::<String>()
        .expect("the panic carries a formatted message")
        .clone();
    assert!(
        text.contains("source 0 (keymap.toml) index 0")
            && text.contains("source 1 (keymap.toml) index 0"),
        "the two sources are indistinguishable: {text}"
    );
}

/// A duplicate **within one source** is the same failure and the likelier one:
/// a copy-pasted row in a growing table.
#[test]
#[should_panic(expected = "source 0 (self-colliding) index 0 collides with \
                           source 0 (self-colliding) index 1")]
fn a_duplicate_inside_one_source_is_also_rejected() {
    let twice = Source {
        name: Cow::Borrowed("self-colliding"),
        actions: Cow::Owned(vec![
            Action {
                id: Cow::Borrowed("app.quit"),
                label: Cow::Borrowed("Quit"),
                channels: PALETTE,
                arg: ArgShape::None,
            },
            Action {
                id: Cow::Borrowed("app.quit"),
                label: Cow::Borrowed("Quit, pasted"),
                channels: PALETTE,
                arg: ArgShape::None,
            },
        ]),
    };
    merge(&[twice]);
}

#[test]
fn an_empty_source_list_produces_an_empty_registry() {
    let r = merge(&[]);
    assert!(r.count() == 0);
    assert!(r.resolve("anything").is_none());
}

/// A test cannot observe an asymptote, so the structural property is asserted at
/// the source, the same way `baton-term` keeps `blocking_send` out of its read
/// path. A scan here would be invisible at eighty rows and wrong by the time it
/// mattered.
#[test]
fn name_lookup_does_not_scan_the_row_slice() {
    let src = include_str!("../src/registry.rs");
    let body = src
        .split("pub fn resolve(&self")
        .nth(1)
        .and_then(|rest| rest.split_once('}'))
        .map(|(body, _)| body)
        .expect("Registry::resolve must exist with that signature");

    assert!(body.contains("by_id"), "resolve() left the index: {body}");
    for scan in [".iter()", ".position(", "for "] {
        assert!(!body.contains(scan), "resolve() contains {scan:?}: {body}");
    }
}

/// A containment test against `KEY_ONLY` answers `true` for everything,
/// `PALETTE` included, because it is the empty set.
///
/// Pinned so the obvious "fix" is not made: special-casing the empty set inside
/// `reachable_from` would make it something other than containment.
#[test]
fn the_empty_channel_set_is_a_value_and_not_a_query() {
    let key_only = ACTIONS
        .iter()
        .find(|a| a.id == "palette.open")
        .expect("palette.open is registered");
    let in_palette = ACTIONS
        .iter()
        .find(|a| a.id == "term.copy")
        .expect("term.copy is registered");

    assert_eq!(key_only.channels, KEY_ONLY);
    assert!(!reachable_from(key_only.channels, PALETTE));

    // Both answer `true`, which is why the question has to be asked with `==`.
    assert!(reachable_from(key_only.channels, KEY_ONLY));
    assert!(reachable_from(in_palette.channels, KEY_ONLY));
    assert_ne!(in_palette.channels, KEY_ONLY);
}
