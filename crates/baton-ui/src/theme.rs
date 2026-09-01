use iced::Color;

/// The shell palette used to keep UI colours consistent and replaceable.
pub(crate) struct Theme {
    pub(crate) window_background: Color,
    pub(crate) dock_background: Color,
    pub(crate) dock_border: Color,
    pub(crate) text_primary: Color,
    pub(crate) text_muted: Color,
    #[allow(dead_code)]
    pub(crate) accent: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            window_background: iced::color!(0x1e1e2e),
            dock_background: iced::color!(0x181825),
            dock_border: iced::color!(0x313244),
            text_primary: iced::color!(0xcdd6f4),
            text_muted: iced::color!(0xa6adc8),
            accent: iced::color!(0x89b4fa),
        }
    }
}
