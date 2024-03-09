use iced::{theme, widget::text_input, Background, Border, Color, Theme};

use super::DISABLED_TEXT_COLOR;

pub(crate) fn transparent_style() -> theme::TextInput {
    theme::TextInput::Custom(Box::new(Transparent))
}

#[derive(Default)]
pub(crate) struct Transparent;

impl text_input::StyleSheet for Transparent {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::with_radius(0.0),
            icon_color: Color::TRANSPARENT,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn placeholder_color(&self, _: &Self::Style) -> Color {
        DISABLED_TEXT_COLOR
    }

    fn value_color(&self, _: &Self::Style) -> Color {
        Color::BLACK
    }

    fn disabled_color(&self, _: &Self::Style) -> Color {
        DISABLED_TEXT_COLOR
    }

    fn selection_color(&self, _: &Self::Style) -> Color {
        Color::BLACK
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
}
