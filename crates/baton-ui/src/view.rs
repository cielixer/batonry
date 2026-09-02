use crate::app::Message;
use crate::chrome::CHROME;
use crate::palette_view;
use crate::project::Projection;
use baton_term::TerminalView;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Length};

/// Assembles the stage-1 shell with both docks present from the start.
/// Reads the projection and nothing else.
pub(crate) fn view<'a>(p: &Projection<'a>) -> Element<'a, Message> {
    let left_dock = dock(
        container(text(p.terminal_label).color(p.theme.text_muted))
            .padding(CHROME.padding),
        CHROME.left_dock_width,
        p.theme.dock_background,
        p.theme.dock_border,
    );

    let centre = match p.terminal {
        Some(terminal) => {
            let window_background = p.theme.window_background;
            let text_primary = p.theme.text_primary;
            container(TerminalView::show(terminal).map(Message::Terminal))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| {
                    container::Style::default()
                        .background(window_background)
                        .color(text_primary)
                })
                .into()
        },
        None => empty_centre(p.theme.window_background, p.theme.text_primary),
    };

    let right_content = if p.right_dock_collapsed {
        container(text(p.right_dock_hint).color(p.theme.text_muted))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
    } else {
        container(Space::new())
    };
    let right_dock = dock(
        right_content,
        CHROME.right_dock_collapsed_width,
        p.theme.dock_background,
        p.theme.dock_border,
    );

    let base = row![left_dock, centre, right_dock]
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    match p.palette.as_ref() {
        Some(palette) => iced::widget::stack![
            base,
            palette_view::overlay(palette.clone(), p.theme, p.recent_tag),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        None => base,
    }
}

fn dock<'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
    background: Color,
    border_color: Color,
) -> Element<'a, Message> {
    container(content)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .style(move |_| {
            container::Style::default()
                .background(background)
                .border(iced::Border::default().color(border_color).width(1))
        })
        .into()
}

fn empty_centre<'a>(
    background: Color,
    text_color: Color,
) -> Element<'a, Message> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| {
            container::Style::default()
                .background(background)
                .color(text_color)
        })
        .into()
}
