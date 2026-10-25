use super::{caret::Caret, theme};
use iced::advanced::Renderer as _;
use iced::advanced::text::{Paragraph, Renderer as _};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, renderer, text, widget::tree,
};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, alignment};
use typerr::{
    engine::{CharacterState, TypingSession},
    layout::{TextLayout, Viewport},
};

type NativeParagraph = <Renderer as text::Renderer>::Paragraph;

pub struct TypingPanel<'a, Message> {
    session: &'a TypingSession,
    revision: u64,
    on_event: fn(Event) -> Message,
}

#[derive(Default)]
struct State {
    layout: Option<TextLayout>,
    target: Vec<char>,
    viewport: Viewport,
    size: Size,
    revision: u64,
    caret: Caret,
    wheel_remainder: f32,
}

fn text_definition(content: &str, bounds: Size) -> text::Text<&str> {
    text::Text {
        content,
        bounds,
        size: theme::FONT_SIZE.into(),
        line_height: text::LineHeight::Absolute(theme::LINE_HEIGHT.into()),
        font: theme::FONT,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    }
}

impl<'a, Message> TypingPanel<'a, Message> {
    pub fn new(session: &'a TypingSession, revision: u64, on_event: fn(Event) -> Message) -> Self {
        Self {
            session,
            revision,
            on_event,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for TypingPanel<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let state = tree.state.downcast_mut::<State>();
        let resized =
            state.layout.is_none() || state.size != size || state.target != self.session.target();
        if resized {
            // Query the renderer's paragraph implementation after startup font registration.
            // A single monospaced ASCII glyph has the same advance as each target cell.
            let metrics = NativeParagraph::with_text(text_definition(
                "m",
                Size::new(f32::INFINITY, theme::LINE_HEIGHT),
            ));
            let advance = metrics.min_width();
            let measured = TextLayout::new(
                self.session.target(),
                (size.width - 2.0 * theme::INSET).max(0.0),
                advance,
                theme::LINE_HEIGHT,
            );
            state.viewport.resize(
                &measured,
                size.height - 2.0 * theme::VERTICAL_INSET,
                self.session.position(),
            );
            state.layout = Some(measured);
            state.target = self.session.target().to_vec();
            state.size = size;
        }
        let measured = state.layout.as_ref().expect("layout was initialized");
        if state.revision != self.revision {
            state.viewport.follow(measured, self.session.position());
            state.revision = self.revision;
            state.wheel_remainder = 0.0;
        }
        state.caret.snap_to(measured.caret(self.session.position()));
        layout::Node::new(size)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // Keyboard input must not cross the bounded asynchronous event-subscription
        // queue: a busy frame can otherwise drop a burst of native keystrokes.
        if matches!(
            event,
            Event::Keyboard(_)
                | Event::Window(iced::window::Event::Focused | iced::window::Event::Unfocused)
        ) {
            shell.publish((self.on_event)(event.clone()));
            if matches!(event, Event::Keyboard(_)) {
                shell.capture_event();
            }
            return;
        }
        if !cursor.is_over(layout.bounds()) {
            return;
        }
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            let state = tree.state.downcast_mut::<State>();
            let amount = match delta {
                mouse::ScrollDelta::Lines { y, .. } => -*y,
                mouse::ScrollDelta::Pixels { y, .. } => -*y / theme::LINE_HEIGHT,
            };
            state.wheel_remainder += amount;
            let lines = state.wheel_remainder.trunc() as isize;
            state.wheel_remainder -= lines as f32;
            if let Some(measured) = &state.layout {
                state.viewport.scroll(lines, measured.lines.len());
                shell.request_redraw();
                shell.capture_event();
            }
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        // Colored rectangles in the Outline reference mark layout areas only.
        // The typing panel is transparent, without a decorative border.
        let state = tree.state.downcast_ref::<State>();
        let Some(measured) = &state.layout else {
            return;
        };
        let inner_width = (bounds.width - 2.0 * theme::INSET).max(0.0);
        let inner_height = (bounds.height - 2.0 * theme::VERTICAL_INSET).max(0.0);
        let offset = state
            .viewport
            .content_origin(measured, inner_width, inner_height);
        let origin = Point::new(
            bounds.x + theme::INSET + offset.x,
            bounds.y + theme::VERTICAL_INSET + offset.y,
        );
        // Keep the monitor aligned with the measured, centered text block,
        // including when only the final line remains or the window reflows.
        let counter = format!(
            "{}/{}",
            self.session.completed_words(),
            self.session.word_count()
        );
        renderer.fill_text(
            text_definition(&counter, Size::new(inner_width, theme::LINE_HEIGHT))
                .with_content(counter.clone()),
            Point::new(origin.x, origin.y - 30.0),
            theme::BLUE,
            *_viewport,
        );
        let clip = Rectangle {
            x: origin.x,
            y: origin.y,
            width: (inner_width - offset.x).max(0.0),
            height: state.viewport.visible_lines(measured.lines.len()).len() as f32
                * theme::LINE_HEIGHT,
        };
        let active = measured.active_line(self.session.position());
        renderer.with_layer(clip, |renderer| {
            for line_index in state.viewport.visible_lines(measured.lines.len()) {
                let range = &measured.lines[line_index];
                let y =
                    origin.y + (line_index - state.viewport.top_line) as f32 * theme::LINE_HEIGHT;
                let opacity = state
                    .viewport
                    .opacity(line_index, active, measured.lines.len());
                for index in range.clone() {
                    let character = self.session.target()[index];
                    let status = self
                        .session
                        .character_state(index)
                        .expect("layout indexes target");
                    let mut color = match status {
                        CharacterState::Pending => theme::PENDING,
                        CharacterState::Correct => theme::CORRECT,
                        CharacterState::Incorrect => theme::INCORRECT,
                    };
                    color.a *= opacity;
                    let x = origin.x + (index - range.start) as f32 * measured.advance;
                    if character == ' ' {
                        if status == CharacterState::Incorrect {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle {
                                        x,
                                        y: y + theme::LINE_HEIGHT - 3.0,
                                        width: measured.advance,
                                        height: 2.0,
                                    },
                                    ..Default::default()
                                },
                                color,
                            );
                        }
                    } else {
                        renderer.fill_text(
                            text_definition(
                                &character.to_string(),
                                Size::new(measured.advance, theme::LINE_HEIGHT),
                            )
                            .with_content(character.to_string()),
                            Point::new(x, y),
                            color,
                            clip,
                        );
                    }
                }
            }
            if state
                .viewport
                .visible_lines(measured.lines.len())
                .contains(&active)
            {
                let caret = state.caret.rendered;
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: origin.x + caret.x,
                            y: origin.y + caret.y
                                - state.viewport.top_line as f32 * theme::LINE_HEIGHT,
                            width: 2.0,
                            height: theme::LINE_HEIGHT,
                        },
                        ..Default::default()
                    },
                    theme::CYAN,
                );
            }
        });
    }
}

impl<'a, Message: 'a> From<TypingPanel<'a, Message>> for Element<'a, Message> {
    fn from(panel: TypingPanel<'a, Message>) -> Self {
        Self::new(panel)
    }
}
