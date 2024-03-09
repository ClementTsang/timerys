use iced::{widget::button, Border, Theme, Vector};

#[derive(Default)]
pub(crate) struct Transparent;

impl button::StyleSheet for Transparent {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::ZERO,
            background: None,
            border: Border::with_radius(0.0),
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
