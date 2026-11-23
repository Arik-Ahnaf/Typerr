//! Figma Dashboard 86:50: fixed header above a 589px scrollable composition.
mod charts;
#[cfg(test)]
mod scroll_tests;
use super::chrome::fitted_size;
use super::{chrome, theme};
use crate::app::Message;
use canvas::{Frame, Path, Stroke, Text};
use iced::advanced::text::{self as native_text, Paragraph};
use iced::widget::{button, canvas, container, responsive, scrollable, stack, text, tooltip};
use iced::{Element, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use typerr::storage::{DashboardData, TestMode};

const NA: &str = "N/A";
const HEADER_HEIGHT: f32 = 75.;
const DESIGN_HEIGHT: f32 = 589.;
const BODY_HEIGHT: f32 = DESIGN_HEIGHT - HEADER_HEIGHT;

/// Scale the fixed preset surface; keep glyphs and artwork undistorted.
#[derive(Clone, Copy)]
struct Scale {
    x: f32,
    y: f32,
}
impl Scale {
    fn new(size: Size) -> Self {
        Self {
            x: (size.width / 700.).max(0.001),
            y: (size.height / BODY_HEIGHT).max(0.001),
        }
    }
    fn content(self) -> f32 {
        self.x.min(self.y)
    }
    fn point(self, point: Point) -> Point {
        Point::new(point.x * self.x, point.y * self.y)
    }
    fn vector(self) -> Vector {
        Vector::new(self.x, self.y)
    }
    fn inverse(self) -> Vector {
        Vector::new(1. / self.x, 1. / self.y)
    }
}

fn scaled_text(frame: &mut Frame, mut text: Text) {
    let scale = Scale::new(frame.size());
    text.position = scale.point(text.position);
    text.size = (text.size.0 * scale.content()).into();
    frame.with_save(|frame| {
        frame.scale_nonuniform(scale.inverse());
        frame.fill_text(text);
    });
}

fn metric(value: Option<f64>, percent: bool) -> String {
    value.map_or_else(
        || NA.into(),
        |v| {
            let number = if v >= 1e6 {
                format!("{v:.1e}")
            } else {
                format!("{v:.0}")
            };
            if percent {
                format!("{number}%")
            } else {
                number
            }
        },
    )
}

fn rank(average_wpm: Option<f64>) -> &'static str {
    let Some(wpm) = average_wpm else {
        return NA;
    };
    match wpm {
        w if w < 40. => "Novice",
        w if w < 60. => "Keybound",
        w if w < 80. => "Clicksmith",
        w if w < 100. => "Swiftstroke",
        w if w < 120. => "Velocity",
        w if w < 140. => "Afterimage",
        w if w < 160. => "Ghostwriter",
        _ => "Machina",
    }
}

fn duration(seconds: f64) -> String {
    if seconds < 60. {
        return format!("{seconds:.0}s");
    }
    let minutes = (seconds / 60.).floor();
    if minutes < 60. {
        format!("{minutes:.0}m")
    } else if minutes < 6e6 {
        format!("{:.0}h {:.0}m", (minutes / 60.).floor(), minutes % 60.)
    } else {
        format!("{:.1e}h", minutes / 60.)
    }
}

fn label(frame: &mut Frame, value: &str, rect: Rectangle, size: f32, font: Font) {
    scaled_text(
        frame,
        Text {
            content: value.into(),
            position: Point::new(rect.center_x(), rect.y),
            size: fitted_size(value, font, size, rect.width).into(),
            font,
            color: theme::palette().correct,
            align_x: iced::alignment::Horizontal::Center.into(),
            line_height: native_text::LineHeight::Relative(1.25),
            ..Text::default()
        },
    );
}

fn left_label(frame: &mut Frame, value: &str, rect: Rectangle, size: f32, font: Font) {
    scaled_text(
        frame,
        Text {
            content: value.into(),
            position: Point::new(rect.x, rect.y),
            size: fitted_size(value, font, size, rect.width).into(),
            font,
            color: theme::palette().correct,
            align_x: iced::alignment::Horizontal::Left.into(),
            line_height: native_text::LineHeight::Relative(1.25),
            ..Text::default()
        },
    );
}

fn centered_label_left(value: &str, rect: Rectangle, size: f32, font: Font) -> f32 {
    type NativeParagraph = <Renderer as native_text::Renderer>::Paragraph;
    let size = fitted_size(value, font, size, rect.width);
    let paragraph = NativeParagraph::with_text(native_text::Text {
        content: value,
        bounds: Size::INFINITE,
        size: size.into(),
        line_height: native_text::LineHeight::Relative(1.25),
        font,
        align_x: native_text::Alignment::Left,
        align_y: iced::alignment::Vertical::Top,
        shaping: native_text::Shaping::Basic,
        wrapping: native_text::Wrapping::None,
    });
    rect.center_x() - paragraph.min_width() / 2.
}

fn rotated(frame: &mut Frame, value: &str, position: Point, size: f32, font: Font) {
    let scale = Scale::new(frame.size());
    let position = scale.point(position);
    frame.with_save(|frame| {
        frame.scale_nonuniform(scale.inverse());
        frame.translate(Vector::new(position.x, position.y));
        frame.rotate(-std::f32::consts::FRAC_PI_2);
        Text {
            content: value.into(),
            size: (size * scale.content()).into(),
            font,
            color: theme::palette().correct,
            ..Text::default()
        }
        .draw_with(|path, color| frame.fill(&path, color));
    });
}

struct DashboardCanvas<'a> {
    name: &'a str,
    data: Option<&'a DashboardData>,
}
impl canvas::Program<Message> for DashboardCanvas<'_> {
    type State = Option<Point>;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if matches!(
            event,
            iced::Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft)
                | iced::Event::Window(iced::window::Event::RedrawRequested(_))
        ) {
            let position = cursor.position_in(bounds);
            if *state != position {
                *state = position;
                return Some(canvas::Action::request_redraw());
            }
        }
        None
    }

    fn draw(
        &self,
        _: &Self::State,
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let scale = Scale::new(bounds.size());
        frame.scale_nonuniform(scale.vector());
        frame.translate(Vector::new(0., -HEADER_HEIGHT));
        for (x, y, h) in [
            (45., 75., 100.),
            (355., 75., 100.),
            (45., 185., 80.),
            (355., 185., 80.),
        ] {
            frame.fill(
                &Path::rounded_rectangle(Point::new(x, y), Size::new(300., h), 4.into()),
                theme::palette().control_background,
            );
        }
        for (value, y, size, font) in [
            (
                self.name,
                97.,
                fitted_size(self.name, theme::SEMIBOLD_FONT, 20., 190.),
                theme::SEMIBOLD_FONT,
            ),
            (
                rank(self.data.and_then(|data| {
                    if data.summary.completed_tests == 0 {
                        Some(0.)
                    } else {
                        data.summary.average_wpm
                    }
                })),
                122.,
                14.,
                theme::ITALIC_FONT,
            ),
            ("Joined 4th January, 2026", 140., 10., theme::FONT),
        ] {
            scaled_text(
                &mut frame,
                Text {
                    content: value.into(),
                    position: Point::new(145., y),
                    size: size.into(),
                    font,
                    color: theme::palette().correct,
                    ..Text::default()
                },
            );
        }
        let summary = self
            .data
            .map(|data| &data.summary)
            .filter(|summary| summary.completed_tests > 0);
        let values = [
            summary.map_or_else(|| NA.into(), |s| s.completed_tests.to_string()),
            metric(summary.and_then(|s| s.average_wpm), false),
            metric(summary.and_then(|s| s.highest_wpm), false),
            summary.map_or_else(|| NA.into(), |s| duration(s.typing_seconds)),
            metric(summary.and_then(|s| s.average_accuracy), true),
            metric(summary.and_then(|s| s.highest_accuracy), true),
        ];
        for (i, title) in [
            "Race Finished",
            "Avg. Wpm",
            "Highest Wpm",
            "Typing Time",
            "Avg. Acc",
            "Highest Acc",
        ]
        .iter()
        .enumerate()
        {
            let column = i % 3;
            let x = [380., 483., 552.][column];
            let width = [100., 67., 86.][column];
            // Center the visible two-row text block in the 75..175 card.
            let y = if i < 3 { 92.5 } else { 132.5 };
            let heading = Rectangle {
                x,
                y,
                width,
                height: 15.,
            };
            let column_heading = ["Race Finished", "Avg. Wpm", "Highest Wpm"][column];
            let content_x = centered_label_left(column_heading, heading, 12., theme::FONT);
            let content_width = x + width - content_x;
            left_label(
                &mut frame,
                title,
                Rectangle {
                    x: content_x,
                    width: content_width,
                    ..heading
                },
                12.,
                theme::FONT,
            );
            left_label(
                &mut frame,
                &values[i],
                Rectangle {
                    x: content_x,
                    y: y + 15.,
                    width: content_width,
                    height: 20.,
                },
                16.,
                theme::MEDIUM_FONT,
            );
        }
        for (mode, title, offset, amounts) in [
            (TestMode::Time, "Time", 0., [15, 30, 60, 120]),
            (TestMode::Words, "Word", 310., [10, 25, 50, 100]),
        ] {
            rotated(
                &mut frame,
                title,
                Point::new(55. + offset, 242.),
                14.,
                theme::ITALIC_FONT,
            );
            for (i, amount) in amounts.into_iter().enumerate() {
                let best = self.data.and_then(|data| {
                    data.personal_bests
                        .iter()
                        .find(|s| s.test.mode == mode && s.test.amount == amount)
                });
                let heading = if mode == TestMode::Time {
                    format!("{amount}s")
                } else {
                    amount.to_string()
                };
                let x = [97., 153., 204., 254.][i] + offset;
                for (value, y, size, font) in [
                    (heading, 197., 14., theme::ITALIC_FONT),
                    (
                        metric(best.map(|b| b.test.wpm), false),
                        215.,
                        20.,
                        theme::MEDIUM_FONT,
                    ),
                    (
                        metric(best.map(|b| b.test.accuracy), true),
                        240.,
                        10.,
                        theme::LIGHT_FONT,
                    ),
                ] {
                    label(
                        &mut frame,
                        &value,
                        Rectangle {
                            x,
                            y,
                            width: 52.,
                            height: 25.,
                        },
                        size,
                        font,
                    );
                }
            }
        }
        frame.stroke(
            &Path::line(Point::new(100., 275.), Point::new(600., 275.)),
            Stroke::default()
                .with_color(theme::palette().correct)
                .with_width(2.),
        );
        let pointer = cursor
            .position_in(bounds)
            .map(|p| Point::new(p.x / scale.x, p.y / scale.y + HEADER_HEIGHT));
        charts::draw(&mut frame, self.data, pointer);
        vec![frame.into_geometry()]
    }
}

pub fn view<'a>(
    data: Option<&'a DashboardData>,
    name: &'a str,
    avatar: &'a iced::widget::image::Handle,
) -> Element<'a, Message> {
    responsive(move |size| {
        let factor = super::sizing::Preset::for_window(size).scale();
        let preset_size = Size::new(700. * factor, BODY_HEIGHT * factor);
        let scale = Scale::new(preset_size);
        let content = scale.content();
        let avatar_center = scale.point(Point::new(95., 125.));
        let avatar = super::picture_prompt::avatar(avatar, 80. * content);
        let greeting_text = format!("Good Afternoon, {name}");
        let greeting_size = fitted_size(
            &greeting_text,
            theme::USER_FONT,
            20.,
            (size.width - 290.).max(1.),
        );
        let greeting = container(
            iced::widget::row![
                chrome::settings_button(true, 25.),
                text(greeting_text)
                    .font(theme::USER_FONT)
                    .size(greeting_size)
                    .line_height(1.25)
                    .color(theme::palette().orange)
            ]
            .spacing(10)
            .align_y(iced::Center),
        )
        .width(iced::Fill)
        .align_x(iced::Right)
        .padding(iced::Padding::ZERO.top(30).right(70));
        let home = tooltip(
            button(iced::widget::Space::new().width(124).height(40))
                .padding(0)
                .on_press(Message::BackToTyping)
                .style(|_, _| button::Style::default()),
            "New test (Tab)",
            tooltip::Position::Bottom,
        );
        let body = stack![
            canvas(DashboardCanvas { data, name })
                .width(preset_size.width)
                .height(preset_size.height),
            container(avatar).padding(
                iced::Padding::ZERO
                    .left(avatar_center.x - 40. * content)
                    .top(avatar_center.y - (40. + HEADER_HEIGHT) * content)
            ),
        ];
        stack![
            container(
                scrollable(container(body).center_x(iced::Fill))
                    .direction(scrollable::Direction::Vertical(
                        scrollable::Scrollbar::new()
                            .width(6)
                            .scroller_width(6)
                            .margin(4)
                    ))
                    .style(|theme, status| {
                        let mut style = scrollable::default(theme, status);
                        style.vertical_rail.background = None;
                        style.vertical_rail.scroller.background =
                            super::theme::palette().pending.into();
                        style
                    })
                    .width(iced::Fill)
                    .height(iced::Fill)
            )
            .padding(iced::Padding::ZERO.top(HEADER_HEIGHT)),
            chrome::brand(),
            greeting,
            container(home).padding(iced::Padding::ZERO.left(50).top(30)),
        ]
        .width(iced::Fill)
        .height(iced::Fill)
        .into()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranks_use_full_precision_average_at_each_boundary() {
        assert_eq!(rank(None), "N/A");
        assert_eq!(rank(Some(0.)), "Novice");
        for (threshold, below, at) in [
            (40., "Novice", "Keybound"),
            (60., "Keybound", "Clicksmith"),
            (80., "Clicksmith", "Swiftstroke"),
            (100., "Swiftstroke", "Velocity"),
            (120., "Velocity", "Afterimage"),
            (140., "Afterimage", "Ghostwriter"),
            (160., "Ghostwriter", "Machina"),
        ] {
            assert_eq!(rank(Some(threshold - 0.01)), below);
            assert_eq!(rank(Some(threshold)), at);
        }
        assert_eq!(rank(Some(500.)), "Machina");
    }

    #[test]
    fn missing_metrics_differ_from_measured_zero() {
        assert_eq!(metric(None, false), "N/A");
        assert_eq!(metric(None, true), "N/A");
        assert_eq!(metric(Some(0.), false), "0");
        assert_eq!(metric(Some(0.), true), "0%");
        assert_eq!(duration(48000.), "13h 20m");
        assert_eq!(duration(15.), "15s");
    }
}
