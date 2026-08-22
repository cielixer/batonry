//! The action table and the merge that assembles it.
//!
//! What these assert, and why each one exists:
//!
//! - The stage-1 inventory is **exactly** ten named ids -- not "about ten",
//!   which was unverifiable and let any count claim completion.
//! - Ids obey the rules that can actually be checked. The specification also
//!   says the middle segment is a verb; that is **not** checkable and is not
//!   checked here -- see `id_grammar_is_only_what_can_be_checked`.
//! - A duplicate id names **both** sources and positions, which is the whole
//!   reason sources are named instead of anonymous slices.
//! - Merging is exercised with two sources. Merging one is not a merge.
//! - String lookup does not scan, asserted at the source level because a test
//!   cannot observe an asymptote.

use std::collections::HashSet;

use baton_action::{
    ACTIONS, ActionMeta, ActionSource, ArgKind, IssueSites, RegistryError,
    STAGE1_SOURCE, try_merge,
};

/// The ten ids stage 1 implements. Written out rather than derived from
/// `ACTIONS`, because a test that reads the value it is checking asserts
/// nothing.
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
    let got: Vec<&str> = ACTIONS.iter().map(|a| a.id).collect();
    assert_eq!(
        got, STAGE1_IDS,
        "the stage-1 inventory is fixed at ten ids in a fixed order. Adding an \
         action here means it is implemented and reachable -- an action nothing \
         executes is a palette entry that does nothing"
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

/// The palette shows what carries `PALETTE`, and the difference between "in the
/// registry" and "in the palette" is the point of the flags. If every action
/// carried `PALETTE` the field would be decoration.
#[test]
fn issue_sites_distinguish_registry_membership_from_palette_visibility() {
    let palette = ACTIONS
        .iter()
        .filter(|a| a.issue.contains(IssueSites::PALETTE))
        .count();
    assert!(
        palette > 0 && palette < ACTIONS.len(),
        "{palette} of {} actions are palette-visible; if it were all or none, \
         IssueSites would not be carrying any information",
        ACTIONS.len()
    );
    // The palette-open action itself must never be palette-visible: reaching it
    // from the palette would require the palette to already be open.
    let open = ACTIONS.iter().find(|a| a.id == "palette.open").unwrap();
    assert!(!open.issue.contains(IssueSites::PALETTE));
}

/// The documented grammar says the middle segment is a verb and noun forms are
/// forbidden. **That cannot be checked**, and this test records why rather than
/// leaving a future reader to rediscover it: the canonical table breaks it in
/// about eleven rows -- `sidebar.mode.hosts` has no verb at all,
/// `term.search.open` and `term.font.increase` and `conn.input.flush` put a
/// noun in the middle, and `term.scroll.line.up` has four segments. The rule
/// stays as naming guidance. These are the parts that are true.
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
        assert!(seen.insert(a.id), "{} is registered twice", a.id);
    }
}

// A second source, standing in for a crate that will contribute actions later.
// It also carries the composite argument shape, which no stage-1 action uses --
// so the shape is exercised here rather than being merely declared.
const FIXTURE: &[ActionMeta] = &[
    ActionMeta {
        id: "host.edit",
        label: "Edit Host…",
        issue: IssueSites::PALETTE.union(IssueSites::MENU),
        arg: ArgKind::HostTab,
    },
    ActionMeta {
        id: "pane.split.vertical",
        label: "Split Vertically",
        issue: IssueSites::PALETTE.union(IssueSites::MENU),
        arg: ArgKind::Pane,
    },
];

const FIXTURE_SOURCE: ActionSource = ActionSource {
    name: "fixture",
    actions: FIXTURE,
};

#[test]
fn merging_two_sources_keeps_both_and_indexes_both() {
    let registry = try_merge(&[STAGE1_SOURCE, FIXTURE_SOURCE])
        .expect("two disjoint sources merge");

    assert_eq!(registry.len(), ACTIONS.len() + FIXTURE.len());

    // Every id from both sources resolves, and resolving round-trips.
    for meta in ACTIONS.iter().chain(FIXTURE) {
        let id = registry
            .id(meta.id)
            .unwrap_or_else(|| panic!("{} did not resolve", meta.id));
        assert_eq!(registry.get(id).unwrap().id, meta.id);
    }

    // Order is source order, so an ActionId stays meaningful across a boot
    // with the same source list.
    assert_eq!(registry.actions()[0].id, ACTIONS[0].id);
    assert_eq!(registry.actions()[ACTIONS.len()].id, FIXTURE[0].id);

    // The composite argument shape survives the merge.
    let edit = registry.id("host.edit").unwrap();
    assert_eq!(registry.get(edit).unwrap().arg, ArgKind::HostTab);

    assert!(registry.id("nope.missing").is_none());
}

#[test]
fn merging_one_source_is_not_a_merge_but_still_works() {
    let registry = try_merge(&[STAGE1_SOURCE]).expect("single source");
    assert_eq!(registry.len(), ACTIONS.len());
    assert!(!registry.is_empty());
    assert_eq!(registry.iter().count(), ACTIONS.len());
}

#[test]
fn a_duplicate_id_names_both_sources_and_both_positions() {
    // `term.copy` is at index 6 of the stage-1 table; put it at index 1 of a
    // colliding source so the two positions differ and cannot be confused.
    const COLLIDING: &[ActionMeta] = &[
        ActionMeta {
            id: "pane.close",
            label: "Close Pane",
            issue: IssueSites::PALETTE,
            arg: ArgKind::Pane,
        },
        ActionMeta {
            id: "term.copy",
            label: "Copy (again)",
            issue: IssueSites::PALETTE,
            arg: ArgKind::None,
        },
    ];
    let other = ActionSource {
        name: "other-crate",
        actions: COLLIDING,
    };

    let err = try_merge(&[STAGE1_SOURCE, other])
        .expect_err("a duplicate id must not merge");

    match err {
        RegistryError::DuplicateId {
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

    // The message has to be readable on its own: it is what someone sees when
    // the app refuses to boot.
    let text = try_merge(&[STAGE1_SOURCE, other]).unwrap_err().to_string();
    for needle in ["term.copy", "baton-action", "other-crate", "6", "1"] {
        assert!(
            text.contains(needle),
            "the error message omits {needle:?}: {text}"
        );
    }
}

/// A duplicate **within one source** is the same failure, and it is the likelier
/// one -- a copy-pasted row in a growing table.
#[test]
fn a_duplicate_inside_one_source_is_also_rejected() {
    const TWICE: &[ActionMeta] = &[
        ActionMeta {
            id: "app.quit",
            label: "Quit",
            issue: IssueSites::PALETTE,
            arg: ArgKind::None,
        },
        ActionMeta {
            id: "app.quit",
            label: "Quit, pasted",
            issue: IssueSites::PALETTE,
            arg: ArgKind::None,
        },
    ];
    let err = try_merge(&[ActionSource {
        name: "self-colliding",
        actions: TWICE,
    }])
    .expect_err("a source that collides with itself must not merge");
    assert!(matches!(
        err,
        RegistryError::DuplicateId {
            first_position: 0,
            second_position: 1,
            ..
        }
    ));
}

/// A test cannot observe an asymptote, so the structural property is asserted
/// at the source, the same way `baton-term` keeps `blocking_send` out of its
/// read path. A linear scan here would be invisible at eighty-two actions and
/// wrong by the time it mattered.
#[test]
fn string_lookup_does_not_scan_the_action_slice() {
    let src = include_str!("../src/registry.rs");
    let body = src
        .split("pub fn id(&self")
        .nth(1)
        .and_then(|rest| rest.split_once('}'))
        .map(|(body, _)| body)
        .expect("Registry::id must exist with that signature");

    assert!(
        body.contains("by_id"),
        "Registry::id no longer goes through the index: {body}"
    );
    for scan in [".iter()", ".position(", "for "] {
        assert!(
            !body.contains(scan),
            "Registry::id contains {scan:?}, which means it scans: {body}"
        );
    }
}
