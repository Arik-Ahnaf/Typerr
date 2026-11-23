//! Dashboard charts use the same transformed canvas, fonts, and palette as cards.
use super::{NA, label, left_label, metric, rotated, theme};
use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Point, Rectangle, Size};
use typerr::storage::{ActivityDay, DashboardData};

const GRID_X: f32 = 102.;
const GRID_Y: f32 = 329.;
const CELL: f32 = 10.;
const PLOT: Rectangle = Rectangle {
    x: 89.,
    y: 478.,
    width: 550.,
    height: 57.,
};

fn caption(frame: &mut Frame, value: &str, x: f32, y: f32, width: f32, size: f32) {
    left_label(
        frame,
        value,
        Rectangle {
            x,
            y,
            width,
            height: size * 1.25,
        },
        size,
        theme::FONT,
    );
}

fn weekday(unix_day: i64) -> usize {
    (unix_day + 3).rem_euclid(7) as usize // Unix epoch was a Thursday; Monday = 0.
}

fn day_bounds(first_day: i64, index: usize) -> Rectangle {
    let cell = weekday(first_day) + index;
    Rectangle {
        x: GRID_X + (cell / 7) as f32 * CELL,
        y: GRID_Y + (cell % 7) as f32 * CELL,
        width: 8.,
        height: 8.,
    }
}

fn activity_color(count: u64, peak: u64) -> iced::Color {
    if count == 0 {
        theme::palette().background
    } else {
        // Four levels relative to this year's busiest day; zero remains distinct.
        let level = (count as f64 / peak.max(1) as f64 * 4.)
            .ceil()
            .clamp(1., 4.);
        theme::alpha(theme::palette().blue, (0.25 + level * 0.1875) as f32)
    }
}

fn activity(frame: &mut Frame, data: Option<&DashboardData>, pointer: Option<Point>) {
    caption(frame, "Activity", 57., 297., 90., 14.);
    caption(frame, "Last 12 months", 151., 300., 130., 10.);
    caption(frame, "Less", 527., 301., 28., 8.);
    for level in 0..=4 {
        frame.fill_rectangle(
            Point::new(559. + level as f32 * 10., 302.),
            Size::new(8., 8.),
            activity_color(level, 4),
        );
    }
    caption(frame, "More", 612., 301., 30., 8.);
    for (day, row) in [("Mon", 0), ("Wed", 2), ("Fri", 4)] {
        caption(frame, day, 65., GRID_Y + row as f32 * CELL - 1., 30., 9.);
    }
    let Some(days) = data
        .map(|data| data.activity.as_slice())
        .filter(|days| !days.is_empty())
    else {
        caption(frame, NA, 330., 356., 80., 16.);
        return;
    };
    let peak = days
        .iter()
        .map(|day| day.completed_tests)
        .max()
        .unwrap_or(0);
    let total: u64 = days.iter().map(|day| day.completed_tests).sum();
    caption(frame, &format!("{total} tests"), 292., 300., 210., 10.);
    let mut hovered: Option<&ActivityDay> = None;
    let mut previous_label_x = f32::NEG_INFINITY;
    for (index, day) in days.iter().enumerate() {
        let rect = day_bounds(days[0].unix_day, index);
        let path = Path::rounded_rectangle(rect.position(), rect.size(), 1.5.into());
        frame.fill(&path, activity_color(day.completed_tests, peak));
        if pointer.is_some_and(|p| rect.contains(p)) {
            hovered = Some(day);
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(theme::palette().correct)
                    .with_width(1.),
            );
        }
        let month = &day.date[5..7];
        if day.date.ends_with("-01") && rect.x >= previous_label_x + 28. && rect.x < 610. {
            let name = match month {
                "01" => "Jan",
                "02" => "Feb",
                "03" => "Mar",
                "04" => "Apr",
                "05" => "May",
                "06" => "Jun",
                "07" => "Jul",
                "08" => "Aug",
                "09" => "Sep",
                "10" => "Oct",
                "11" => "Nov",
                _ => "Dec",
            };
            caption(frame, name, rect.x, 403., 28., 8.);
            previous_label_x = rect.x;
        }
    }
    let detail = hovered.map_or_else(
        || "Daily completed tests · UTC".into(),
        |day| {
            format!(
                "{} · {} completed tests · UTC",
                day.date, day.completed_tests
            )
        },
    );
    caption(frame, &detail, 102., 422., 535., 9.);
}

fn histogram(frame: &mut Frame, data: Option<&DashboardData>, pointer: Option<Point>) {
    caption(frame, "All-time WPM", 57., 458., 150., 12.);
    let maximum_count = data.map_or(0, |data| *data.bins.iter().max().unwrap());
    let step = ((maximum_count as f64 / 3.).ceil() as u64).max(1);
    let maximum = step as f64 * 3.;
    rotated(frame, "Tests", Point::new(56., 524.), 9., theme::FONT);
    for tick in 0..=3 {
        let y = PLOT.y + PLOT.height * (1. - tick as f32 / 3.);
        frame.stroke(
            &Path::line(Point::new(PLOT.x, y), Point::new(PLOT.x + PLOT.width, y)),
            Stroke::default()
                .with_color(theme::alpha(theme::palette().pending, 0.4))
                .with_width(0.5),
        );
        if data.is_some() {
            label(
                frame,
                &metric(Some(tick as f64 * step as f64), false),
                Rectangle {
                    x: 61.,
                    y: y - 5.,
                    width: 25.,
                    height: 10.,
                },
                8.,
                theme::FONT,
            );
        }
    }
    let Some(data) = data else {
        caption(frame, NA, 330., 500., 80., 16.);
        return;
    };
    let cell = PLOT.width / 12.;
    let width = data.bin_width.max(10.);
    let mut detail = if data.summary.completed_tests == 0 {
        "No completed tests yet".into()
    } else {
        "".into()
    };
    for (i, count) in data.bins.iter().enumerate() {
        let x = PLOT.x + cell * i as f32;
        let height = (*count as f64 / maximum) as f32 * PLOT.height;
        let hit = Rectangle {
            x,
            y: PLOT.y,
            width: cell,
            height: PLOT.height,
        };
        let hovered = pointer.is_some_and(|p| hit.contains(p));
        if *count > 0 {
            frame.fill_rectangle(
                Point::new(x + cell * 0.14, PLOT.y + PLOT.height - height),
                Size::new(cell * 0.72, height),
                if hovered {
                    theme::palette().correct
                } else {
                    theme::palette().blue
                },
            );
        }
        let low = metric(Some(i as f64 * width), false);
        let high = metric(Some((i + 1) as f64 * width - 1.), false);
        label(
            frame,
            &format!("{low}–{high}"),
            Rectangle {
                x,
                y: 541.,
                width: cell - 2.,
                height: 12.,
            },
            8.,
            theme::FONT,
        );
        if hovered {
            detail = format!(
                "{count} tests · {low}–<{} WPM",
                metric(Some((i + 1) as f64 * width), false)
            );
        }
    }
    caption(frame, &detail, 230., 459., 409., 10.);
}

pub(super) fn draw(frame: &mut Frame, data: Option<&DashboardData>, pointer: Option<Point>) {
    for (y, height) in [(287., 156.), (453., 106.)] {
        frame.fill(
            &Path::rounded_rectangle(Point::new(45., y), Size::new(610., height), 4.into()),
            theme::palette().control_background,
        );
    }
    activity(frame, data, pointer);
    histogram(frame, data, pointer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_cells_align_monday_first_and_fit_leap_years() {
        assert_eq!(weekday(0), 3);
        for start in 0..7 {
            let first = day_bounds(start, 0);
            assert_eq!(first.y, GRID_Y + weekday(start) as f32 * CELL);
            for index in 0..366 {
                let bounds = day_bounds(start, index);
                assert!(bounds.x >= GRID_X && bounds.x + bounds.width <= 641.);
                assert!(bounds.y >= GRID_Y && bounds.y + bounds.height <= 399.);
                if index >= 7 {
                    let previous = day_bounds(start, index - 7);
                    assert_eq!(bounds.y, previous.y);
                    assert_eq!(bounds.x, previous.x + CELL);
                }
            }
        }
    }
}
