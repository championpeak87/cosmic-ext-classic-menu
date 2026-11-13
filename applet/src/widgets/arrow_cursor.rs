// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::advanced::layout::{self, Layout};
use cosmic::iced::advanced::widget::{Operation, Tree};
use cosmic::iced::advanced::{Clipboard, Shell, Widget, overlay, renderer};
use cosmic::iced::{Event, Length, Rectangle, Size, Vector, mouse};
use cosmic::{Element, Renderer, Theme};

/// Shows the regular arrow instead of the hand over its content and the
/// overlays (such as context menus) it opens.
///
/// libcosmic's buttons always ask for the hand, and `mouse_area`'s
/// `interaction` only applies where the content asks for no cursor, so it
/// cannot override them.
pub struct ArrowCursor<'a, Message> {
    content: Element<'a, Message>,
}

impl<'a, Message> ArrowCursor<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

fn arrow(interaction: mouse::Interaction) -> mouse::Interaction {
    match interaction {
        mouse::Interaction::Pointer => mouse::Interaction::Idle,
        other => other,
    }
}

impl<Message> Widget<Message, Theme, Renderer> for ArrowCursor<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        arrow(self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        ))
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, viewport, translation)
            .map(ArrowOverlay::wrap)
    }

    fn drag_destinations(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        dnd_rectangles: &mut cosmic::iced::advanced::clipboard::DndDestinationRectangles,
    ) {
        self.content.as_widget().drag_destinations(
            &tree.children[0],
            layout,
            renderer,
            dnd_rectangles,
        );
    }

    fn a11y_nodes(
        &self,
        layout: Layout<'_>,
        tree: &Tree,
        cursor: mouse::Cursor,
    ) -> iced_accessibility::A11yTree {
        self.content
            .as_widget()
            .a11y_nodes(layout, &tree.children[0], cursor)
    }
}

impl<'a, Message: 'a> From<ArrowCursor<'a, Message>> for Element<'a, Message> {
    fn from(arrow_cursor: ArrowCursor<'a, Message>) -> Self {
        Element::new(arrow_cursor)
    }
}

/// [`ArrowCursor`] for an overlay and the overlays nested in it.
struct ArrowOverlay<'a, Message> {
    content: overlay::Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message: 'a> ArrowOverlay<'a, Message> {
    fn wrap(
        content: overlay::Element<'a, Message, Theme, Renderer>,
    ) -> overlay::Element<'a, Message, Theme, Renderer> {
        overlay::Element::new(Box::new(Self { content }))
    }
}

impl<'a, Message: 'a> overlay::Overlay<Message, Theme, Renderer> for ArrowOverlay<'a, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        self.content.as_overlay_mut().layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content
            .as_overlay()
            .draw(renderer, theme, style, layout, cursor);
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_overlay_mut()
            .operate(layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        self.content
            .as_overlay_mut()
            .update(event, layout, cursor, renderer, clipboard, shell);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        arrow(
            self.content
                .as_overlay()
                .mouse_interaction(layout, cursor, renderer),
        )
    }

    fn overlay<'b>(
        &'b mut self,
        layout: Layout<'b>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_overlay_mut()
            .overlay(layout, renderer)
            .map(ArrowOverlay::wrap)
    }

    fn index(&self) -> f32 {
        self.content.as_overlay().index()
    }
}
