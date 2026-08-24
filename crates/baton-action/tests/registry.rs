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
    ACTIONS, Action, ArgKind, BUILT_IN, KEY_ONLY, MergeError, PALETTE, Source,
    reachable_from, try_merge,
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
            ArgKind::None,
            "{} takes an argument, but nothing in stage 1 can supply one",
            a.id
        );
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
/// forbidden. **That cannot be checked**, and this records why rather than
/// leaving a future reader to rediscover it: the canonical table breaks it in
/// about eleven rows -- `sidebar.mode.hosts` has no verb at all,
/// `term.search.open` and `term.font.increase` and `conn.input.flush` put a noun
/// in the middle, and `term.scroll.line.up` has four segments. The rule stays as
/// naming guidance. These are the parts that are true.
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
    let r = try_merge(&[BUILT_IN]).expect("the built-in table merges");
    assert_eq!(r.len(), ACTIONS.len());
    assert!(!r.is_empty());

    // Ids are handed out in contribution order. Asserting that through the
    // public surface rather than through the integer: the id a name resolves to
    // has to address the row at that name's position, and the ten names are
    // distinct, so nothing but a dense in-order assignment satisfies it.
    for (position, row) in ACTIONS.iter().enumerate() {
        let id = r
            .resolve(&row.id)
            .unwrap_or_else(|| panic!("{} did not resolve", row.id));
        assert_eq!(r.get(id).unwrap(), row);
        assert_eq!(&r.rows()[position], row);
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
        rows: Cow::Owned(vec![
            Action {
                id: Cow::Owned(String::from("plugin.greet")),
                label: Cow::Owned(String::from("Say Hello")),
                channels: PALETTE,
                arg: ArgKind::None,
            },
            Action {
                id: Cow::Owned(String::from("host.edit")),
                label: Cow::Owned(String::from("Edit Host…")),
                channels: PALETTE,
                arg: ArgKind::HostTab,
            },
        ]),
    };

    let r = try_merge(&[BUILT_IN, loaded]).expect("disjoint sources merge");
    assert_eq!(r.len(), ACTIONS.len() + 2);

    // Built-in rows keep their positions, so an index issued before the loaded
    // table existed still means the same thing.
    assert_eq!(r.rows()[0].id, ACTIONS[0].id);

    let greet = r.resolve("plugin.greet").expect("the loaded row resolves");
    assert_eq!(r.get(greet).unwrap().label, "Say Hello");
    let edit = r.resolve("host.edit").unwrap();
    assert_eq!(r.get(edit).unwrap().arg, ArgKind::HostTab);
}

/// **Merging adds; it never redefines.** A loaded table cannot take over a
/// built-in name -- if it could, a file would silently change what an action
/// does while the palette still showed the built-in label.
#[test]
fn a_runtime_table_cannot_redefine_a_built_in_action() {
    let hostile = Source {
        name: Cow::Owned(String::from("keymap.toml")),
        rows: Cow::Owned(vec![Action {
            id: Cow::Borrowed("term.copy"),
            label: Cow::Owned(String::from("Copy, but different")),
            channels: PALETTE,
            arg: ArgKind::None,
        }]),
    };
    let err = try_merge(&[BUILT_IN, hostile])
        .expect_err("a loaded table must not redefine a built-in action");
    match err {
        MergeError::DuplicateId {
            id, second_source, ..
        } => {
            assert_eq!(id, "term.copy");
            assert_eq!(second_source, "keymap.toml");
        },
        other => panic!("expected DuplicateId, got {other:?}"),
    }
}

#[test]
fn a_duplicate_names_both_sources_and_both_positions() {
    // `term.copy` is at index 6 of the built-in table; put it at index 1 of a
    // colliding source so the positions differ and cannot be confused.
    let other = Source {
        name: Cow::Borrowed("other-crate"),
        rows: Cow::Owned(vec![
            Action {
                id: Cow::Borrowed("pane.close"),
                label: Cow::Borrowed("Close Pane"),
                channels: PALETTE,
                arg: ArgKind::Pane,
            },
            Action {
                id: Cow::Borrowed("term.copy"),
                label: Cow::Borrowed("Copy (again)"),
                channels: PALETTE,
                arg: ArgKind::None,
            },
        ]),
    };

    let err = try_merge(&[BUILT_IN, other.clone()])
        .expect_err("a duplicate must not merge");
    match err {
        MergeError::DuplicateId {
            id,
            first_source,
            first_position,
            second_source,
            second_position,
        } => {
            assert_eq!(id, "term.copy");
            assert_eq!(first_source, "baton-action");
            assert_eq!(first_position, 6);
            assert_eq!(second_source, "other-crate");
            assert_eq!(second_position, 1);
        },
        other => panic!("expected DuplicateId, got {other:?}"),
    }

    // The message stands alone: it is what someone sees when boot refuses.
    let text = try_merge(&[BUILT_IN, other]).unwrap_err().to_string();
    for needle in ["term.copy", "baton-action", "other-crate", "6", "1"] {
        assert!(text.contains(needle), "message omits {needle:?}: {text}");
    }
}

/// A duplicate **within one source** is the same failure and the likelier one:
/// a copy-pasted row in a growing table.
#[test]
fn a_duplicate_inside_one_source_is_also_rejected() {
    let twice = Source {
        name: Cow::Borrowed("self-colliding"),
        rows: Cow::Owned(vec![
            Action {
                id: Cow::Borrowed("app.quit"),
                label: Cow::Borrowed("Quit"),
                channels: PALETTE,
                arg: ArgKind::None,
            },
            Action {
                id: Cow::Borrowed("app.quit"),
                label: Cow::Borrowed("Quit, pasted"),
                channels: PALETTE,
                arg: ArgKind::None,
            },
        ]),
    };
    let err = try_merge(&[twice])
        .expect_err("a source that collides with itself must not merge");
    assert!(matches!(
        err,
        MergeError::DuplicateId {
            first_position: 0,
            second_position: 1,
            ..
        }
    ));
}

#[test]
fn an_empty_source_list_produces_an_empty_registry() {
    let r = try_merge(&[]).unwrap();
    assert!(r.is_empty());
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

/// `KEY_ONLY` is the empty set, and every set contains the empty set. So it is a
/// value to build a row with and never one to ask about -- a containment test
/// against it answers `true` for everything, including an action that carries
/// `PALETTE`.
///
/// Pinned because the reading is counter-intuitive and the obvious "fix" would
/// be to special-case the empty set inside `reachable_from`, which would make it
/// something other than containment.
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
