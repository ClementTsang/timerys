use iced::Color;

pub(crate) mod button;
pub(crate) mod text_input;

pub(crate) const DISABLED_TEXT_COLOR: Color = Color {
    a: Color::BLACK.a * 0.5,
    ..Color::BLACK
};

pub(crate) const DEFAULT_TEXT_COLOR: Color = Color::BLACK;
