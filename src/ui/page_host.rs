//! Synchronous native input and page-level redraw scheduling, independent of views.
use crate::app::Message;
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
    widget::{Operation, Tree},
};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, window};
use std::time::Instant;

pub struct PageHost<'a> {
    content: Element<'a, Message>,
    next_tick: Option<Instant>,
    animating: bool,
    interactive: bool,
}
impl<'a> PageHost<'a> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        next_tick: Option<Instant>,
        animating: bool,
    ) -> Self {
        Self {
            content: content.into(),
            next_tick,
            animating,
            interactive: !animating,
        }
    }
    fn native(&self, event: &Event, shell: &mut Shell<'_, Message>) {
        match event {
            Event::Keyboard(_) => {
                shell.publish(Message::Native(event.clone()));
                shell.capture_event();
            }
            Event::Window(window::Event::Focused | window::Event::Unfocused) => {
                shell.publish(Message::Native(event.clone()));
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if self.animating || self.next_tick.is_some_and(|deadline| *now >= deadline) {
                    shell.publish(Message::Native(event.clone()));
                }
                if self.animating {
                    shell.request_redraw();
                } else if let Some(deadline) = self.next_tick.filter(|deadline| *now < *deadline) {
                    shell.request_redraw_at(deadline);
                }
            }
            _ => {}
        }
    }
}
impl Widget<Message, Theme, Renderer> for PageHost<'_> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
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
        // Children still see window events (caret focus/resize), but never keys.
        // Disable pointer actions during the fade, including hover tooltips.
        if !matches!(event, Event::Keyboard(_))
            && (self.interactive || matches!(event, Event::Window(_)))
        {
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
        self.native(event, shell);
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.interactive {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        } else {
            mouse::Interaction::None
        }
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
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        if self.interactive {
            self.content.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
            )
        } else {
            None
        }
    }
}
impl<'a> From<PageHost<'a>> for Element<'a, Message> {
    fn from(host: PageHost<'a>) -> Self {
        Self::new(host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{keyboard, window::RedrawRequest};
    #[test]
    fn native_events_publish_once_and_deadlines_share_the_shell() {
        let now = Instant::now();
        let host = PageHost::new(iced::widget::Space::new(), Some(now), false);
        let mut messages = Vec::new();
        for event in [
            Event::Keyboard(keyboard::Event::ModifiersChanged(
                keyboard::Modifiers::SHIFT,
            )),
            Event::Window(window::Event::Focused),
            Event::Window(window::Event::RedrawRequested(now)),
        ] {
            let before = messages.len();
            host.native(&event, &mut Shell::new(&mut messages));
            assert_eq!(messages.len(), before + 1);
        }
        let future = now + std::time::Duration::from_secs(1);
        let host = PageHost::new(iced::widget::Space::new(), Some(future), false);
        let mut shell = Shell::new(&mut messages);
        host.native(
            &Event::Window(window::Event::RedrawRequested(now)),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::At(future));
        // A caret frame wins over a later countdown deadline.
        shell.request_redraw();
        host.native(
            &Event::Window(window::Event::RedrawRequested(now)),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::NextFrame);
        let host = PageHost::new(iced::widget::Space::new(), None, true);
        host.native(
            &Event::Window(window::Event::RedrawRequested(now)),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::NextFrame);
        messages.clear();
        let mut shell = Shell::new(&mut messages);
        PageHost::new(iced::widget::Space::new(), None, false).native(
            &Event::Window(window::Event::RedrawRequested(now)),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::Wait);
        assert!(shell.is_empty());
    }
}
