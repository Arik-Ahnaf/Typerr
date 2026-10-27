//! Native score presentation. Fixtures deliberately do not measure the session.
use super::theme;
use crate::app::Message;
use canvas::{Frame, Path, Stroke, Text};
use iced::widget::{button, canvas, container, row, stack, svg, tooltip};
use iced::{Element, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

pub struct MockScore {
    pub wpm: &'static str,
    pub accuracy: &'static str,
    pub raw: &'static str,
    pub consistency: &'static str,
    pub time: &'static str,
    pub speed: [f32; 15],
    pub raw_speed: [f32; 15],
    pub errors: [(f32, f32); 3],
    pub personal_best: f32,
}

pub const MOCK_SCORE: MockScore = MockScore {
    wpm: "120",
    accuracy: "95%",
    raw: "95%",
    consistency: "95%",
    time: "15s",
    speed: [
        108., 100., 106., 113., 116., 118., 119., 117., 120., 118., 112., 109., 108., 104., 104.,
    ],
    raw_speed: [
        103., 110., 119., 130., 132., 130., 119., 113., 122., 119., 130., 115., 99., 81., 118.,
    ],
    errors: [(11., 2.), (12., 2.), (13., 3.)],
    personal_best: 117.42,
};

// The right axis fits inside the reference's green slot (150..670).
const PLOT: Rectangle = Rectangle {
    x: 150.,
    y: 100.,
    width: 480.,
    height: 150.,
};
fn point(second: f32, value: f32, maximum: f32) -> Point {
    Point::new(
        PLOT.x + (second - 1.) / 14. * PLOT.width,
        PLOT.y + (1. - value / maximum) * PLOT.height,
    )
}

/// Equal-spaced monotone cubic Hermite slopes: zero at extrema, harmonic
/// means elsewhere. Each Bezier segment stays inside its adjacent sample range.
fn slopes(values: &[f32; 15]) -> [f32; 15] {
    let mut slopes = [0.; 15];
    slopes[0] = values[1] - values[0];
    slopes[14] = values[14] - values[13];
    for i in 1..14 {
        let left = values[i] - values[i - 1];
        let right = values[i + 1] - values[i];
        if left * right > 0. {
            slopes[i] = 2. * left * right / (left + right);
        }
    }
    slopes
}
fn curve(values: &[f32; 15]) -> Path {
    let slopes = slopes(values);
    Path::new(|path| {
        path.move_to(point(1., values[0], 140.));
        for i in 0..14 {
            let second = i as f32 + 1.;
            path.bezier_curve_to(
                point(second + 1. / 3., values[i] + slopes[i] / 3., 140.),
                point(second + 2. / 3., values[i + 1] - slopes[i + 1] / 3., 140.),
                point(second + 1., values[i + 1], 140.),
            );
        }
    })
}

struct ScoreCanvas {
    opacity: f32,
}
impl ScoreCanvas {
    fn text(&self, frame: &mut Frame, value: &str, x: f32, y: f32, size: f32, color: iced::Color) {
        frame.fill_text(Text {
            content: value.into(),
            position: Point::new(x, y),
            size: size.into(),
            font: theme::FONT,
            color: theme::alpha(color, self.opacity),
            line_height: iced::widget::text::LineHeight::Absolute((size * 1.25).into()),
            ..Text::default()
        });
    }
    fn rotated(&self, frame: &mut Frame, value: &str, position: Point, angle: f32, size: f32) {
        // Outline text so rotation is honored by both native renderer backends.
        frame.with_save(|frame| {
            frame.translate(Vector::new(position.x, position.y));
            frame.rotate(angle);
            Text {
                content: value.into(),
                size: size.into(),
                font: theme::FONT,
                color: theme::alpha(theme::PENDING, self.opacity),
                ..Text::default()
            }
            .draw_with(|path, color| frame.fill(&path, color));
        });
    }
}
impl canvas::Program<Message> for ScoreCanvas {
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
        let score = &MOCK_SCORE;
        let stroke = |color, width| {
            Stroke::default()
                .with_color(theme::alpha(color, self.opacity))
                .with_width(width)
        };
        for second in 1..=15 {
            let top = point(second as f32, 140., 140.);
            let bottom = point(second as f32, 0., 140.);
            frame.stroke(
                &Path::line(top, bottom),
                stroke(theme::alpha(theme::GRAPH_GRID, 0.6), 1.),
            );
            frame.fill_text(Text {
                content: second.to_string(),
                position: Point::new(bottom.x, 252.),
                size: 12.into(),
                font: theme::FONT,
                align_x: iced::alignment::Horizontal::Center.into(),
                color: theme::alpha(theme::PENDING, self.opacity),
                ..Text::default()
            });
        }
        for value in [0., 40., 80., 120.] {
            let left = point(1., value, 140.);
            frame.stroke(
                &Path::line(left, point(15., value, 140.)),
                stroke(theme::alpha(theme::GRAPH_GRID, 0.6), 1.),
            );
            self.rotated(
                &mut frame,
                &format!("{value:.0}"),
                Point::new(130., left.y + 7.),
                -std::f32::consts::FRAC_PI_2,
                14.,
            );
        }
        self.rotated(
            &mut frame,
            "Words per minute",
            Point::new(105., 233.),
            -std::f32::consts::FRAC_PI_2,
            12.,
        );
        for errors in 0..=3 {
            self.text(
                &mut frame,
                &errors.to_string(),
                638.,
                point(15., errors as f32, 3.).y - 7.,
                11.,
                theme::PENDING,
            );
        }
        self.rotated(
            &mut frame,
            "Errors",
            Point::new(670., 151.),
            std::f32::consts::FRAC_PI_2,
            11.,
        );
        // The curves and markers use their own clipped frame; axis labels do not.
        frame.with_clip(PLOT, |frame| {
            frame.stroke(&curve(&score.raw_speed), stroke(theme::GRAPH_RAW, 2.));
            frame.stroke(&curve(&score.speed), stroke(theme::GRAPH_WPM, 2.));
            let mut guide = stroke(theme::alpha(theme::GRAPH_WPM, 0.65), 1.);
            guide.line_dash = canvas::LineDash {
                segments: &[7., 5.],
                offset: 0,
            };
            frame.stroke(
                &Path::line(
                    point(1., score.personal_best, 140.),
                    point(15., score.personal_best, 140.),
                ),
                guide,
            );
            for &(second, errors) in &score.errors {
                let p = point(second, errors, 3.);
                // Inset a maximum-valued marker so its complete cross stays visible.
                let y = p.y.max(PLOT.y + 4.);
                for direction in [-1., 1.] {
                    frame.stroke(
                        &Path::line(
                            Point::new(p.x - 2.5, y - 2.5 * direction),
                            Point::new(p.x + 2.5, y + 2.5 * direction),
                        ),
                        stroke(theme::GRAPH_ERROR, 1.5),
                    );
                }
            }
        });
        let pb_y = point(8., score.personal_best, 140.).y;
        frame.fill_rectangle(
            Point::new(342., pb_y - 10.),
            Size::new(91., 20.),
            theme::alpha(theme::CONTROL_BACKGROUND, self.opacity),
        );
        self.text(
            &mut frame,
            "PB: 117.42",
            348.,
            pb_y - 7.,
            11.,
            theme::PENDING,
        );
        self.text(&mut frame, "WPM", 30., 100., 24., theme::CORRECT);
        self.text(&mut frame, score.wpm, 30., 130., 32., theme::BLUE);
        self.text(&mut frame, "ACC", 30., 175., 24., theme::CORRECT);
        self.text(&mut frame, score.accuracy, 31., 205., 32., theme::BLUE);
        for (label, value, x) in [
            ("Raw", score.raw, 160.),
            ("Consistency", score.consistency, 244.),
            ("Time", score.time, 390.),
        ] {
            self.text(&mut frame, label, x, 270., 16., theme::CORRECT);
            self.text(&mut frame, value, x, 290., 24., theme::BLUE);
        }
        vec![frame.into_geometry()]
    }
}

pub fn view(opacity: f32, enabled: bool) -> Element<'static, Message> {
    let control = |label, message, bytes: &'static [u8], width, height| {
        let artwork = svg(svg::Handle::from_memory(bytes))
            .width(width)
            .height(height)
            .opacity(opacity)
            .style(|_, _| svg::Style {
                color: Some(theme::PENDING),
            });
        tooltip(
            button(container(artwork).center_x(36).center_y(40))
                .padding(0)
                .on_press_maybe(enabled.then_some(message))
                .style(move |_, status| iced::widget::button::Style {
                    background: (enabled
                        && matches!(status, iced::widget::button::Status::Hovered))
                    .then_some(theme::alpha(theme::CONTROL_BACKGROUND, 0.35).into()),
                    border: iced::Border::default().rounded(4),
                    ..Default::default()
                }),
            label,
            tooltip::Position::Bottom,
        )
    };
    let controls = row![
        control(
            "Next test (Tab)",
            Message::Next,
            include_bytes!("../../assets/icons/next.svg"),
            12,
            20
        ),
        control(
            "Retry same text",
            Message::Retry,
            include_bytes!("../../assets/icons/restart.svg"),
            25,
            25
        ),
    ];
    stack![
        canvas(ScoreCanvas { opacity }).width(700).height(400),
        container(controls).padding(iced::Padding::ZERO.left(309.5).top(330))
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_coordinates_and_gutters_fit_reference() {
        assert_eq!(point(1., 140., 140.), Point::new(150., 100.));
        assert_eq!(point(15., 0., 140.), Point::new(630., 250.));
        assert_eq!(point(15., 0., 140.).x + 40., 670.);
        for (second, errors) in MOCK_SCORE.errors {
            assert!(PLOT.contains(point(second, errors, 3.)));
        }
    }
    #[test]
    fn interpolated_samples_do_not_overshoot() {
        for values in [&MOCK_SCORE.speed, &MOCK_SCORE.raw_speed, &[42.; 15]] {
            let m = slopes(values);
            for i in 0..14 {
                for step in 0..=100 {
                    let t = step as f32 / 100.;
                    let y = (2. * t * t * t - 3. * t * t + 1.) * values[i]
                        + (t * t * t - 2. * t * t + t) * m[i]
                        + (-2. * t * t * t + 3. * t * t) * values[i + 1]
                        + (t * t * t - t * t) * m[i + 1];
                    assert!(
                        y >= values[i].min(values[i + 1]) - 0.001
                            && y <= values[i].max(values[i + 1]) + 0.001
                    );
                }
            }
        }
    }
}
