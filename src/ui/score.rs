//! Native presentation of the frozen completed-session measurements.
use super::theme;
use crate::app::Message;
use canvas::{Frame, Path, Stroke, Text};
use iced::widget::{button, canvas, container, row, stack, svg, tooltip};
use iced::{Element, Point, Rectangle, Renderer, Theme, Vector, mouse};
use typerr::statistics::Score;

// The right axis fits inside the reference's green slot (150..670).
const PLOT: Rectangle = Rectangle {
    x: 150.,
    y: 100.,
    width: 480.,
    height: 150.,
};
fn point(second: f32, value: f32, maximum: f32, duration: f32) -> Point {
    Point::new(
        PLOT.x + second / duration * PLOT.width,
        PLOT.y + (1. - value / maximum) * PLOT.height,
    )
}

/// Shape-preserving slopes for samples at arbitrary elapsed timestamps.
fn slopes(values: &[Point]) -> Vec<f32> {
    let mut slopes = vec![0.; values.len()];
    if values.len() < 2 {
        return slopes;
    }
    let secant = |i: usize| {
        let dx = values[i + 1].x - values[i].x;
        if dx > 0. {
            (values[i + 1].y - values[i].y) / dx
        } else {
            0.
        }
    };
    slopes[0] = secant(0);
    let last = values.len() - 1;
    slopes[last] = secant(last - 1);
    for (i, slope) in slopes.iter_mut().enumerate().take(last).skip(1) {
        let left = secant(i - 1);
        let right = secant(i);
        if left * right > 0. {
            *slope = 2. * left * right / (left + right);
        }
    }
    slopes
}
fn curve(values: &[Point]) -> Path {
    let slopes = slopes(values);
    Path::new(|path| {
        if let Some(first) = values.first() {
            path.move_to(*first);
        }
        for i in 0..values.len().saturating_sub(1) {
            let dx = (values[i + 1].x - values[i].x) / 3.;
            path.bezier_curve_to(
                Point::new(values[i].x + dx, values[i].y + slopes[i] * dx),
                Point::new(values[i + 1].x - dx, values[i + 1].y - slopes[i + 1] * dx),
                values[i + 1],
            );
        }
    })
}

fn tick_step(maximum: f32, divisions: f32) -> f32 {
    let rough = maximum.max(1.) / divisions;
    let power = 10_f32.powf(rough.log10().floor());
    [1., 2., 5., 10.]
        .into_iter()
        .find(|v| v * power >= rough)
        .unwrap()
        * power
}
fn number(value: f32) -> String {
    if value >= 1_000_000. {
        format!("{:.0}M", value / 1_000_000.)
    } else if value >= 10_000. {
        format!("{:.0}k", value / 1000.)
    } else if value.fract().abs() < 0.001 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

struct ScoreCanvas<'a> {
    score: &'a Score,
    opacity: f32,
}
impl ScoreCanvas<'_> {
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
impl canvas::Program<Message> for ScoreCanvas<'_> {
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
        let score = self.score;
        let duration = score.elapsed.as_secs_f32().max(1.);
        let peak = score
            .samples
            .iter()
            .map(|s| s.raw.max(s.wpm) as f32)
            .fold(0., f32::max);
        let speed_step = tick_step(peak, 4.).max(1.);
        let maximum = (peak / speed_step).ceil().max(1.) * speed_step;
        let error_peak = score.samples.iter().map(|s| s.errors).max().unwrap_or(0) as f32;
        let error_step = tick_step(error_peak, 3.).ceil().max(1.);
        let error_maximum = (error_peak / error_step).ceil().max(1.) * error_step;
        let point = |second, value, max| point(second, value, max, duration);
        let stroke = |color, width| {
            Stroke::default()
                .with_color(theme::alpha(color, self.opacity))
                .with_width(width)
        };
        let time_step = tick_step(duration, 8.);
        for tick in 0..=(duration / time_step).floor() as usize {
            let second = tick as f32 * time_step;
            let top = point(second, maximum, maximum);
            let bottom = point(second, 0., maximum);
            frame.stroke(
                &Path::line(top, bottom),
                stroke(theme::alpha(theme::GRAPH_GRID, 0.6), 1.),
            );
            frame.fill_text(Text {
                content: number(second),
                position: Point::new(bottom.x, 252.),
                size: 12.into(),
                font: theme::FONT,
                align_x: iced::alignment::Horizontal::Center.into(),
                color: theme::alpha(theme::PENDING, self.opacity),
                ..Text::default()
            });
        }
        for tick in 0..=(maximum / speed_step).round() as usize {
            let value = tick as f32 * speed_step;
            let left = point(0., value, maximum);
            frame.stroke(
                &Path::line(left, point(duration, value, maximum)),
                stroke(theme::alpha(theme::GRAPH_GRID, 0.6), 1.),
            );
            self.rotated(
                &mut frame,
                &number(value),
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
        for tick in 0..=(error_maximum / error_step).round() as usize {
            let errors = tick as f32 * error_step;
            self.text(
                &mut frame,
                &number(errors),
                638.,
                point(duration, errors, error_maximum).y - 7.,
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
            for (raw, color) in [(true, theme::GRAPH_RAW), (false, theme::GRAPH_WPM)] {
                let points: Vec<_> = score
                    .samples
                    .iter()
                    .map(|s| {
                        point(
                            s.seconds as f32,
                            if raw { s.raw } else { s.wpm } as f32,
                            maximum,
                        )
                    })
                    .collect();
                frame.stroke(&curve(&points), stroke(color, 2.));
                if points.len() == 1 {
                    frame.fill(
                        &Path::circle(points[0], 2.5),
                        theme::alpha(color, self.opacity),
                    );
                }
            }
            for sample in score.samples.iter().filter(|sample| sample.errors > 0) {
                let p = point(sample.seconds as f32, sample.errors as f32, error_maximum);
                // Inset a maximum-valued marker so its complete cross stays visible.
                let y = p.y.clamp(PLOT.y + 4., PLOT.y + PLOT.height - 4.);
                let x = p.x.clamp(PLOT.x + 4., PLOT.x + PLOT.width - 4.);
                for direction in [-1., 1.] {
                    frame.stroke(
                        &Path::line(
                            Point::new(x - 2.5, y - 2.5 * direction),
                            Point::new(x + 2.5, y + 2.5 * direction),
                        ),
                        stroke(theme::GRAPH_ERROR, 1.5),
                    );
                }
            }
        });
        self.text(&mut frame, "WPM", 30., 100., 24., theme::CORRECT);
        self.text(
            &mut frame,
            &format!("{:.0}", score.wpm),
            30.,
            130.,
            metric_size(score.wpm, 32.),
            theme::BLUE,
        );
        self.text(&mut frame, "ACC", 30., 175., 24., theme::CORRECT);
        self.text(
            &mut frame,
            &format!("{:.0}%", score.accuracy),
            31.,
            205.,
            if score.accuracy.round() >= 100. {
                28.
            } else {
                32.
            },
            theme::BLUE,
        );
        for (label, value, x) in [
            ("Raw", format!("{:.0}", score.raw), 160.),
            ("Consistency", format!("{:.0}%", score.consistency), 244.),
            (
                "Time",
                format!("{}s", number(score.elapsed.as_secs_f32())),
                390.,
            ),
        ] {
            self.text(&mut frame, label, x, 270., 16., theme::CORRECT);
            self.text(
                &mut frame,
                &value,
                x,
                290.,
                (120. / value.len().max(5) as f32).min(24.),
                theme::BLUE,
            );
        }
        vec![frame.into_geometry()]
    }
}

pub fn view(score: &Score, opacity: f32, enabled: bool) -> Element<'_, Message> {
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
        canvas(ScoreCanvas { score, opacity })
            .width(700)
            .height(400),
        container(controls).padding(iced::Padding::ZERO.left(309.5).top(330))
    ]
    .into()
}

fn metric_size(value: f64, normal: f32) -> f32 {
    (110. / format!("{value:.0}").len().max(1) as f32).min(normal)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_coordinates_scale_to_duration_and_speed() {
        for duration in [1., 15., 30., 60., 120., 1500.] {
            assert_eq!(point(0., 240., 240., duration), Point::new(150., 100.));
            assert_eq!(point(duration, 0., 240., duration), Point::new(630., 250.));
            assert!(duration / tick_step(duration, 8.) <= 8.);
        }
    }
    #[test]
    fn interpolated_samples_do_not_overshoot_including_partial_last_interval() {
        for heights in [vec![100., 40., 60., 10.], vec![42.; 4]] {
            let values: Vec<_> = [1., 2., 3., 3.1]
                .into_iter()
                .zip(heights)
                .map(|(x, y)| Point::new(x, y))
                .collect();
            let m = slopes(&values);
            for i in 0..values.len() - 1 {
                let dx = values[i + 1].x - values[i].x;
                for step in 0..=100 {
                    let t = step as f32 / 100.;
                    let y = (2. * t * t * t - 3. * t * t + 1.) * values[i].y
                        + (t * t * t - 2. * t * t + t) * m[i] * dx
                        + (-2. * t * t * t + 3. * t * t) * values[i + 1].y
                        + (t * t * t - t * t) * m[i + 1] * dx;
                    assert!(
                        y >= values[i].y.min(values[i + 1].y) - 0.001
                            && y <= values[i].y.max(values[i + 1].y) + 0.001
                    );
                }
            }
        }
        assert!(slopes(&[]).is_empty());
        assert_eq!(slopes(&[Point::ORIGIN]), vec![0.]);
    }
}
