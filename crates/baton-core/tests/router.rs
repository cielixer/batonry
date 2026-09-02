//! The input router: one pure dispatch decision, no object and no writes.
//!
//! What these tests pin, in the ticket's words (#13): bytes arrive exactly as
//! dispatched with no appended newline; a `Set` of two panes receives
//! identical bytes even though no M1 UI reaches that path; and a pane that
//! closed between targeting and dispatch is dropped quietly, never panicked
//! on.

use baton_core::{PaneId, TargetSet, route_input};

/// Runs one dispatch and records every delivery as `(pane, bytes)`.
fn deliveries(
    targets: &TargetSet,
    focused: Option<PaneId>,
    live: &[PaneId],
    bytes: &[u8],
) -> Vec<(PaneId, Vec<u8>)> {
    let mut log = Vec::new();
    route_input(
        targets,
        focused,
        |id| live.contains(&id),
        bytes,
        |id, b| log.push((id, b.to_vec())),
    );
    log
}

/// M2 stages the tmux install command and M3 the agent one, both leaving the
/// user to press Enter -- so the send path appending a newline is the exact
/// regression this guards, extending baton-term's existing
/// `write_path_sends_raw_bytes_without_newline` up to the router.
#[test]
fn bytes_arrive_without_an_appended_newline() {
    let pane = PaneId::new(1);
    let log = deliveries(
        &TargetSet::Focused,
        Some(pane),
        &[pane],
        b"echo no newline",
    );
    assert_eq!(log, [(pane, b"echo no newline".to_vec())]);
}

/// Escape sequences and embedded newlines pass through untouched too: the
/// router forwards bytes, it does not interpret them.
#[test]
fn control_bytes_pass_through_exactly() {
    let pane = PaneId::new(1);
    let raw = b"\x1b[200~pasted\ntext\x1b[201~";
    let log = deliveries(&TargetSet::Focused, Some(pane), &[pane], raw);
    assert_eq!(log, [(pane, raw.to_vec())]);
}

/// The broadcast path is proven now, while no UI reaches it (#13): both panes
/// in a `Set` receive identical bytes from one dispatch.
#[test]
fn a_set_of_two_panes_receives_identical_bytes() {
    let (a, b) = (PaneId::new(1), PaneId::new(2));
    let log =
        deliveries(&TargetSet::Set(vec![a, b]), None, &[a, b], b"broadcast");
    assert_eq!(
        log,
        [(a, b"broadcast".to_vec()), (b, b"broadcast".to_vec())]
    );
}

/// A pane that closed between targeting and dispatch is dropped quietly; the
/// panes still live receive as usual.
#[test]
fn a_closed_pane_in_a_set_is_dropped_quietly() {
    let (closed, live) = (PaneId::new(2), PaneId::new(1));
    let log = deliveries(
        &TargetSet::Set(vec![closed, live]),
        None,
        &[live],
        b"still flowing",
    );
    assert_eq!(log, [(live, b"still flowing".to_vec())]);
}

/// The focused pane closing is the same quiet drop, and so is having no
/// focus at all -- a dispatch with nowhere to go does nothing.
#[test]
fn a_closed_or_absent_focus_drops_quietly() {
    assert!(deliveries(&TargetSet::Focused, None, &[], b"nowhere").is_empty());

    let closed = PaneId::new(7);
    let log =
        deliveries(&TargetSet::Focused, Some(closed), &[], b"pane closed");
    assert!(log.is_empty());
}

/// Focus is an argument, not captured state: the same call with a different
/// focused pane reaches a different pane, and only that one.
#[test]
fn dispatch_follows_the_focus() {
    let (first, second) = (PaneId::new(1), PaneId::new(2));
    let live = [first, second];

    let to_first =
        deliveries(&TargetSet::Focused, Some(first), &live, b"to first");
    let to_second =
        deliveries(&TargetSet::Focused, Some(second), &live, b"to second");

    assert_eq!(to_first, [(first, b"to first".to_vec())]);
    assert_eq!(to_second, [(second, b"to second".to_vec())]);
}
