use crate::actions::Action;
use crate::backend;
use crate::bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::font::TermFont;
use crate::settings::{FontSettings, Settings, ThemeSettings};
use crate::theme::{ColorPalette, Theme};
use crate::AlacrittyEvent;
use alacritty_terminal::grid::Dimensions as _;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::canvas::Cache;
use iced::Subscription;
use std::hash::{Hash, Hasher};
use std::io::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum Event {
    BackendCall(u64, backend::Command),
}

#[derive(Debug, Clone)]
pub enum Command {
    ChangeTheme(Box<ColorPalette>),
    ChangeFont(FontSettings),
    AddBindings(Vec<(Binding<InputKind>, BindingAction)>),
    ProxyToBackend(backend::Command),
}

pub struct Terminal {
    pub id: u64,
    widget_id: iced::widget::Id,
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
    pub(crate) cache: Cache,
    pub(crate) bindings: BindingsLayout,
    pub(crate) backend: backend::Backend,
    backend_event_rx: Arc<Mutex<backend::EventSink>>,
}

impl Terminal {
    pub fn new(id: u64, settings: Settings) -> Result<Self> {
        // BATON: two channels. Wakeups get capacity 1 and are coalesced at
        // the source; everything else is unbounded so it cannot be dropped.
        // See `backend::EventProxy` for why.
        let (wakeup_tx, wakeup_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let theme = Theme::new(settings.theme);
        let font = TermFont::new(settings.font);

        Ok(Self {
            id,
            widget_id: iced::widget::Id::unique(),
            font,
            theme,
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            backend: backend::Backend::new(
                id,
                wakeup_tx,
                event_tx,
                settings.backend,
            )?,
            backend_event_rx: Arc::new(Mutex::new(backend::EventSink {
                wakeups: wakeup_rx,
                events: event_rx,
            })),
        })
    }

    pub fn widget_id(&self) -> &iced::widget::Id {
        &self.widget_id
    }

    pub fn subscription(&self) -> Subscription<Event> {
        let data = TerminalSubscriptionData {
            id: self.id,
            event_receiver: self.backend_event_rx.clone(),
        };

        Subscription::run_with(data, terminal_subscription_stream)
    }

    pub fn handle(&mut self, cmd: Command) -> Action {
        let mut action = Action::default();

        match cmd {
            Command::ChangeTheme(color_pallete) => {
                self.theme = Theme::new(ThemeSettings::new(color_pallete));
            },
            Command::ChangeFont(font_settings) => {
                self.font = TermFont::new(font_settings);
            },
            Command::AddBindings(bindings) => {
                self.bindings.add_bindings(bindings);
            },
            Command::ProxyToBackend(cmd) => {
                crate::metrics::COMMANDS
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                action = self.backend.handle(cmd);
            },
        };

        self.sync_and_redraw();
        action
    }

    fn sync_and_redraw(&mut self) {
        self.sync_font();
        self.backend.sync();
        self.redraw();
    }

    fn sync_font(&mut self) {
        self.font.sync();
        self.backend
            .handle(backend::Command::Resize(None, Some(self.font.measure)));
    }

    fn redraw(&mut self) {
        // BATON: the original clears the cache on every command. Damage-based
        // invalidation only throws the tessellation away when the grid
        // actually changed.
        if crate::metrics::damage_aware() {
            if self.backend.take_damaged() {
                self.cache.clear();
            }
        } else {
            self.cache.clear();
        }
    }
}

#[derive(Clone)]
struct TerminalSubscriptionData {
    id: u64,
    event_receiver: Arc<Mutex<backend::EventSink>>,
}

impl Hash for TerminalSubscriptionData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn terminal_subscription_stream(
    data: &TerminalSubscriptionData,
) -> BoxStream<'static, Event> {
    let id = data.id;
    let event_receiver = data.event_receiver.clone();
    iced::stream::channel(1000, async move |mut output| {
        loop {
            let mut sink = event_receiver.lock().await;
            // Borrow the receivers separately: `select!` cannot borrow the
            // same value twice.
            let backend::EventSink { wakeups, events } = &mut *sink;
            // BATON: wait on both. `biased` checks the lossless path first;
            // wakeups are coalesced to at most one in flight, so they cannot
            // starve.
            let event = tokio::select! {
                biased;
                ev = events.recv() => match ev {
                    Some(ev) => {
                        crate::metrics::queue_pop();
                        ev
                    },
                    // All senders gone: the backend is finished.
                    None => return,
                },
                w = wakeups.recv() => match w {
                    Some(()) => AlacrittyEvent::Wakeup,
                    None => return,
                },
            };
            drop(sink);

            // BATON: the original panics here. A closed channel is the normal
            // way a pane shuts down -- the UI dropped it first -- so just stop.
            if output
                .send(Event::BackendCall(
                    id,
                    backend::Command::ProcessAlacrittyEvent(event),
                ))
                .await
                .is_err()
            {
                return;
            }
        }
    })
    .boxed()
}

impl Terminal {
    /// BATON: dump the grid as text.
    ///
    /// This is the canonical correctness oracle for rendering. A pixel
    /// comparison only says that something changed, never what is correct,
    /// and it depends on the machine's fonts. This does not.
    ///
    /// Spacer cells following a double-width character are skipped so the dump
    /// reads the way a human sees the screen. Trailing blanks are trimmed.
    pub fn dump_grid(&mut self) -> String {
        use alacritty_terminal::term::cell::Flags;
        self.backend.sync();
        let content = self.backend.renderable_content();
        let cols = content.grid.columns();
        let mut out = String::new();
        let mut line = String::new();
        let mut col = 0usize;
        for indexed in content.grid.display_iter() {
            if !indexed.cell.flags.intersects(
                Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER,
            ) {
                line.push(indexed.c);
            }
            col += 1;
            if col == cols {
                out.push_str(line.trim_end());
                out.push('\n');
                line.clear();
                col = 0;
            }
        }
        // Trailing blank lines carry no information.
        while out.ends_with("\n\n") {
            out.pop();
        }
        out
    }

    /// BATON: the event stream without going through `iced`.
    ///
    /// Tests use this door. A `Subscription` only runs inside the iced
    /// runtime, but the deadlock regression has to be driven from outside it,
    /// so that a wedged pump fails the test instead of hanging it.
    pub fn event_stream(&self) -> BoxStream<'static, Event> {
        terminal_subscription_stream(&TerminalSubscriptionData {
            id: self.id,
            event_receiver: self.backend_event_rx.clone(),
        })
    }
}
