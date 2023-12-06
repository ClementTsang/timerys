use iced::{widget::button, Color, Theme, Vector};

#[derive(Default)]
pub(crate) struct Transparent;

impl button::StyleSheet for Transparent {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::ZERO,
            background: None,
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}
