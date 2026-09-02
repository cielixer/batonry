//! Handles one terminal event: `update`'s terminal arm, extracted so that
//! A11 has a single home -- every byte heading for a terminal goes through
//! `baton_core::route_input`, and this file is the shell's only pty-write
//! site (it implements the Delivery half of baton-core's vocabulary). Non-`Write` commands bypass the
//! router because they are pane-bound by construction: resize and selection
//! control the widget or backend, while `MouseReport` and alt-screen
//! `Scroll` do write pty bytes -- but as pointer telemetry whose coordinates
//! target the pane under the cursor, they can never follow focus or
//! broadcast (#107). What routes is what M2's broadcast can fan
//! out: keystrokes, paste, snippets, palette sends.

use baton_core::{TargetSet, route_input};
use baton_term::actions::Action;
use baton_term::{BackendCommand, Command, Event, Terminal};

/// Handles one terminal event: input through the router, control directly.
/// Returns the widget's reaction -- [`Action::Shutdown`] and
/// [`Action::ChangeTitle`] must not be dropped silently; the caller decides.
pub(crate) fn handle(
    targets: &TargetSet,
    focused: Option<baton_core::PaneId>,
    pane_of_terminal: impl Fn(u64) -> Option<baton_core::PaneId>,
    terminal: &mut Option<Terminal>,
    event: Event,
) -> Action {
    let Event::BackendCall(term_id, command) = event;

    match command {
        BackendCommand::Write(bytes) => {
            let terminal_is_live = terminal
                .as_ref()
                .is_some_and(|current| current.id == term_id);
            let is_live = |pane| {
                terminal_is_live && pane_of_terminal(term_id) == Some(pane)
            };

            let mut action = Action::Ignore;
            route_input(targets, focused, is_live, &bytes, |_, input| {
                if let Some(current) = terminal.as_mut()
                    && current.id == term_id
                {
                    action = current.handle(Command::ProxyToBackend(
                        BackendCommand::Write(input.to_vec()),
                    ));
                }
            });
            action
        },
        command => match terminal.as_mut() {
            Some(current) if current.id == term_id => {
                current.handle(Command::ProxyToBackend(command))
            },
            _ => Action::Ignore,
        },
    }
}
