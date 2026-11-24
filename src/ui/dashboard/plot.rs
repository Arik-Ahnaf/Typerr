//! Completed-test history plot; filter and preset state remains transient.
use super::{label, left_label, theme};
use crate::settings::{Course, Mode};
use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Point, Rectangle, Size};

const TOP: f32 = 585.;
const CHART: Rectangle = Rectangle {
    x: 89.,
    y: 733.,
    width: 510.,
    height: 154.,
};
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Filters {
    range: usize,
    mode: usize,
    amount: usize,
    punctuation: usize,
    numbers: usize,
}
#[derive(Clone, Debug)]
pub(super) struct State {
    filters: Filters,
    series: [bool; 4],
    advanced: bool,
    preset: Option<Filters>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            filters: Filters {
                range: 4,
                ..Filters::default()
            },
            series: [true; 4],
            advanced: false,
            preset: None,
        }
    }
}
#[derive(Clone, Copy)]
enum Action {
    All,
    Current,
    Advanced,
    Preset,
    Range(usize),
    Mode,
    Amount,
    Punctuation,
    Numbers,
    Series(usize),
}
struct Control {
    bounds: Rectangle,
    title: String,
    active: bool,
    action: Action,
}
fn control(
    x: f32,
    y: f32,
    width: f32,
    title: impl Into<String>,
    active: bool,
    action: Action,
) -> Control {
    Control {
        bounds: Rectangle {
            x,
            y,
            width,
            height: 24.,
        },
        title: title.into(),
        active,
        action,
    }
}
impl State {
    fn controls(&self) -> Vec<Control> {
        let mut controls = vec![
            control(
                57.,
                614.,
                140.,
                "All",
                self.filters.mode == 0
                    && self.filters.amount == 0
                    && self.filters.punctuation == 0
                    && self.filters.numbers == 0,
                Action::All,
            ),
            control(205., 614., 140., "Current settings", false, Action::Current),
            control(
                353.,
                614.,
                140.,
                "Advanced",
                self.advanced,
                Action::Advanced,
            ),
            control(
                501.,
                614.,
                142.,
                if self.preset.is_some() {
                    "Apply preset"
                } else {
                    "Save as preset"
                },
                false,
                Action::Preset,
            ),
        ];
        for (i, title) in [
            "Last day",
            "Last week",
            "Last month",
            "Last 3 months",
            "All time",
        ]
        .into_iter()
        .enumerate()
        {
            controls.push(control(
                57. + i as f32 * 119.,
                646.,
                110.,
                title,
                self.filters.range == i,
                Action::Range(i),
            ));
        }
        if self.advanced {
            let mode = ["All modes", "Words", "Time"][self.filters.mode];
            let amount = self
                .amount()
                .map_or("All amounts".into(), |v| format!("Amount: {v}"));
            for (i, (title, action)) in [
                (mode.to_owned(), Action::Mode),
                (amount, Action::Amount),
                (
                    format!(
                        "Punctuation: {}",
                        ["all", "off", "on"][self.filters.punctuation]
                    ),
                    Action::Punctuation,
                ),
                (
                    format!("Numbers: {}", ["all", "off", "on"][self.filters.numbers]),
                    Action::Numbers,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                controls.push(control(
                    57. + i as f32 * 148.,
                    681.,
                    140.,
                    title,
                    false,
                    action,
                ));
            }
        }
        for (i, title) in ["Speed", "Accuracy", "Avg of 10", "Avg of 100"]
            .into_iter()
            .enumerate()
        {
            controls.push(control(
                269. + i as f32 * 94.,
                922.,
                86.,
                title,
                self.series[i],
                Action::Series(i),
            ));
        }
        controls
    }
    fn amount(&self) -> Option<u16> {
        (self.filters.amount > 0).then(|| {
            if self.filters.mode == 2 {
                Mode::Time
            } else {
                Mode::Words
            }
            .amounts()[self.filters.amount - 1]
        })
    }
    pub(super) fn hit(&self, pointer: Point) -> bool {
        self.controls().iter().any(|c| c.bounds.contains(pointer))
    }
    pub(super) fn click(&mut self, pointer: Point, course: Course) -> bool {
        let Some(action) = self
            .controls()
            .into_iter()
            .find(|c| c.bounds.contains(pointer))
            .map(|c| c.action)
        else {
            return false;
        };
        match action {
            Action::All => {
                self.filters = Filters {
                    range: self.filters.range,
                    ..Filters::default()
                }
            }
            Action::Current => {
                let amount = if course.mode == Mode::Words {
                    course.word_target
                } else {
                    course.time_limit_seconds
                };
                self.filters.mode = if course.mode == Mode::Words { 1 } else { 2 };
                self.filters.amount = course
                    .mode
                    .amounts()
                    .iter()
                    .position(|v| *v == amount)
                    .unwrap_or(0)
                    + 1;
                self.filters.punctuation = usize::from(course.options.punctuation) + 1;
                self.filters.numbers = usize::from(course.options.numbers) + 1;
            }
            Action::Advanced => self.advanced = !self.advanced,
            Action::Preset => {
                if let Some(preset) = self.preset {
                    self.filters = preset;
                } else {
                    self.preset = Some(self.filters);
                }
            }
            Action::Range(range) => self.filters.range = range,
            Action::Mode => {
                self.filters.mode = (self.filters.mode + 1) % 3;
                self.filters.amount = 0;
            }
            Action::Amount => self.filters.amount = (self.filters.amount + 1) % 5,
            Action::Punctuation => self.filters.punctuation = (self.filters.punctuation + 1) % 3,
            Action::Numbers => self.filters.numbers = (self.filters.numbers + 1) % 3,
            Action::Series(i) => self.series[i] = !self.series[i],
        }
        true
    }
    fn samples(&self, data: &typerr::storage::DashboardData) -> Vec<(f32, f32)> {
        let days = [1i64, 7, 30, 90, 0][self.filters.range];
        let start = if days == 0 {
            i64::MIN
        } else {
            data.as_of_ms.saturating_sub(days * 86_400_000)
        };
        data.history
            .iter()
            .map(|row| &row.test)
            .filter(|test| {
                test.completed_at_ms >= start
                    && test.completed_at_ms <= data.as_of_ms
                    && (self.filters.mode == 0
                        || self.filters.mode
                            == if test.mode == typerr::storage::TestMode::Words {
                                1
                            } else {
                                2
                            })
                    && self.amount().is_none_or(|amount| test.amount == amount)
                    && (self.filters.punctuation == 0
                        || test.punctuation == (self.filters.punctuation == 2))
                    && (self.filters.numbers == 0 || test.numbers == (self.filters.numbers == 2))
            })
            .map(|test| (test.wpm as f32, test.accuracy as f32))
            .collect()
    }
}
fn caption(frame: &mut Frame, text: &str, x: f32, y: f32, width: f32, size: f32) {
    left_label(
        frame,
        text,
        Rectangle {
            x,
            y,
            width,
            height: 20.,
        },
        size,
        theme::FONT,
    );
}
fn point(index: usize, count: usize, value: f32, accuracy: bool) -> Point {
    let fraction = if accuracy { value / 100. } else { value / 140. };
    Point::new(
        CHART.x + index as f32 / count.saturating_sub(1).max(1) as f32 * CHART.width,
        CHART.y + (1. - fraction) * CHART.height,
    )
}
fn line(frame: &mut Frame, samples: &[(f32, f32)], window: usize, accuracy: bool, color: Color) {
    if window == 1 {
        for (i, sample) in samples.iter().enumerate() {
            frame.fill(
                &Path::circle(
                    point(
                        i,
                        samples.len(),
                        if accuracy { sample.1 } else { sample.0 },
                        accuracy,
                    ),
                    0.8,
                ),
                color,
            );
        }
        return;
    }
    if samples.len() == 1 {
        frame.fill(&Path::circle(point(0, 1, samples[0].0, false), 2.), color);
        return;
    }
    let path = Path::new(|builder| {
        for i in 0..samples.len() {
            let start = (i + 1).saturating_sub(window);
            let value = samples[start..=i]
                .iter()
                .map(|v| if accuracy { v.1 } else { v.0 })
                .sum::<f32>()
                / (i - start + 1) as f32;
            let p = point(i, samples.len(), value, accuracy);
            if i == 0 {
                builder.move_to(p);
            } else {
                builder.line_to(p);
            }
        }
    });
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(color)
            .with_width(if window == 1 { 0.7 } else { 1.5 }),
    );
}
pub(super) fn draw(
    frame: &mut Frame,
    state: &State,
    pointer: Option<Point>,
    data: Option<&typerr::storage::DashboardData>,
) {
    let samples = data.map(|data| state.samples(data)).unwrap_or_default();
    let maximum = (samples.iter().map(|s| s.0).fold(140f32, f32::max) / 20.).ceil() * 20.;
    let palette = theme::palette();
    frame.fill(
        &Path::rounded_rectangle(Point::new(45., TOP), Size::new(610., 375.), 4.into()),
        palette.control_background,
    );
    caption(frame, "Filters", 57., 595., 100., 12.);
    caption(frame, "Test history", 530., 595., 112., 10.);
    for control in state.controls() {
        let hovered = pointer.is_some_and(|p| control.bounds.contains(p));
        frame.fill(
            &Path::rounded_rectangle(control.bounds.position(), control.bounds.size(), 4.into()),
            if control.active {
                palette.blue
            } else if hovered {
                palette.pending
            } else {
                palette.background
            },
        );
        label(
            frame,
            &control.title,
            Rectangle {
                y: control.bounds.y + 5.,
                ..control.bounds
            },
            10.,
            theme::FONT,
        );
    }
    if !state.advanced {
        caption(
            frame,
            &format!(
                "{} · {} · punctuation {} · numbers {}",
                ["All modes", "Words", "Time"][state.filters.mode],
                state
                    .amount()
                    .map_or("all amounts".into(), |v| v.to_string()),
                ["all", "off", "on"][state.filters.punctuation],
                ["all", "off", "on"][state.filters.numbers]
            ),
            75.,
            687.,
            550.,
            10.,
        );
    }
    caption(frame, "WPM", 57., 714., 40., 9.);
    caption(frame, "Acc %", 603., 714., 42., 9.);
    for tick in 0..=4 {
        let y = CHART.y + CHART.height * tick as f32 / 4.;
        frame.stroke(
            &Path::line(Point::new(CHART.x, y), Point::new(CHART.x + CHART.width, y)),
            Stroke::default()
                .with_color(theme::alpha(palette.pending, 0.4))
                .with_width(0.5),
        );
        caption(
            frame,
            &format!("{:.0}", maximum * (1. - tick as f32 / 4.)),
            57.,
            y - 5.,
            29.,
            9.,
        );
        caption(frame, &(100 - tick * 25).to_string(), 607., y - 5., 35., 9.);
    }
    let plotted: Vec<_> = samples
        .iter()
        .map(|s| (s.0 * 140. / maximum, s.1))
        .collect();
    for (i, (window, accuracy, color)) in [
        (1, false, theme::alpha(palette.blue, 0.45)),
        (1, true, theme::alpha(palette.cyan, 0.3)),
        (10, false, palette.orange),
        (100, false, palette.correct),
    ]
    .into_iter()
    .enumerate()
    {
        if state.series[i] {
            line(frame, &plotted, window, accuracy, color);
        }
    }
    caption(frame, "Older", CHART.x, 894., 60., 9.);
    caption(frame, "Latest", 558., 894., 45., 9.);
    let detail = pointer
        .filter(|p| !samples.is_empty() && CHART.contains(*p))
        .map(|p| {
            let index = (((p.x - CHART.x) / CHART.width * (samples.len() - 1) as f32).round()
                as usize)
                .min(samples.len() - 1);
            let value = samples[index];
            format!("Test {} · {:.0} WPM · {:.1}%", index + 1, value.0, value.1)
        })
        .unwrap_or_else(|| {
            if data.is_none() {
                "History unavailable".into()
            } else if samples.is_empty() {
                "No matching completed tests".into()
            } else {
                format!("{} tests · oldest → newest", samples.len())
            }
        });
    caption(frame, &detail, 160., 894., 390., 9.);
    caption(frame, "Completed tests", 57., 929., 200., 10.);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_history_filters_intersect_and_empty_history_stays_empty() {
        use typerr::storage::{CompletedTest, ScoreStore, TestMode};
        let store = ScoreStore::in_memory().unwrap();
        let now = 1_800_000_000_000;
        for (age, mode, punctuation) in [
            (0, TestMode::Words, true),
            (2, TestMode::Words, false),
            (0, TestMode::Time, false),
        ] {
            store
                .insert(&CompletedTest {
                    completed_at_ms: now - age * 86_400_000,
                    mode,
                    amount: 50,
                    punctuation,
                    numbers: false,
                    wpm: 175.,
                    raw_wpm: 180.,
                    accuracy: 72.,
                    consistency: 80.,
                    elapsed_seconds: 30.,
                })
                .unwrap();
        }
        let data = store.dashboard_at(now).unwrap();
        let mut state = State::default();
        assert_eq!(state.samples(&data).len(), 3);
        state.filters.range = 0;
        assert_eq!(state.samples(&data).len(), 2);
        state.filters.mode = 1;
        assert_eq!(state.samples(&data), vec![(175., 72.)]);
        state.filters.punctuation = 1;
        assert!(state.samples(&data).is_empty());
        assert!(state.samples(&Default::default()).is_empty());
    }
}
