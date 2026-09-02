use crate::terminal_event;
use crate::theme::Theme;
use crate::{keys, search};
use baton_action::{ActionId, Flags, Keymap, Keystroke, Registry};
use baton_core::{PaneId, Store, TargetSet};
use baton_term::settings::{BackendSettings, Settings};
use baton_term::{Event as TerminalEvent, Terminal};

const PALETTE_RECENT_KEY: &str = "palette.recent";
const PALETTE_RECENT_LIMIT: usize = 8;

/// The mutable, non-visual state of the command palette.
pub(crate) struct PaletteState {
    pub(crate) query: String,
    pub(crate) selected: usize,
}

/// The stable id used by the palette query field when the view is added.
pub(crate) const PALETTE_INPUT_ID: iced::widget::Id =
    iced::widget::Id::new("palette_input");

/// The Elm shell state that owns the stage-1 terminal and its routing state.
pub struct App {
    pub(crate) terminal: Option<Terminal>,
    pane: PaneId,
    focused: Option<PaneId>,
    pub(crate) theme: Theme,
    pub(crate) app_title: String,
    pub(crate) terminal_label: String,
    pub(crate) right_dock_collapsed_hint: String,
    pub(crate) palette_placeholder: String,
    pub(crate) recent_tag: String,
    pub(crate) store: Option<Box<dyn Store>>,
    pub(crate) palette: Option<PaletteState>,
    pub(crate) registry: Registry,
    pub(crate) keymap: Keymap,
    pub(crate) recents: Vec<String>,
}

/// Messages emitted by the stage-1 terminal and keyboard subscriptions.
#[derive(Clone, Debug)]
pub enum Message {
    Terminal(TerminalEvent),
    Key(Keystroke),
    PaletteInput(String),
    PaletteHover(usize),
    PaletteConfirmRow(usize),
}

impl App {
    /// Creates the shell with one focused pane; the returned task moves
    /// the widget's keyboard focus onto it.
    ///
    /// `shell` is the path of the shell this terminal runs; the three
    /// strings are every word the shell shows. All injected by `main`: which
    /// shell a platform defaults to is an OS decision this crate does not
    /// make, and the copy is the assembler's to own and to localise -- this
    /// crate writes no user-visible literal. `store` is optional convenience
    /// persistence and may be absent when opening the database failed.
    pub fn new(
        shell: String,
        app_title: String,
        terminal_label: String,
        right_dock_collapsed_hint: String,
        palette_placeholder: String,
        recent_tag: String,
        store: Option<Box<dyn Store>>,
    ) -> (Self, iced::Task<Message>) {
        let registry = baton_action::merge(&[baton_action::BUILT_IN]);
        let keymap =
            baton_action::assemble(baton_action::DEFAULT_KEYMAP, &registry);
        let recents = store
            .as_ref()
            .and_then(|store| store.app_pref(PALETTE_RECENT_KEY))
            .map(|value| {
                value
                    .lines()
                    .take(PALETTE_RECENT_LIMIT)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
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
                palette_placeholder,
                recent_tag,
                store,
                palette: None,
                registry,
                keymap,
                recents,
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
                iced::Task::none()
            },
            Message::Key(stroke) => self.handle_key(stroke),
            Message::PaletteInput(query) => {
                if let Some(palette) = self.palette.as_mut() {
                    palette.query = query;
                    palette.selected = 0;
                }
                iced::Task::none()
            },
            Message::PaletteHover(index) => {
                let result_count = self
                    .palette
                    .as_ref()
                    .map(|palette| {
                        search::palette_results(&self.registry, &palette.query)
                            .len()
                    })
                    .unwrap_or(0);
                if let Some(palette) = self.palette.as_mut() {
                    palette.selected =
                        index.min(result_count.saturating_sub(1));
                }
                iced::Task::none()
            },
            Message::PaletteConfirmRow(index) => {
                let result_count = self
                    .palette
                    .as_ref()
                    .map(|palette| {
                        search::palette_results(&self.registry, &palette.query)
                            .len()
                    })
                    .unwrap_or(0);
                if let Some(palette) = self.palette.as_mut() {
                    palette.selected =
                        index.min(result_count.saturating_sub(1));
                    self.confirm_selected()
                } else {
                    iced::Task::none()
                }
            },
        }
    }

    /// Subscribes to terminal events and ignored keyboard events.
    ///
    /// A focused terminal eats bare keys as pty input, and the palette's text
    /// input eats characters while editing. What falls through is what the
    /// keymap may claim.
    pub fn subscription(&self) -> iced::Subscription<Message> {
        let terminal = self
            .terminal
            .as_ref()
            .map_or_else(iced::Subscription::none, |terminal| {
                terminal.subscription().map(Message::Terminal)
            });
        let keyboard = iced::event::listen_with(|event, status, _window| {
            if status != iced::event::Status::Ignored {
                return None;
            }
            let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                physical_key,
                modifiers,
                ..
            }) = event
            else {
                return None;
            };
            keys::keystroke(physical_key, modifiers).map(Message::Key)
        });

        // A focused terminal eats bare keys as pty input, and the palette's
        // text input eats characters while editing. Only what falls through
        // may be claimed by the keymap.
        iced::Subscription::batch([terminal, keyboard])
    }

    fn handle_key(&mut self, stroke: Keystroke) -> iced::Task<Message> {
        let mut context = Flags::NONE;
        if self.palette.is_some() {
            context = baton_action::combine(context, Flags::PALETTE_OPEN);
            // Stage 1 treats the palette query as the only text editor.
            context = baton_action::combine(context, Flags::EDITING_TEXT);
        }
        if self.terminal.is_some() && self.palette.is_none() {
            // Stage 1 has one pane, so a live terminal approximates focus.
            context = baton_action::combine(context, Flags::PANE_FOCUSED);
        }

        self.keymap
            .lookup(stroke, context)
            .map_or_else(iced::Task::none, |id| self.act(id))
    }

    fn act(&mut self, id: ActionId) -> iced::Task<Message> {
        let Some(name) =
            self.registry.get(id).map(|action| action.id.to_string())
        else {
            return iced::Task::none();
        };

        match name.as_str() {
            "palette.open" => {
                self.palette = Some(PaletteState {
                    query: String::new(),
                    selected: 0,
                });
                iced::widget::operation::focus(PALETTE_INPUT_ID.clone())
            },
            "palette.close" => {
                self.palette = None;
                self.terminal.as_ref().map_or_else(
                    iced::Task::none,
                    |terminal| {
                        baton_term::TerminalView::focus(
                            terminal.widget_id().clone(),
                        )
                    },
                )
            },
            "palette.next" => {
                self.move_selection(true);
                iced::Task::none()
            },
            "palette.prev" => {
                self.move_selection(false);
                iced::Task::none()
            },
            "palette.confirm" | "palette.confirm.alt" => {
                self.confirm_selected()
            },
            "app.quit" => iced::exit(),
            action if action.starts_with("term.") => {
                // Terminal action wiring belongs to its own action-wiring ticket.
                iced::Task::none()
            },
            _ => iced::Task::none(),
        }
    }

    fn move_selection(&mut self, next: bool) {
        let Some(palette) = self.palette.as_mut() else {
            return;
        };
        let result_count =
            search::palette_results(&self.registry, &palette.query).len();
        if result_count == 0 {
            palette.selected = 0;
            return;
        }

        let last = result_count - 1;
        palette.selected = palette.selected.min(last);
        if next {
            // Navigation stops at each end; wrapping makes keyboard focus
            // jump away from the result the user just reached.
            palette.selected = palette.selected.saturating_add(1).min(last);
        } else {
            palette.selected = palette.selected.saturating_sub(1);
        }
    }

    fn confirm_selected(&mut self) -> iced::Task<Message> {
        let selection = self.palette.as_mut().and_then(|palette| {
            let results =
                search::palette_results(&self.registry, &palette.query);
            if palette.selected >= results.len() {
                palette.selected = results.len().saturating_sub(1);
            }
            results
                .get(palette.selected)
                .map(|result| (result.id.to_owned(), result.availability))
        });
        let Some((id, search::Availability::Ready)) = selection else {
            // An empty or unavailable row stays open and cannot claim to run.
            return iced::Task::none();
        };
        let Some(action_id) = self.registry.resolve(&id) else {
            return iced::Task::none();
        };

        self.recents.retain(|recent| recent != &id);
        self.recents.insert(0, id.clone());
        self.recents.truncate(PALETTE_RECENT_LIMIT);
        if let Some(store) = self.store.as_mut() {
            store.set_app_pref(PALETTE_RECENT_KEY, &self.recents.join("\n"));
        }
        self.palette = None;
        // Closing by confirm must hand focus back like palette.close does:
        // with no widget focused, bare keys fall through everywhere -- the
        // palette guards reject them and the terminal never sees them.
        let refocus =
            self.terminal
                .as_ref()
                .map_or_else(iced::Task::none, |terminal| {
                    baton_term::TerminalView::focus(
                        terminal.widget_id().clone(),
                    )
                });
        iced::Task::batch([refocus, self.act(action_id)])
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
