//! A text input for time.

use iced::{
    advanced::{renderer, Widget},
    Length, Size,
};

pub(crate) struct TimeInput {}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for TimeInput
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        todo!()
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        todo!()
    }
}
