//! Hardcopy of `iced_term` 0.8.0. Every line we changed carries a `// BATON:`
//! comment; `UPSTREAM.diff` next to this file is the same delta in
//! machine-readable form and says how to regenerate it.

pub mod actions;
pub mod bindings;
pub mod metrics; // BATON: new. Instrumentation for the render path.
pub mod settings;

mod backend;
mod font;
mod terminal;
mod theme;
mod view;

pub use alacritty_terminal::event::Event as AlacrittyEvent;
pub use alacritty_terminal::index::Point as AlacrittyPoint;
pub use alacritty_terminal::selection::SelectionType;
pub use alacritty_terminal::term::TermMode;
pub use backend::Command as BackendCommand;
pub use backend::{LinkAction, MouseButton};
pub use terminal::{Command, Event, Terminal};
pub use theme::{ColorPalette, Theme};
pub use view::TerminalView;
