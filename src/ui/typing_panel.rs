use super::{caret::Caret, sizing::Preset, theme};
use crate::settings::CaretStyle;
use iced::advanced::Renderer as _;
use iced::advanced::text::{Paragraph, Renderer as _};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, renderer, text, widget::tree,
};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, alignment};
use std::time::Instant;
use typerr::{
    engine::{CharacterState, TypingSession},
    layout::{CaretTarget, TextLayout, Viewport},
};

type NativeParagraph = <Renderer as text::Renderer>::Paragraph;

pub struct TypingPanel<'a> {
    session: &'a TypingSession,
    revision: u64,
    generation: u64,
    monitor: String,
    opacity: f32,
    interactive: bool,
    settings: crate::settings::Settings,
    preset: Preset,
}

#[derive(Default)]
struct State {
    layout: Option<TextLayout>,
    target: Vec<char>,
    viewport: Viewport,
    size: Size,
    revision: u64,
    generation: u64,
    caret: Caret,
    manually_scrolled: bool,
    unfocused: bool,
    wheel_remainder: f32,
    font_size: f32,
    preset: Preset,
}

impl State {
    fn content_origin(&self) -> CaretTarget {
        let measured = self.layout.as_ref().expect("layout was initialized");
        let offset = self.viewport.content_origin(
            measured,
            (self.size.width - 2.0 * (theme::INSET * self.preset.scale())).max(0.0),
            (self.size.height - 2.0 * (theme::VERTICAL_INSET * self.preset.scale())).max(0.0),
        );
        CaretTarget {
            x: (theme::INSET * self.preset.scale()) + offset.x,
            y: (theme::VERTICAL_INSET * self.preset.scale()) + offset.y,
        }
    }

    fn caret_target(&self, position: usize) -> CaretTarget {
        let measured = self.layout.as_ref().expect("layout was initialized");
        let target = measured.caret(position);
        let origin = self.content_origin();
        CaretTarget {
            x: origin.x + target.x,
            y: origin.y + target.y - self.viewport.top_line as f32 * measured.line_height,
        }
    }

    fn sync_caret(
        &mut self,
        position: usize,
        revision: u64,
        generation: u64,
        geometry_changed: bool,
        now: Instant,
    ) {
        let reset = geometry_changed || self.generation != generation;
        let edited = self.revision != revision;
        if !reset && !edited {
            return;
        }
        let snap = reset || self.manually_scrolled || self.unfocused;
        self.follow(position);
        let target = self.caret_target(position);
        if snap {
            self.caret.snap_to(target);
        } else {
            self.caret.retarget(target, now);
        }
        self.revision = revision;
        self.generation = generation;
    }

    fn follow(&mut self, position: usize) {
        if let Some(measured) = &self.layout {
            self.viewport.follow(measured, position);
        }
        self.manually_scrolled = false;
        self.wheel_remainder = 0.0;
    }

    fn scroll(&mut self, lines: isize, position: usize) {
        if let Some(measured) = &self.layout {
            let previous_top = self.viewport.top_line;
            self.viewport.scroll(lines, measured.lines.len());
            if self.viewport.top_line != previous_top {
                self.manually_scrolled = true;
                self.caret.snap_to(self.caret_target(position));
            }
        }
    }
}

fn text_definition(
    content: &str,
    bounds: Size,
    settings: crate::settings::Settings,
) -> text::Text<&str> {
    text::Text {
        content,
        bounds,
        size: settings.font_size.into(),
        line_height: text::LineHeight::Absolute(settings.line_height().into()),
        font: theme::FONT,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    }
}

impl<'a> TypingPanel<'a> {
    pub fn new(
        session: &'a TypingSession,
        revision: u64,
        generation: u64,
        monitor: String,
        settings: crate::settings::Settings,
    ) -> Self {
        Self {
            session,
            revision,
            generation,
            monitor,
            opacity: 1.0,
            interactive: true,
            settings,
            preset: Preset::Design,
        }
    }

    pub fn preset(mut self, preset: Preset) -> Self {
        self.preset = preset;
        self.settings.font_size *= preset.scale();
        self
    }

    pub fn presentation(mut self, opacity: f32, interactive: bool) -> Self {
        self.opacity = opacity;
        self.interactive = interactive;
        self
    }

    fn window_event<Message>(
        &self,
        state: &mut State,
        event: &iced::window::Event,
        shell: &mut Shell<'_, Message>,
    ) {
        use iced::window::Event;
        match event {
            Event::RedrawRequested(now) => {
                if state.caret.advance(*now) {
                    shell.request_redraw();
                }
            }
            Event::Unfocused => {
                state.unfocused = true;
                state.caret.snap_to(state.caret.target);
                shell.request_redraw();
            }
            Event::Focused => state.unfocused = false,
            Event::Resized(_) | Event::Rescaled(_) => {
                // A window resize can leave a capped panel's size unchanged.
                // It must still end inspection/motion and restore active following.
                state.follow(self.session.position());
                if state.layout.is_some() {
                    state
                        .caret
                        .snap_to(state.caret_target(self.session.position()));
                }
                shell.request_redraw();
            }
            _ => {}
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for TypingPanel<'_> {
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
        let resized = state.layout.is_none()
            || state.size != size
            || state.target != self.session.target()
            || state.generation != self.generation
            || state.font_size != self.settings.font_size
            || state.preset != self.preset;
        if resized {
            // Query the renderer's paragraph implementation after startup font registration.
            // A single monospaced ASCII glyph has the same advance as each target cell.
            let metrics = NativeParagraph::with_text(text_definition(
                "m",
                Size::new(f32::INFINITY, self.settings.line_height()),
                self.settings,
            ));
            let advance = metrics.min_width();
            let measured = TextLayout::new(
                self.session.target(),
                (size.width - 2.0 * (theme::INSET * self.preset.scale())).max(0.0),
                advance,
                self.settings.line_height(),
            );
            state.viewport.resize(
                &measured,
                size.height - 2.0 * (theme::VERTICAL_INSET * self.preset.scale()),
                self.session.position(),
            );
            state.layout = Some(measured);
            state.target = self.session.target().to_vec();
            state.size = size;
            state.font_size = self.settings.font_size;
            state.preset = self.preset;
        }
        state.sync_caret(
            self.session.position(),
            self.revision,
            self.generation,
            resized,
            Instant::now(),
        );
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
        if let Event::Window(event) = event {
            self.window_event(tree.state.downcast_mut::<State>(), event, shell);
        }
        // PageHost forwards native keys/focus exactly once on every page.
        if !self.interactive {
            return;
        }
        if !cursor.is_over(layout.bounds()) {
            return;
        }
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            let state = tree.state.downcast_mut::<State>();
            let amount = match delta {
                mouse::ScrollDelta::Lines { y, .. } => -*y,
                mouse::ScrollDelta::Pixels { y, .. } => -*y / self.settings.line_height(),
            };
            state.wheel_remainder += amount;
            let lines = state.wheel_remainder.trunc() as isize;
            state.wheel_remainder -= lines as f32;
            state.scroll(lines, self.session.position());
            shell.request_redraw();
            shell.capture_event();
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
        let inner_width = (bounds.width - 2.0 * (theme::INSET * self.preset.scale())).max(0.0);
        let inner_height =
            (bounds.height - 2.0 * (theme::VERTICAL_INSET * self.preset.scale())).max(0.0);
        let offset = state.content_origin();
        let origin = Point::new(bounds.x + offset.x, bounds.y + offset.y);
        // Keep the monitor aligned with the measured, centered text block,
        // including when only the final line remains or the window reflows.
        renderer.fill_text(
            text_definition(
                &self.monitor,
                Size::new(inner_width, theme::LINE_HEIGHT * self.preset.scale()),
                crate::settings::Settings {
                    font_size: 16. * self.preset.scale(),
                    ..Default::default()
                },
            )
            .with_content(self.monitor.clone()),
            Point::new(origin.x, origin.y - 30.0 * self.preset.scale()),
            theme::alpha(theme::palette().blue, self.opacity),
            *_viewport,
        );
        let clip = Rectangle {
            x: origin.x,
            y: origin.y,
            width: (bounds.width - (theme::INSET * self.preset.scale()) - offset.x).max(0.0),
            height: state.viewport.visible_lines(measured.lines.len()).len() as f32
                * self.settings.line_height(),
        };
        let active = measured.active_line(self.session.position());
        renderer.with_layer(clip, |renderer| {
            for line_index in state.viewport.visible_lines(measured.lines.len()) {
                let range = &measured.lines[line_index];
                let y = origin.y
                    + (line_index - state.viewport.top_line) as f32 * self.settings.line_height();
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
                        CharacterState::Pending => theme::palette().pending,
                        CharacterState::Correct => theme::palette().correct,
                        CharacterState::Incorrect => theme::palette().incorrect,
                    };
                    color.a *= opacity * self.opacity;
                    let x = origin.x + (index - range.start) as f32 * measured.advance;
                    if character == ' ' {
                        if status == CharacterState::Incorrect {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle {
                                        x,
                                        y: y + self.settings.line_height()
                                            - 3.0 * self.preset.scale(),
                                        width: measured.advance,
                                        height: 2.0 * self.preset.scale(),
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
                                Size::new(measured.advance, self.settings.line_height()),
                                self.settings,
                            )
                            .with_content(character.to_string()),
                            Point::new(x, y),
                            color,
                            clip,
                        );
                    }
                }
            }
        });
        if state
            .viewport
            .visible_lines(measured.lines.len())
            .contains(&active)
        {
            // Retain the whole padded panel during a return movement, even if
            // the newly centered text block is narrower than the previous one.
            let caret_clip = Rectangle {
                x: bounds.x + (theme::INSET * self.preset.scale()),
                y: bounds.y + (theme::VERTICAL_INSET * self.preset.scale()),
                width: inner_width,
                height: inner_height,
            };
            renderer.with_layer(caret_clip, |renderer| {
                let caret = state.caret.rendered;
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x + caret.x,
                            y: bounds.y
                                + caret.y
                                + if self.settings.caret_style == CaretStyle::Underline {
                                    self.settings.line_height() - 2.0 * self.preset.scale()
                                } else {
                                    0.0
                                },
                            width: if self.settings.caret_style == CaretStyle::Line {
                                2.0 * self.preset.scale()
                            } else {
                                measured.advance
                            },
                            height: if self.settings.caret_style == CaretStyle::Underline {
                                2.0 * self.preset.scale()
                            } else {
                                self.settings.line_height()
                            },
                        },
                        ..Default::default()
                    },
                    theme::alpha(theme::palette().cyan, self.opacity),
                );
                if self.settings.caret_style == CaretStyle::Block {
                    // Redraw intersecting glyph portions in the background color.
                    // Clipping to the animated block keeps text readable even
                    // while the caret moves between cells or across rows.
                    let block = Rectangle {
                        x: bounds.x + caret.x,
                        y: bounds.y + caret.y,
                        width: measured.advance,
                        height: self.settings.line_height(),
                    };
                    if let Some(block_clip) = block.intersection(&clip) {
                        renderer.with_layer(block_clip, |renderer| {
                            for line_index in state.viewport.visible_lines(measured.lines.len()) {
                                let range = &measured.lines[line_index];
                                let y = origin.y
                                    + (line_index - state.viewport.top_line) as f32
                                        * self.settings.line_height();
                                for index in range.clone() {
                                    let character = self.session.target()[index];
                                    let x =
                                        origin.x + (index - range.start) as f32 * measured.advance;
                                    let cell = Rectangle {
                                        x,
                                        y,
                                        width: measured.advance,
                                        height: self.settings.line_height(),
                                    };
                                    if character != ' ' && cell.intersects(&block_clip) {
                                        renderer.fill_text(
                                            text_definition(
                                                &character.to_string(),
                                                Size::new(
                                                    measured.advance,
                                                    self.settings.line_height(),
                                                ),
                                                self.settings,
                                            )
                                            .with_content(character.to_string()),
                                            Point::new(x, y),
                                            theme::alpha(theme::palette().background, self.opacity),
                                            block_clip,
                                        );
                                    }
                                }
                            }
                        });
                    }
                }
            });
        }
    }
}

impl<'a, Message: 'a> From<TypingPanel<'a>> for Element<'a, Message> {
    fn from(panel: TypingPanel<'a>) -> Self {
        Self::new(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::window::{Event as WindowEvent, RedrawRequest};
    use std::time::Duration;

    #[test]
    fn rendered_cursor_shapes_follow_measured_cells_at_all_font_sizes() {
        use iced::advanced::renderer::Headless;
        use std::{
            future::Future,
            task::{Context, Poll, Waker},
        };
        let mut future = std::pin::pin!(<Renderer as Headless>::new(
            theme::FONT,
            16.into(),
            Some("tiny-skia")
        ));
        let Poll::Ready(Some(mut renderer)) = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        else {
            panic!("software renderer unavailable")
        };
        for font_size in [8., 16., 24., 48.] {
            for caret_style in [CaretStyle::Line, CaretStyle::Block, CaretStyle::Underline] {
                let session = TypingSession::new(" ");
                let mut panel = TypingPanel::new(
                    &session,
                    0,
                    0,
                    String::new(),
                    crate::settings::Settings {
                        font_size,
                        caret_style,
                        ..Default::default()
                    },
                );
                let mut tree = tree::Tree::new(&panel as &dyn Widget<(), Theme, Renderer>);
                let size = Size::new(640., 300.);
                let node = <TypingPanel<'_> as Widget<(), Theme, Renderer>>::layout(
                    &mut panel,
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(size, size),
                );
                let state = tree.state.downcast_ref::<State>();
                let caret = state.caret.rendered;
                let advance = state.layout.as_ref().unwrap().advance;
                let expected_width = if caret_style == CaretStyle::Line {
                    2.
                } else {
                    advance
                };
                let expected_height = if caret_style == CaretStyle::Underline {
                    2.
                } else {
                    font_size * 1.25
                };
                let expected_y = caret.y
                    + if caret_style == CaretStyle::Underline {
                        font_size * 1.25 - 2.
                    } else {
                        0.
                    };
                renderer.reset(Rectangle::with_size(size));
                <TypingPanel<'_> as Widget<(), Theme, Renderer>>::draw(
                    &panel,
                    &tree,
                    &mut renderer,
                    &Theme::Dark,
                    &renderer::Style {
                        text_color: iced::Color::WHITE,
                    },
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &Rectangle::with_size(size),
                );
                let pixels = renderer.screenshot(Size::new(640, 300), 1., iced::Color::BLACK);
                let mut colored = Vec::new();
                for y in 0..300usize {
                    for x in 0..640usize {
                        let offset = (y * 640 + x) * 4;
                        if pixels[offset..offset + 3]
                            .iter()
                            .any(|channel| *channel > 0)
                        {
                            colored.push((x, y));
                        }
                    }
                }
                let left = colored.iter().map(|p| p.0).min().unwrap() as f32;
                let right = colored.iter().map(|p| p.0).max().unwrap() as f32 + 1.;
                let top = colored.iter().map(|p| p.1).min().unwrap() as f32;
                let bottom = colored.iter().map(|p| p.1).max().unwrap() as f32 + 1.;
                assert!(
                    (left - caret.x).abs() <= 1. && (right - caret.x - expected_width).abs() <= 1.
                );
                assert!(
                    (top - expected_y).abs() <= 1.
                        && (bottom - expected_y - expected_height).abs() <= 1.
                );
            }
        }
    }

    const MOVEMENT: Duration = Duration::from_millis(100);

    #[test]
    fn preset_changes_keep_four_rows_and_follow_the_same_logical_position() {
        let renderer = iced::futures::executor::block_on(
            <Renderer as iced::advanced::renderer::Headless>::new(
                theme::FONT,
                16.into(),
                Some("tiny-skia"),
            ),
        )
        .unwrap();
        let mut session = TypingSession::new(
            std::iter::repeat_n("sample", 500)
                .collect::<Vec<_>>()
                .join(" "),
        );
        for _ in 0..110 {
            session.input('x');
        }
        for font_size in [8., 16., 16.5, 24., 48.] {
            let settings = crate::settings::Settings {
                font_size,
                ..Default::default()
            };
            let mut tree = tree::Tree::empty();
            for preset in [
                Preset::Design,
                Preset::Medium,
                Preset::Large,
                Preset::ExtraLarge,
                Preset::Design,
            ] {
                let mut panel =
                    TypingPanel::new(&session, 110, 0, "0/50".into(), settings).preset(preset);
                let widget: &dyn Widget<(), Theme, Renderer> = &panel;
                tree.diff(widget);
                let size = Size::new(
                    640. * preset.scale(),
                    (theme::typing_height(settings.line_height()) * preset.scale()).ceil(),
                );
                <TypingPanel<'_> as Widget<(), Theme, Renderer>>::layout(
                    &mut panel,
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(size, size),
                );
                let state = tree.state.downcast_ref::<State>();
                let measured = state.layout.as_ref().unwrap();
                assert_eq!(state.viewport.visible_lines(measured.lines.len()).len(), 4);
                assert_eq!(
                    state.viewport.top_line,
                    measured.active_line(session.position())
                );
                assert_eq!(state.caret.rendered, state.caret_target(session.position()));
                assert_eq!(session.position(), 110);
            }
        }
    }

    fn geometry(state: &mut State, session: &TypingSession, size: Size, font_size: f32) {
        // Synthetic measured metrics; production still obtains these from Iced.
        let measured = TextLayout::new(session.target(), size.width - 40.0, 10.0, font_size * 1.25);
        state
            .viewport
            .resize(&measured, size.height - 30.0, session.position());
        state.layout = Some(measured);
        state.target = session.target().to_vec();
        state.size = size;
        state.font_size = font_size;
    }

    fn state(session: &TypingSession, now: Instant) -> State {
        let mut state = State::default();
        geometry(&mut state, session, Size::new(122.0, 110.0), 16.0);
        state.sync_caret(session.position(), 0, 0, true, now);
        state
    }

    fn panel(session: &TypingSession) -> TypingPanel<'_> {
        TypingPanel::new(session, 0, 0, String::new(), Default::default())
    }

    #[test]
    fn wraps_and_backspace_animate_in_panel_coordinates_after_recentering() {
        let now = Instant::now();
        let mut session = TypingSession::new("one two three");
        let mut state = state(&session, now);
        for c in "one two".chars() {
            session.input(c);
        }
        state.sync_caret(session.position(), 1, 0, false, now);
        state.caret.advance(now + MOVEMENT);
        let before = state.caret.rendered;
        assert_eq!(before, CaretTarget { x: 90.0, y: 35.0 });
        session.input('x'); // A wrong separator has the same wrapping behavior.
        state.sync_caret(session.position(), 2, 0, false, now + MOVEMENT);
        assert_eq!(state.viewport.top_line, 1);
        assert_eq!(state.caret.rendered, before);
        assert_eq!(state.caret.target, CaretTarget { x: 35.0, y: 45.0 });
        assert!(state.caret.advance(now + MOVEMENT + MOVEMENT / 2));
        assert_eq!(state.caret.rendered, CaretTarget { x: 62.5, y: 40.0 });
        state.caret.advance(now + MOVEMENT * 2);
        assert!(session.backspace());
        state.sync_caret(session.position(), 3, 0, false, now + MOVEMENT * 2);
        assert_eq!(state.viewport.top_line, 0);
        assert_eq!(state.caret.target, before);
        assert!(state.caret.advance(now + MOVEMENT * 2 + MOVEMENT / 2));
        assert_eq!(state.caret.rendered, CaretTarget { x: 62.5, y: 40.0 });
        assert!(!state.caret.advance(now + MOVEMENT * 3));
        assert_eq!(state.caret.rendered, before);
    }

    #[test]
    fn identical_text_reset_and_reflow_cancel_in_flight_movement() {
        let now = Instant::now();
        let session = TypingSession::new("one two three");
        let mut state = state(&session, now);
        state.sync_caret(5, 1, 0, false, now);
        state.sync_caret(0, 2, 1, false, now + MOVEMENT / 2);
        assert_eq!(state.target, session.target());
        assert_eq!(state.caret.rendered, state.caret_target(0));
        assert!(!state.caret.advance(now + MOVEMENT));
        for (size, font_size) in [
            (Size::new(142.0, 110.0), 16.0),
            (Size::new(142.0, 150.0), 24.0),
        ] {
            state.sync_caret(6, state.revision + 1, 1, false, now);
            geometry(&mut state, &session, size, font_size);
            state.sync_caret(6, state.revision, 1, true, now + MOVEMENT / 2);
            assert_eq!(state.caret.rendered, state.caret_target(6));
            assert!(!state.caret.advance(now + MOVEMENT));
        }
    }

    #[test]
    fn inspection_and_noops_preserve_viewport_until_an_accepted_edit() {
        let now = Instant::now();
        let session = TypingSession::new("one two three four five six seven eight nine ten");
        let mut state = state(&session, now);
        state.sync_caret(2, 1, 0, false, now);
        state.scroll(2, 2);
        assert_eq!(state.viewport.top_line, 2);
        assert!(state.manually_scrolled);
        assert!(!state.caret.advance(now + MOVEMENT / 2));
        let measured = state.layout.as_ref().unwrap();
        assert!(
            !state
                .viewport
                .visible_lines(measured.lines.len())
                .contains(&measured.active_line(2))
        );
        state.sync_caret(2, 1, 0, false, now + MOVEMENT);
        assert_eq!(state.viewport.top_line, 2);
        state.sync_caret(3, 2, 0, false, now + MOVEMENT);
        assert_eq!(state.viewport.top_line, 0);
        assert_eq!(state.caret.rendered, state.caret_target(3));
        assert!(!state.caret.advance(now + MOVEMENT));
        state.sync_caret(4, 3, 0, false, now + MOVEMENT);
        assert!(state.caret.advance(now + MOVEMENT + MOVEMENT / 2));
    }

    #[test]
    fn animation_redraws_do_not_edit_or_prolong_motion() {
        let now = Instant::now();
        let deadline = now + Duration::from_secs(1);
        let session = TypingSession::new("one two three four five six seven");
        let mut state = state(&session, now);
        state.sync_caret(1, 1, 0, false, now);
        let timed_panel = panel(&session);
        let mut messages: Vec<Event> = Vec::new();
        let mut shell = Shell::new(&mut messages);
        timed_panel.window_event(
            &mut state,
            &WindowEvent::RedrawRequested(now + MOVEMENT / 2),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::NextFrame);
        assert!(shell.is_empty());
        // An unchanged view, including a countdown refresh, must not restart it.
        state.sync_caret(1, 1, 0, false, now + MOVEMENT / 2);
        let mut shell = Shell::new(&mut messages);
        timed_panel.window_event(
            &mut state,
            &WindowEvent::RedrawRequested(now + MOVEMENT),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::Wait);
        assert_eq!(state.caret.rendered, state.caret.target);
        state.scroll(1, 1);
        let mut shell = Shell::new(&mut messages);
        timed_panel.window_event(
            &mut state,
            &WindowEvent::RedrawRequested(deadline),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::Wait);
        assert!(messages.is_empty());
        assert_eq!(state.revision, 1);
        assert_eq!(state.viewport.top_line, 1);
        let mut shell = Shell::new(&mut messages);
        panel(&session).window_event(
            &mut state,
            &WindowEvent::RedrawRequested(deadline + MOVEMENT),
            &mut shell,
        );
        assert_eq!(shell.redraw_request(), RedrawRequest::Wait);
    }

    #[test]
    fn focus_loss_and_window_resize_settle_without_waiting_for_animation() {
        let now = Instant::now();
        let session = TypingSession::new("one two three four five six seven");
        let mut state = state(&session, now);
        let panel = panel(&session);
        let mut messages: Vec<Event> = Vec::new();
        let mut shell = Shell::new(&mut messages);
        state.sync_caret(2, 1, 0, false, now);
        panel.window_event(&mut state, &WindowEvent::Unfocused, &mut shell);
        assert_eq!(state.caret.rendered, state.caret.target);
        assert!(!state.caret.advance(now + MOVEMENT / 2));
        // Pending edits delivered with focus loss must not start new animation.
        state.sync_caret(3, 2, 0, false, now);
        assert!(!state.caret.advance(now + MOVEMENT / 2));
        panel.window_event(&mut state, &WindowEvent::Focused, &mut shell);
        state.sync_caret(4, 3, 0, false, now);
        assert!(state.caret.advance(now + MOVEMENT / 2));
        state.scroll(2, 4);
        panel.window_event(
            &mut state,
            &WindowEvent::Resized(Size::new(1000.0, 600.0)),
            &mut shell,
        );
        assert_eq!(state.viewport.top_line, 0);
        assert!(!state.manually_scrolled);
        assert!(!state.caret.advance(now + MOVEMENT));
    }

    #[test]
    fn terminal_movement_finishes_and_ignored_edits_leave_inspection_intact() {
        let now = Instant::now();
        let mut session = TypingSession::new("one two three");
        let mut state = state(&session, now);
        for c in "one two three".chars() {
            session.input(c);
        }
        state.sync_caret(session.position(), 1, 0, false, now);
        assert!(state.caret.advance(now + MOVEMENT / 2));
        assert!(!session.input('a'));
        assert!(!session.backspace());
        state.sync_caret(session.position(), 1, 0, false, now + MOVEMENT / 2);
        assert!(!state.caret.advance(now + MOVEMENT));
        assert_eq!(state.caret.rendered, CaretTarget { x: 85.0, y: 45.0 });
        state.scroll(-1, session.position());
        assert!(!session.input('a'));
        state.sync_caret(session.position(), 1, 0, false, now + MOVEMENT * 2);
        assert_eq!(state.viewport.top_line, 0);
        assert_eq!(state.caret.rendered, state.caret_target(session.position()));
    }
}
