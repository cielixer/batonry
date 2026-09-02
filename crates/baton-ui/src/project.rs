use baton_term::Terminal;

use crate::app::App;
use crate::search;
use crate::theme::Theme;

/// The palette data needed by the overlay, borrowed from application state.
#[derive(Clone)]
pub(crate) struct PaletteProjection<'a> {
    pub(crate) query: &'a str,
    pub(crate) placeholder: &'a str,
    pub(crate) rows: Vec<PaletteRow<'a>>,
}

/// One palette row projected for rendering and interaction.
#[derive(Clone)]
pub(crate) struct PaletteRow<'a> {
    pub(crate) label: &'a str,
    pub(crate) selected: bool,
    pub(crate) recent: bool,
    pub(crate) reason: Option<&'static str>,
}

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
    pub(crate) palette: Option<PaletteProjection<'a>>,
    pub(crate) recent_tag: &'a str,
}

/// Projects application state one-way and lossy on purpose, in the CQRS sense.
/// Every value read by `view()` comes from this one named pure function.
pub(crate) fn project(app: &App) -> Projection<'_> {
    let palette = app.palette.as_ref().map(|palette| {
        let rows = search::palette_results(&app.registry, &palette.query)
            .into_iter()
            .enumerate()
            .map(|(index, result)| PaletteRow {
                label: result.label,
                selected: index == palette.selected,
                recent: palette.query.is_empty()
                    && app.recents.iter().any(|recent| recent == result.id),
                reason: match result.availability {
                    search::Availability::Ready => None,
                    // Availability is a crate-internal fact, so its reason is
                    // intentionally not another main-injected UI string.
                    search::Availability::Unavailable(reason) => Some(reason),
                },
            })
            .collect();
        PaletteProjection {
            query: &palette.query,
            placeholder: &app.palette_placeholder,
            rows,
        }
    });

    Projection {
        terminal: app.terminal.as_ref(),
        theme: &app.theme,
        terminal_label: &app.terminal_label,
        right_dock_hint: &app.right_dock_collapsed_hint,
        right_dock_collapsed: true,
        palette,
        recent_tag: &app.recent_tag,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `project` is a pure function over `&App`: no window, no renderer, no
    /// IO. Both branches of the one display decision it makes are pinned.
    #[test]
    fn projection_is_computable_without_a_window() {
        let (mut app, _task) = App::new(
            "/bin/sh".into(),
            "t".into(),
            "l".into(),
            "h".into(),
            "Type a command...".into(),
            "recent".into(),
            None,
        );

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
