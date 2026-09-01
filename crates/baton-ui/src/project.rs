use baton_term::Terminal;

use crate::app::App;
use crate::theme::Theme;

/// A flat, display-only snapshot of everything `view()` reads.
///
/// One deliberate exception to "display-only": `terminal` is the widget
/// handle the view embeds, because `TerminalView::show` borrows the widget
/// itself -- it is still the result of this named pure function, which is
/// the property that matters (the view reads nothing else).
pub(crate) struct Projection<'a> {
    pub(crate) terminal: Option<&'a Terminal>,
    pub(crate) theme: &'a Theme,
    pub(crate) terminal_label: &'a str,
    pub(crate) right_dock_hint: &'a str,
    pub(crate) right_dock_collapsed: bool,
}

/// Projects application state one-way and lossy on purpose, in the CQRS sense.
/// Every value read by `view()` comes from this one named pure function.
pub(crate) fn project(app: &App) -> Projection<'_> {
    Projection {
        terminal: app.terminal.as_ref(),
        theme: &app.theme,
        terminal_label: &app.terminal_label,
        right_dock_hint: &app.right_dock_collapsed_hint,
        right_dock_collapsed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `project` is a pure function over `&App`: no window, no renderer, no
    /// IO. Both branches of the one display decision it makes are pinned.
    #[test]
    fn projection_is_computable_without_a_window() {
        let (mut app, _task) =
            App::new("/bin/sh".into(), "t".into(), "l".into(), "h".into());

        let alive = project(&app);
        assert!(alive.terminal.is_some());
        assert_eq!(alive.terminal_label, "l");
        assert!(alive.right_dock_collapsed, "M1 ships it collapsed");

        app.terminal = None;
        let dead = project(&app);
        assert!(
            dead.terminal.is_none(),
            "a lost terminal projects as not alive"
        );
    }
}
