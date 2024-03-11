use iced::{
    advanced::{
        layout, mouse, overlay, renderer,
        widget::{tree, Operation, Tree},
        Clipboard, Layout, Shell, Widget,
    },
    event,
    keyboard::{self, key::Named},
    widget::{container::StyleSheet, Container},
    Element, Event, Length, Rectangle, Renderer, Size, Vector,
};

/// A wrapper container that will intercept any numerical inputs (or backspaces).
pub(crate) struct NumInputContainer<'a, Message, Theme = crate::Theme>
where
    Theme: StyleSheet,
{
    container: Container<'a, Message, Theme, iced::Renderer>,
    on_num: Box<dyn Fn(u32) -> Message + 'a>,
    on_backspace: Box<dyn Fn() -> Message + 'a>,
    ignore_events: bool,
}

impl<'a, Message, Theme> NumInputContainer<'a, Message, Theme>
where
    Theme: StyleSheet,
{
    /// Creates an empty [`Container`].
    pub(crate) fn new(
        container: Container<'a, Message, Theme, iced::Renderer>,
        on_num: Box<dyn Fn(u32) -> Message + 'a>,
        on_backspace: Box<dyn Fn() -> Message + 'a>,
        ignore_events: bool,
    ) -> Self {
        NumInputContainer {
            container,
            on_num,
            on_backspace,
            ignore_events,
        }
    }
}

impl<'a, Message, Theme> Widget<Message, Theme, iced::Renderer>
    for NumInputContainer<'a, Message, Theme>
where
    Theme: StyleSheet,
{
    fn tag(&self) -> tree::Tag {
        self.container.tag()
    }

    fn state(&self) -> tree::State {
        self.container.state()
    }

    fn children(&self) -> Vec<Tree> {
        self.container.children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.container.diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.container.size()
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.container.layout(tree, renderer, limits)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        self.container.operate(tree, layout, renderer, operation)
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        if !self.ignore_events {
            match event {
                Event::Keyboard(ref event) => match event {
                    keyboard::Event::KeyPressed { key, text, .. } => {
                        match key {
                            keyboard::Key::Named(Named::Backspace) => {
                                shell.publish((self.on_backspace)());
                                return event::Status::Captured;
                            }
                            keyboard::Key::Character(c) => {
                                if let Ok(digit) = c.parse::<u32>() {
                                    shell.publish((self.on_num)(digit));
                                    return event::Status::Captured;
                                }
                            }
                            _ => {}
                        }

                        // Not sure if this works.
                        if let Some(text) = text {
                            if let Some(c) = text.chars().next().filter(|c| !c.is_control()) {
                                if let Some(num) = c.to_digit(10) {
                                    shell.publish((self.on_num)(num));
                                    return event::Status::Captured;
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        self.container.on_event(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.container
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.container
            .draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.container.overlay(tree, layout, renderer, translation)
    }
}

impl<'a, Message, Theme> From<NumInputContainer<'a, Message, Theme>>
    for Element<'a, Message, Theme, iced::Renderer>
where
    Message: 'a,
    Theme: 'a + StyleSheet,
{
    fn from(
        container: NumInputContainer<'a, Message, Theme>,
    ) -> Element<'a, Message, Theme, iced::Renderer> {
        Element::new(container)
    }
}
