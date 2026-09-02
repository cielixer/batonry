//! Wire up adapters and run. No logic lives here.

// Every user-visible string, in the one place to edit copy -- the UI
// renders what it is handed and writes no literal of its own.
const APP_TITLE: &str = "Baton";
const TERMINAL_LABEL: &str = "local";
const RIGHT_DOCK_COLLAPSED_HINT: &str = "Panel";
const PALETTE_PLACEHOLDER: &str = "Type a command...";
const RECENT_TAG: &str = "recent";

fn main() -> iced::Result {
    iced::application(
        || {
            let store = baton_store::SqliteStore::open_at(
                &baton_platform::data_dir().join("baton.db"),
            )
            .ok()
            .map(|store| Box::new(store) as Box<dyn baton_core::Store>);
            baton_ui::App::new(
                baton_platform::default_shell(),
                APP_TITLE.into(),
                TERMINAL_LABEL.into(),
                RIGHT_DOCK_COLLAPSED_HINT.into(),
                PALETTE_PLACEHOLDER.into(),
                RECENT_TAG.into(),
                store,
            )
        },
        baton_ui::App::update,
        baton_ui::App::view,
    )
    .subscription(baton_ui::App::subscription)
    .title(baton_ui::App::title)
    .run()
}
