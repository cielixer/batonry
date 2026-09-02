use crate::terminal_event;
use crate::theme::Theme;
use baton_core::{PaneId, TargetSet};
use baton_term::settings::{BackendSettings, Settings};
use baton_term::{Event as TerminalEvent, Terminal};

/// The Elm shell state that owns the stage-1 terminal and its routing state.
pub struct App {
    pub(crate) terminal: Option<Terminal>,
    pane: PaneId,
    focused: Option<PaneId>,
    pub(crate) theme: Theme,
    pub(crate) app_title: String,
    pub(crate) terminal_label: String,
    pub(crate) right_dock_collapsed_hint: String,
}

/// Messages emitted by the stage-1 terminal subscription.
#[derive(Clone, Debug)]
pub enum Message {
    Terminal(TerminalEvent),
}

impl App {
    /// Creates the shell with one focused pane; the returned task moves
    /// the widget's keyboard focus onto it.
    ///
    /// `shell` is the path of the shell this terminal runs; the three
    /// strings are every word the shell shows. All injected by `main`: which
    /// shell a platform defaults to is an OS decision this crate does not
    /// make, and the copy is the assembler's to own and to localise -- this
    /// crate writes no user-visible literal.
    pub fn new(
        shell: String,
        app_title: String,
        terminal_label: String,
        right_dock_collapsed_hint: String,
    ) -> (Self, iced::Task<Message>) {
        let pane = PaneId::new(0);
        let settings = Settings {
            backend: BackendSettings {
                program: shell,
                ..BackendSettings::default()
            },
            ..Settings::default()
        };
        // The io::Error cause is dropped: stage 1 has no failure surface to
        // show it on, and never-rule 7 forbids logging the path. The
        // disconnected-pane UX (stage 2) is where the cause becomes visible.
        let terminal = Terminal::new(0, settings).ok();

        // The widget starts unfocused and its keyboard arm drops events
        // until it is focused -- without this task, typing does nothing
        // until the user clicks the terminal.
        let focus_widget =
            terminal.as_ref().map_or_else(iced::Task::none, |terminal| {
                baton_term::TerminalView::focus(terminal.widget_id().clone())
            });

        (
            Self {
                terminal,
                pane,
                focused: Some(pane),
                theme: Theme::default(),
                app_title,
                terminal_label,
                right_dock_collapsed_hint,
            },
            focus_widget,
        )
    }

    /// Applies a message without panicking when the terminal has disappeared.
    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Terminal(event) => {
                let pane = self.pane;
                let terminal_id =
                    self.terminal.as_ref().map(|terminal| terminal.id);
                // The routing mode is a constant until a broadcast toggle
                // exists (M2): a field nothing can change is not state, and
                // A11's promise -- broadcast is one routing-site change --
                // is exactly this line.
                let action = terminal_event::handle(
                    &TargetSet::Focused,
                    self.focused,
                    move |id| (terminal_id == Some(id)).then_some(pane),
                    &mut self.terminal,
                    event,
                );
                match action {
                    // The pty ended (the shell exited): the terminal is
                    // gone, not frozen. The stage-2 disconnected-pane UX
                    // replaces this with a reason and a reconnect action.
                    baton_term::actions::Action::Shutdown => {
                        self.terminal = None
                    },
                    // Consumed by the tab title when workspaces arrive
                    // (stage 3); dropped knowingly until then.
                    baton_term::actions::Action::ChangeTitle(_) => {},
                    baton_term::actions::Action::Ignore => {},
                }
            },
        }

        iced::Task::none()
    }

    /// Subscribes to terminal events while the stage-1 terminal is live.
    pub fn subscription(&self) -> iced::Subscription<Message> {
        self.terminal
            .as_ref()
            .map_or_else(iced::Subscription::none, |terminal| {
                terminal.subscription().map(Message::Terminal)
            })
    }

    /// Returns the application window title.
    pub fn title(&self) -> String {
        self.app_title.clone()
    }

    /// Moves keyboard focus to a pane, or nowhere. Routing follows it: input
    /// is delivered to the focused pane and dropped when there is none.
    pub fn focus(&mut self, pane: Option<PaneId>) {
        self.focused = pane;
    }

    /// The terminal grid as text, the render-verification channel this
    /// repository uses instead of pixels (root contract section 7): what the
    /// pty produced, asserted as characters. `None` when no terminal lives.
    pub fn dump_grid(&mut self) -> Option<String> {
        self.terminal.as_mut().map(|terminal| terminal.dump_grid())
    }

    /// Builds the shell from a fresh display-only projection.
    pub fn view(&self) -> iced::Element<'_, Message> {
        let projection = crate::project::project(self);
        crate::view::view(&projection)
    }
}
