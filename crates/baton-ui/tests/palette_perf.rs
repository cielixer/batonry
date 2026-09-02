//! The ticket's performance floor (#15): open to first result in under
//! 50 ms with 500 actions registered. Measured, not asserted by feel --
//! the number this prints goes into the PR.

use std::borrow::Cow;
use std::time::Instant;

use baton_action::{Action, ArgShape, Channels, Source, merge};

/// 500 synthetic actions beside the built-in table, in one registry.
fn registry_with_500() -> (baton_action::Registry, Vec<String>) {
    let ids: Vec<String> =
        (0..500).map(|n| format!("synthetic.action{n}")).collect();
    let actions: Vec<Action> = ids
        .iter()
        .enumerate()
        .map(|(n, id)| Action {
            id: Cow::Owned(id.clone()),
            label: Cow::Owned(format!("Synthetic Action {n} For Measure")),
            channels: Channels::PALETTE,
            arg: ArgShape::None,
        })
        .collect();
    let synthetic = Source {
        name: Cow::Borrowed("perf-fixture"),
        actions: Cow::Owned(actions),
    };
    (merge(&[baton_action::BUILT_IN, synthetic]), ids)
}

/// Open-to-first-result: what the palette computes between the chord and
/// the first painted row is one ranking pass over every label. The floor
/// covers the worst of an empty query (everything matches) and a narrowing
/// one (scores everywhere).
#[test]
fn open_to_first_result_stays_under_50ms_with_500_actions() {
    let (registry, _ids) = registry_with_500();
    let labels = || registry.iter().map(|(_, action)| action.label.as_ref());
    assert!(labels().count() >= 500, "the fixture actually registered");

    // Warm one pass so the measurement is the steady state, like a palette
    // reopened after boot.
    let _ = baton_ui::palette_rank_for_measure("", labels());

    let empty_start = Instant::now();
    let all = baton_ui::palette_rank_for_measure("", labels());
    let empty_elapsed = empty_start.elapsed();

    let query_start = Instant::now();
    let narrowed = baton_ui::palette_rank_for_measure("synac37", labels());
    let query_elapsed = query_start.elapsed();

    eprintln!(
        "palette ranking over {} labels: empty query {:?}, fuzzy query {:?}",
        all.len(),
        empty_elapsed,
        query_elapsed
    );
    assert!(!narrowed.is_empty(), "the fuzzy query matches the fixture");
    assert!(
        empty_elapsed.as_millis() < 50 && query_elapsed.as_millis() < 50,
        "open-to-first-result exceeded the 50 ms floor: empty {empty_elapsed:?}, fuzzy {query_elapsed:?}"
    );
}
