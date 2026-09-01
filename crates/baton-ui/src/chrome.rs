//! A7 sizing uses window geometry, fixed chrome constants, and cell metrics
//! only; it never uses a render result.

use iced::Size;

const MIN_PANE_COLUMNS: f32 = 20.0;
const MIN_PANE_ROWS: f32 = 5.0;

/// Fixed shell dimensions used by sizing calculations.
pub(crate) struct Chrome {
    pub(crate) left_dock_width: f32,
    pub(crate) right_dock_collapsed_width: f32,
    pub(crate) centre_gutter_width: f32,
    pub(crate) status_bar_height: f32,
    pub(crate) title_bar_height: f32,
    pub(crate) padding: f32,
}

pub(crate) const CHROME: Chrome = Chrome {
    left_dock_width: 244.0,
    right_dock_collapsed_width: 30.0,
    centre_gutter_width: 26.0,
    status_bar_height: 32.0,
    title_bar_height: 34.0,
    padding: 18.0,
};

/// Returns the centre pane size from window geometry and cell metrics only.
///
/// The result is floored at the minimum pane size so no pane can become
/// smaller than 20 columns by 5 rows. Render output is deliberately not an
/// input: sizing from it would create a resize feedback loop.
#[allow(dead_code)]
pub(crate) fn centre_size(window: Size, cell: Size) -> Size {
    let minimum = minimum_pane(cell);
    let available = window - chrome_size();

    available.max(minimum)
}

/// Returns the minimum window size for the given terminal cell metrics.
#[allow(dead_code)]
pub(crate) fn min_window(cell: Size) -> Size {
    minimum_pane(cell) + chrome_size()
}

fn chrome_size() -> Size {
    Size::new(
        CHROME.left_dock_width
            + CHROME.right_dock_collapsed_width
            + CHROME.centre_gutter_width,
        CHROME.title_bar_height + CHROME.status_bar_height + CHROME.padding,
    )
}

fn minimum_pane(cell: Size) -> Size {
    Size::new(MIN_PANE_COLUMNS * cell.width, MIN_PANE_ROWS * cell.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: Size = Size::new(9.0, 19.0);

    /// A7's property, stated as a test: sizing is a pure function of window
    /// geometry, chrome constants and cell metrics, so feeding it the same
    /// window twice -- or a window derived from its own output -- cannot
    /// grow. The previous attempt grew about 19pt per resize pass.
    #[test]
    fn resizing_converges_instead_of_growing() {
        let window = Size::new(1280.0, 800.0);
        let first = centre_size(window, CELL);
        let second = centre_size(window, CELL);
        assert_eq!(first, second, "same inputs must mean same centre");

        // Simulate the feedback loop A7 forbids: even if a buggy caller fed
        // the centre back in as the window, the centre shrinks by the chrome
        // each pass (until the floor) -- it can never grow.
        let fed_back = centre_size(first, CELL);
        assert!(
            fed_back.width <= first.width && fed_back.height <= first.height,
            "a feedback pass must never grow the centre"
        );
    }

    /// The centre never shrinks below 20 columns by 5 rows, whatever the
    /// window says; the minimum window formula is the same constants.
    #[test]
    fn the_minimum_pane_is_a_floor() {
        let tiny = centre_size(Size::new(100.0, 60.0), CELL);
        assert_eq!(tiny, Size::new(20.0 * CELL.width, 5.0 * CELL.height));

        let min = min_window(CELL);
        assert_eq!(
            centre_size(min, CELL),
            Size::new(20.0 * CELL.width, 5.0 * CELL.height),
            "the minimum window yields exactly the minimum pane"
        );
    }
}
