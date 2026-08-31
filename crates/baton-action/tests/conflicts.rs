//! The key-conflict check: two actions must never both fire on one keystroke.
//!
//! `keymap.rs` beside this tests `DEFAULT_KEYMAP` as data and `lookup.rs`
//! tests what a keystroke resolves to; this file tests the checker that keeps
//! the table sound -- and holds the assertion that *is* the CI check (#12).

use std::borrow::Cow;
use std::str::FromStr;

use baton_action::{
    Action, ArgShape, BUILT_IN, Binding, Channels, Flags, Predicate, Registry,
    Source, assemble, holds, merge, satisfiable_together,
};

mod common;

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

fn clause(s: &str) -> Predicate {
    Predicate::from_str(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
}

/// The built-in registry plus the two actions the historical `⌘D` collision
/// was between, so the fixtures can tell that story under its real names.
fn fixture_registry() -> Registry {
    let extra = Source {
        name: Cow::Borrowed("conflict-fixtures"),
        actions: Cow::Owned(vec![
            Action {
                id: Cow::Borrowed("host.connect"),
                label: Cow::Borrowed("Connect"),
                channels: Channels::PALETTE,
                arg: ArgShape::None,
            },
            Action {
                id: Cow::Borrowed("pane.split"),
                label: Cow::Borrowed("Split"),
                channels: Channels::PALETTE,
                arg: ArgShape::None,
            },
        ]),
    };
    merge(&[BUILT_IN, extra])
}

/// **The check itself.** The built-in table has no pair of same-chord bindings
/// whose conditions can hold together. CI runs this on every pull request, so
/// a collision is a red build rather than a behaviour someone notices later.
///
/// The failure message names both action ids and the assignment that makes
/// them collide, because "there is a conflict somewhere" is not actionable.
#[test]
fn the_built_in_keymap_has_no_conflicts() {
    let registry = merge(&[BUILT_IN]);
    let keymap = assemble(baton_action::DEFAULT_KEYMAP, &registry);

    let conflicts = keymap.conflicts();
    assert!(conflicts.is_empty(), "{}", report(&conflicts, &registry));
}

/// The message a red build prints: the chord, both permanent action ids, and
/// the assignment of identifiers that makes them collide -- everything the
/// person changing a key needs, nothing they have to go dig for.
fn report(conflicts: &[baton_action::Conflict], registry: &Registry) -> String {
    let mut out = String::from("keystrokes that can fire two actions:\n");
    for c in conflicts {
        let id = |a| registry.get(a).map(|x| x.id.as_ref()).unwrap_or("?");
        let ctx: Vec<&str> = common::spellings(c.context).collect();
        out.push_str(&format!(
            "  {}: {} and {} both fire when {{{}}}\n",
            c.chord,
            id(c.first),
            id(c.second),
            ctx.join(", "),
        ));
    }
    out
}

/// The message itself is a deliverable: it must name both ids and the
/// assignment, because "there is a conflict somewhere" gets a check ignored.
#[test]
fn the_failure_message_names_ids_chord_and_assignment() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", Some("host_selected")),
            bind("pane.split", "meta+KeyD", Some("pane_focused")),
        ],
        &registry,
    );
    let text = report(&keymap.conflicts(), &registry);

    for needle in [
        "meta+KeyD",
        "host.connect",
        "pane.split",
        "host_selected",
        "pane_focused",
    ] {
        assert!(text.contains(needle), "message omits {needle:?}: {text}");
    }
}

/// The historical collision, rebuilt: `⌘D` on "favourite" (host_selected) and
/// "split" (pane_focused). Those two conditions really are true together -- a
/// card can be selected while a pane has focus -- so this must be detected,
/// and the reported context must be one where both hold.
#[test]
fn the_cmd_d_collision_is_detected_with_its_assignment() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", Some("host_selected")),
            bind("pane.split", "meta+KeyD", Some("pane_focused")),
        ],
        &registry,
    );

    let conflicts = keymap.conflicts();
    assert_eq!(conflicts.len(), 1, "exactly this pair collides");
    let c = &conflicts[0];

    assert_eq!(registry.get(c.first).unwrap().id, "host.connect");
    assert_eq!(registry.get(c.second).unwrap().id, "pane.split");
    assert!(
        holds(c.context, Flags::HOST_SELECTED)
            && holds(c.context, Flags::PANE_FOCUSED),
        "the reported context must satisfy both guards"
    );
    // And the assignment renders as names a person can act on.
    let listed: Vec<&str> = common::spellings(c.context).collect();
    assert!(
        listed.contains(&"host_selected") && listed.contains(&"pane_focused")
    );
}

/// A shared chord whose guards are exclusive is NOT a conflict -- false
/// positives are how a check gets switched off. The sidebar has one mode, so
/// these two can never both hold.
#[test]
fn a_pair_split_by_an_exclusive_group_is_not_reported() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", Some("sidebar_hosts")),
            bind("pane.split", "meta+KeyD", Some("sidebar_workspaces")),
        ],
        &registry,
    );
    assert!(keymap.conflicts().is_empty());
}

/// The exclusivity declaration is load-bearing: the same pair IS satisfiable
/// with no exclusivities declared. This is what pins "the declaration is what
/// the checker uses" -- delete the group and the test above starts lying.
#[test]
fn the_exclusive_declaration_is_what_splits_that_pair() {
    let a = clause("sidebar_hosts");
    let b = clause("sidebar_workspaces");

    assert!(
        satisfiable_together(&a, &b, &[]).is_some(),
        "without the declaration the pair is satisfiable"
    );
    assert!(
        satisfiable_together(&a, &b, Flags::EXCLUSIVE).is_none(),
        "the declared group is the only thing splitting it"
    );
}

/// Negation splits a pair with no declaration needed.
#[test]
fn a_negated_pair_is_disjoint_on_its_own() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", Some("palette_open")),
            bind("pane.split", "meta+KeyD", Some("!palette_open")),
        ],
        &registry,
    );
    assert!(keymap.conflicts().is_empty());
}

/// Two unconditional bindings on one chord conflict everywhere; the reported
/// context is the empty one, which is the smallest honest witness.
#[test]
fn two_unconditional_bindings_conflict_in_the_empty_context() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", None),
            bind("pane.split", "meta+KeyD", None),
        ],
        &registry,
    );
    let conflicts = keymap.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].context, Flags::NONE);
}

/// An unconditional binding conflicts with a conditional one wherever the
/// condition holds.
#[test]
fn unconditional_meets_conditional_where_the_condition_holds() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", None),
            bind("pane.split", "meta+KeyD", Some("pane_focused")),
        ],
        &registry,
    );
    let conflicts = keymap.conflicts();
    assert_eq!(conflicts.len(), 1);
    assert!(holds(conflicts[0].context, Flags::PANE_FOCUSED));
}

/// One member of an exclusive group, alone, is an ordinary condition -- the
/// groups mean "at most one", not "none". Two guards that both need
/// `pane_live` collide where it holds, and a checker that quietly excluded
/// every group member would MISS this collision, which is the dangerous
/// direction: a false negative here ships a broken keymap.
#[test]
fn a_single_member_of_an_exclusive_group_still_collides() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", Some("pane_live")),
            bind(
                "pane.split",
                "meta+KeyD",
                Some("pane_live && has_selection"),
            ),
        ],
        &registry,
    );
    let conflicts = keymap.conflicts();
    assert_eq!(conflicts.len(), 1, "one member of a group is satisfiable");
    assert!(holds(conflicts[0].context, Flags::PANE_LIVE));
}

/// The sweep reaches the highest bit. `dialog_hostkey_changed` is the last
/// declared flag, so a pair only satisfiable where it holds pins the sweep's
/// upper bound -- an off-by-one halving the shift in `assignments()` would
/// silently shrink the assignment space and miss exactly this collision, the same
/// false-negative class the single-member test pins for `excludes`.
#[test]
fn the_sweep_reaches_the_highest_flag() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", Some("dialog_hostkey_changed")),
            bind("pane.split", "meta+KeyD", Some("dialog_hostkey_changed")),
        ],
        &registry,
    );
    let conflicts = keymap.conflicts();
    assert_eq!(conflicts.len(), 1, "the top bit must be reachable");
    assert!(holds(conflicts[0].context, Flags::DIALOG_HOSTKEY_CHANGED));
}

/// A guard that is unsatisfiable on its own (it violates an exclusive group)
/// can never fire, so it conflicts with nothing -- not even an unconditional
/// binding on the same chord. That is a defect in the binding, not a
/// collision; it is not this checker's finding to report.
#[test]
fn an_unsatisfiable_guard_conflicts_with_nothing() {
    let registry = fixture_registry();
    let keymap = assemble(
        &[
            bind("host.connect", "meta+KeyD", None),
            bind(
                "pane.split",
                "meta+KeyD",
                Some("sidebar_hosts && sidebar_workspaces"),
            ),
        ],
        &registry,
    );
    assert!(keymap.conflicts().is_empty());
}
