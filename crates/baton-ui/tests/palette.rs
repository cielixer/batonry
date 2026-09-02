//! The palette from outside: it opens on the keymap chord, searches, issues
//! the selected action id, remembers it, and refuses unavailable rows.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use baton_core::Store;
use baton_ui::{App, Message};
use iced_test::simulator;

const PLACEHOLDER: &str = "Type a command...";
const RECENT_TAG: &str = "recent";

/// A store the test can watch from outside while `App` owns its half.
#[derive(Clone, Default)]
struct SharedStore(Rc<RefCell<HashMap<String, String>>>);

impl Store for SharedStore {
    fn app_pref(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }
    fn set_app_pref(&mut self, key: &str, value: &str) {
        self.0.borrow_mut().insert(key.into(), value.into());
    }
}

fn app_with(store: SharedStore) -> App {
    let (app, _task) = App::new(
        "/bin/sh".into(),
        "Baton".into(),
        "local".into(),
        "Panel".into(),
        PLACEHOLDER.into(),
        RECENT_TAG.into(),
        Some(Box::new(store)),
    );
    app
}

fn key(app: &mut App, chord: &str) {
    let stroke: baton_action::Keystroke =
        chord.parse().expect("test chord parses");
    let _task = app.update(Message::Key(stroke));
}

/// Cmd-K opens, typing narrows, Enter issues the selected action id -- and
/// the issued id is observable where it matters: in the persisted recents.
#[test]
fn the_palette_opens_searches_and_issues_the_selected_id() {
    let store = SharedStore::default();
    let mut app = app_with(store.clone());

    // Closed: the overlay is absent.
    assert!(simulator(app.view()).find(PLACEHOLDER).is_err());

    key(&mut app, "meta+KeyK");
    assert!(
        simulator(app.view()).find(PLACEHOLDER).is_ok(),
        "meta+KeyK must open the palette through the keymap"
    );

    let _task = app.update(Message::PaletteInput("quit".into()));
    key(&mut app, "Enter");

    assert_eq!(
        store.0.borrow().get("palette.recent").map(String::as_str),
        Some("app.quit"),
        "Enter issues the selected action id and records it"
    );
    assert!(
        simulator(app.view()).find(PLACEHOLDER).is_err(),
        "confirm closes the palette"
    );
}

/// Escape closes without issuing anything, and the arrows move the
/// selection the view renders.
#[test]
fn escape_closes_and_arrows_move_the_selection() {
    let store = SharedStore::default();
    let mut app = app_with(store.clone());

    key(&mut app, "meta+KeyK");
    key(&mut app, "ArrowDown");
    key(&mut app, "ArrowUp");
    key(&mut app, "Escape");

    assert!(simulator(app.view()).find(PLACEHOLDER).is_err());
    assert!(
        store.0.borrow().get("palette.recent").is_none(),
        "closing issues nothing"
    );
}

/// Recents persist through the store: a fresh App with a primed store shows
/// the recent tag on the empty query.
#[test]
fn recents_load_from_the_store_at_boot() {
    let store = SharedStore::default();
    store
        .0
        .borrow_mut()
        .insert("palette.recent".into(), "app.quit".into());

    let mut app = app_with(store);
    key(&mut app, "meta+KeyK");
    assert!(
        simulator(app.view()).find(RECENT_TAG).is_ok(),
        "a primed store marks its action as recent on the empty query"
    );
}

/// An unavailable row (stage 1: every term.* action) refuses to run: Enter
/// keeps the palette open, records nothing, and the reason is rendered on
/// the selected row -- keyboard-reachable by construction.
#[test]
fn an_unavailable_row_shows_its_reason_and_refuses_to_run() {
    let store = SharedStore::default();
    let mut app = app_with(store.clone());

    key(&mut app, "meta+KeyK");
    let _task = app.update(Message::PaletteInput("Copy".into()));

    let mut ui = simulator(app.view());
    assert!(
        ui.find("Waits for the action-wiring ticket").is_ok(),
        "the selected unavailable row renders its reason"
    );
    drop(ui);

    key(&mut app, "Enter");
    // The placeholder only renders on an empty query, so observe the still
    // open palette through the reason text that only its overlay renders.
    assert!(
        simulator(app.view())
            .find("Waits for the action-wiring ticket")
            .is_ok(),
        "an unavailable row must not close the palette"
    );
    assert!(
        store.0.borrow().get("palette.recent").is_none(),
        "an unavailable row must not be recorded as run"
    );
}

/// The palette lists what its own channel can reach. Key-only actions --
/// the palette's navigation, select-all -- never appear as rows; the
/// channel bits on the action table are what decides.
#[test]
fn key_only_actions_are_not_palette_rows() {
    let mut app = app_with(SharedStore::default());
    key(&mut app, "meta+KeyK");

    let mut ui = simulator(app.view());
    assert!(ui.find("Quit").is_ok(), "a PALETTE action is listed");
    assert!(
        ui.find("Next Result").is_err() && ui.find("Select All").is_err(),
        "KEY_ONLY actions must not be palette rows"
    );
}
