//! Figma Dashboard 86:50, using completed-test history and a native histogram.
use super::chrome::fitted_size;
use super::{chrome, theme};
use crate::app::Message;
use canvas::{Frame, Path, Stroke, Text};
use iced::advanced::text::{self as native_text, Paragraph};
use iced::widget::{button, canvas, container, responsive, stack, text, tooltip};
use iced::{Element, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use typerr::storage::{DashboardData, TestMode};

const NA: &str = "N/A";

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
            y: (size.height / 400.).max(0.001),
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
impl DashboardCanvas<'_> {
    fn chart(&self, frame: &mut Frame) {
        // The pink rectangle in the design is placement guidance only.
        frame.stroke(
            &Path::line(Point::new(100., 275.), Point::new(600., 275.)),
            Stroke::default()
                .with_color(theme::palette().correct)
                .with_width(2.),
        );
        let plot = Rectangle {
            x: 79.,
            y: 280.,
            width: 574.,
            height: 72.,
        };
        rotated(frame, "Tests", Point::new(42., 333.), 10., theme::FONT);
        let data = self.data.filter(|data| data.summary.completed_tests > 0);
        let peak = data.map_or(0, |data| *data.bins.iter().max().unwrap());
        let step = ((peak as f64 / 4.).ceil() as u64).max(1);
        let maximum = step as f64 * 4.;
        for tick in 0..=4 {
            let y = plot.y + plot.height * (1. - tick as f32 / 4.);
            frame.stroke(
                &Path::line(Point::new(plot.x, y), Point::new(plot.x + plot.width, y)),
                Stroke::default()
                    .with_color(theme::palette().control_background)
                    .with_width(0.5),
            );
            if data.is_some() {
                label(
                    frame,
                    &metric(Some(tick as f64 * step as f64), false),
                    Rectangle {
                        x: 49.,
                        y: y - 6.,
                        width: 26.,
                        height: 12.,
                    },
                    9.,
                    theme::FONT,
                );
            }
        }
        if let Some(data) = data {
            let cell = plot.width / 12.;
            for (i, count) in data.bins.iter().enumerate() {
                let x = plot.x + cell * i as f32;
                let h = (*count as f64 / maximum) as f32 * plot.height;
                if *count > 0 {
                    frame.fill_rectangle(
                        Point::new(x + cell * 0.14, plot.y + plot.height - h),
                        Size::new(cell * 0.72, h),
                        theme::palette().blue,
                    );
                }
                let low = i as f64 * data.bin_width;
                let high = (i + 1) as f64 * data.bin_width - 1.;
                let range = format!("{}–{}", metric(Some(low), false), metric(Some(high), false));
                label(
                    frame,
                    &range,
                    Rectangle {
                        x,
                        y: 356.,
                        width: cell - 3.,
                        height: 14.,
                    },
                    9.,
                    theme::FONT,
                );
            }
        } else {
            label(
                frame,
                NA,
                Rectangle {
                    x: plot.x,
                    y: 305.,
                    width: plot.width,
                    height: 25.,
                },
                16.,
                theme::FONT,
            );
        }
    }
}

impl canvas::Program<Message> for DashboardCanvas<'_> {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.scale_nonuniform(Scale::new(bounds.size()).vector());
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
            let y = if i < 3 { 85. } else { 125. };
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
        self.chart(&mut frame);
        vec![frame.into_geometry()]
    }
}

pub fn view<'a>(
    data: Option<&'a DashboardData>,
    name: &'a str,
    avatar: &'a iced::widget::image::Handle,
) -> Element<'a, Message> {
    responsive(move |size| {
        let preset_size = super::sizing::Preset::for_window(size).size();
        let scale = Scale::new(preset_size);
        let content = scale.content();
        let avatar_center = scale.point(Point::new(95., 125.));
        let avatar = super::picture_prompt::avatar(avatar, 80. * content);
        let greeting_text = format!("Good Afternoon, {name}");
        let greeting_size = fitted_size(
            &greeting_text,
            theme::USER_FONT,
            20.,
            (size.width - 260.).max(1.),
        );
        let greeting = container(
            text(greeting_text)
                .font(theme::USER_FONT)
                .size(greeting_size)
                .line_height(1.25)
                .color(theme::palette().orange),
        )
        .width(iced::Fill)
        .align_x(iced::Right)
        .padding(iced::Padding::ZERO.top(37).right(70));
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
                    .top(avatar_center.y - 40. * content)
            ),
        ];
        stack![
            container(body).center_x(iced::Fill).center_y(iced::Fill),
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
