//! Translate iced keyboard events into the action crate's physical-key syntax.

/// Converts an iced physical key and its modifiers to a parsed action
/// keystroke.
///
/// Both enums carry W3C UI Events code names, so the iced debug name is the
/// spelling understood by [`baton_action::Keystroke`]. A name that fails to
/// parse is an exotic key, not an application error.
pub(crate) fn keystroke(
    physical: iced::keyboard::key::Physical,
    modifiers: iced::keyboard::Modifiers,
) -> Option<baton_action::Keystroke> {
    let iced::keyboard::key::Physical::Code(code) = physical else {
        return None;
    };

    let mut spelling = String::new();
    if modifiers.logo() {
        spelling.push_str("meta+");
    }
    if modifiers.shift() {
        spelling.push_str("shift+");
    }
    if modifiers.alt() {
        spelling.push_str("alt+");
    }
    if modifiers.control() {
        spelling.push_str("control+");
    }
    spelling.push_str(&format!("{code:?}"));
    spelling.parse().ok()
}
