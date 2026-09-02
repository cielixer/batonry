//! The command palette overlay, rendered from the display projection.

use crate::app::{Message, PALETTE_INPUT_ID};
use crate::project::{PaletteProjection, PaletteRow};
use crate::theme::Theme;
use iced::widget::{column, container, mouse_area, row, text, text_input};
use iced::{Element, Length, alignment};

/// Renders the palette over the unchanged shell without covering it with a scrim.
pub(crate) fn overlay<'a>(
    palette: PaletteProjection<'a>,
    theme: &Theme,
    recent_tag: &str,
) -> Element<'a, Message> {
    let input = text_input(palette.placeholder, palette.query)
        .id(PALETTE_INPUT_ID.clone())
        .on_input(Message::PaletteInput);
    // Enter deliberately has no text-input submit handler: it must fall
    // through to the keymap so palette.confirm owns dispatch.
    let rows = column(
        palette
            .rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| result_row(index, row, theme, recent_tag)),
    )
    .spacing(4)
    .width(Length::Fill);
    let panel_background = theme.dock_background;
    let panel_border = theme.dock_border;
    let panel = container(column![input, rows].spacing(8))
        .width(Length::Fixed(560.0))
        .max_height(420.0)
        .padding(12.0)
        .clip(true)
        .style(move |_| {
            container::Style::default()
                .background(panel_background)
                .border(iced::Border::default().color(panel_border).width(1))
        });

    container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24.0)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Top)
        .into()
}

fn result_row<'a>(
    index: usize,
    palette_row: PaletteRow<'a>,
    theme: &Theme,
    recent_tag: &str,
) -> Element<'a, Message> {
    let mut content =
        row![text(palette_row.label.to_owned()).color(theme.text_primary)]
            .spacing(8)
            .width(Length::Fill);
    if palette_row.recent {
        content =
            content.push(text(recent_tag.to_owned()).color(theme.text_muted));
    }
    if palette_row.selected
        && let Some(reason) = palette_row.reason
    {
        content = content.push(text(reason.to_owned()).color(theme.text_muted));
    }

    let selected = palette_row.selected;
    let accent = theme.accent;
    let row = container(
        mouse_area(content)
            .on_enter(Message::PaletteHover(index))
            .on_press(Message::PaletteConfirmRow(index)),
    )
    .width(Length::Fill)
    .padding([6, 10])
    .style(move |_| {
        if selected {
            container::Style::default().background(accent.scale_alpha(0.18))
        } else {
            container::Style::default()
        }
    });
    row.into()
}
