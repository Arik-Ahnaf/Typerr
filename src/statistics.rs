//! Session measurements, independent of Iced and wall-clock time.
//!
//! WPM counts retained correct characters; raw WPM counts all retained characters.
//! Both use five characters per word. Accuracy includes every printable attempt,
//! even if later deleted. Consistency is 100 * (1 - population CV), clamped to
//! 0..100, of attempted characters/second in one-second intervals. Idle intervals
//! count as zero; the last partial interval is weighted by its actual duration.
use std::{collections::BTreeMap, time::Duration};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Score {
    pub wpm: f64,
    pub accuracy: f64,
    pub raw: f64,
    pub consistency: f64,
    pub elapsed: Duration,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub seconds: f64,
    pub wpm: f64,
    pub raw: f64,
    pub errors: usize,
}

#[derive(Debug, Clone, Copy)]
struct Edit {
    elapsed: Duration,
    retained: usize,
    correct: usize,
    attempt: bool,
    error: bool,
}

#[derive(Debug, Default)]
pub struct Statistics {
    edits: Vec<Edit>,
    retained: usize,
    correct: usize,
    attempts: usize,
    mistakes: usize,
}

impl Statistics {
    /// Call only after the engine accepts a printable scalar.
    pub fn input(&mut self, elapsed: Duration, correct: bool) {
        self.retained += 1;
        self.correct += usize::from(correct);
        self.attempts += 1;
        self.mistakes += usize::from(!correct);
        self.record(elapsed, true, !correct);
    }

    /// Call only after successful Backspace, using the deleted scalar's state.
    pub fn backspace(&mut self, elapsed: Duration, correct: bool) {
        self.retained -= 1;
        self.correct -= usize::from(correct);
        self.record(elapsed, false, false);
    }

    fn record(&mut self, elapsed: Duration, attempt: bool, error: bool) {
        self.edits.push(Edit {
            elapsed,
            retained: self.retained,
            correct: self.correct,
            attempt,
            error,
        });
    }

    pub fn finish(&self, elapsed: Duration) -> Score {
        let seconds = elapsed.as_secs_f64();
        let mut score = Score {
            wpm: rate(self.correct, seconds),
            raw: rate(self.retained, seconds),
            accuracy: if self.attempts == 0 {
                0.
            } else {
                100. * (self.attempts - self.mistakes) as f64 / self.attempts as f64
            },
            elapsed,
            ..Score::default()
        };
        if seconds == 0. {
            // A single committed-text event can finish at the start instant.
            // There is no measurable speed or consistency in that case.
            return score;
        }

        let mut intervals = BTreeMap::<u64, usize>::new();
        for edit in &self.edits {
            if edit.attempt {
                let second = (edit.elapsed.as_secs_f64().ceil() as u64).max(1);
                *intervals.entry(second).or_default() += 1;
            }
        }
        let mean = self.attempts as f64 / seconds;
        // Empty intervals contribute zero to the second moment, but their time
        // remains in the denominator. This also handles very long idle sessions.
        let second_moment = intervals
            .iter()
            .map(|(&second, &count)| {
                let duration = (seconds - (second - 1) as f64).clamp(0., 1.);
                if duration > 0. {
                    (count as f64).powi(2) / duration
                } else {
                    0.
                }
            })
            .sum::<f64>()
            / seconds;
        if mean > 0. {
            let deviation = (second_moment - mean * mean).max(0.).sqrt();
            score.consistency = (100. * (1. - deviation / mean)).clamp(0., 100.);
        }

        // At most 300 graph samples, even if a Words test is left idle for days.
        // Normally one sample/second; very long tests aggregate wider intervals.
        let step = (seconds / 300.).ceil().max(1.);
        let count = (seconds / step).ceil() as usize;
        let mut edits = self.edits.iter().peekable();
        let (mut retained, mut correct) = (0, 0);
        for index in 1..=count {
            let end = (index as f64 * step).min(seconds);
            let mut errors = 0;
            while let Some(edit) = edits.next_if(|edit| edit.elapsed.as_secs_f64() <= end) {
                retained = edit.retained;
                correct = edit.correct;
                errors += usize::from(edit.error);
            }
            score.samples.push(Sample {
                seconds: end,
                wpm: rate(correct, end),
                raw: rate(retained, end),
                errors,
            });
        }
        score
    }
}

fn rate(characters: usize, seconds: f64) -> f64 {
    if seconds > 0. {
        characters as f64 * 12. / seconds
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(seconds: f64) -> Duration {
        Duration::from_secs_f64(seconds)
    }
    fn near(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
    }

    #[test]
    fn steady_typing_has_exact_speed_accuracy_and_consistency() {
        let mut stats = Statistics::default();
        for i in 0..50 {
            stats.input(at(i as f64 / 5.), true);
        }
        let score = stats.finish(at(10.));
        near(score.wpm, 60.);
        near(score.raw, 60.);
        near(score.accuracy, 100.);
        // Put five events in each interval, including the starting character.
        let mut steady = Statistics::default();
        for i in 0..50 {
            steady.input(at(i as f64 / 5. + 0.1), true);
        }
        near(steady.finish(at(10.)).consistency, 100.);
        assert_eq!(score.samples.len(), 10);
        near(score.samples.last().unwrap().wpm, score.wpm);
    }

    #[test]
    fn corrections_restore_speed_but_preserve_accuracy_and_error_history() {
        let mut stats = Statistics::default();
        stats.input(at(0.), true);
        stats.input(at(0.2), false);
        stats.backspace(at(0.4), false);
        stats.input(at(0.6), true);
        stats.backspace(at(0.8), true);
        stats.input(at(1.2), false);
        let score = stats.finish(at(2.));
        near(score.wpm, 6.);
        near(score.raw, 12.);
        near(score.accuracy, 50.);
        assert_eq!(score.samples.iter().map(|s| s.errors).sum::<usize>(), 2);
        near(score.samples[0].wpm, 12.);
        near(score.samples[1].raw, score.raw);
    }

    #[test]
    fn pauses_and_partial_intervals_are_duration_weighted() {
        let mut stats = Statistics::default();
        for t in [0., 0.3, 0.6, 0.9, 1.2, 1.4] {
            stats.input(at(t), true);
        }
        near(stats.finish(at(1.5)).consistency, 100.);
        let paused = stats.finish(at(3.));
        near(paused.consistency, 100. * (1. - (8.0_f64 / 3.).sqrt() / 2.));
        assert_eq!(paused.samples[2].errors, 0);
        near(paused.samples[2].wpm, 24.);
        assert!(paused.consistency < stats.finish(at(1.5)).consistency);
    }

    #[test]
    fn empty_instant_and_long_sessions_are_bounded_and_finite() {
        let mut stats = Statistics::default();
        assert_eq!(stats.finish(Duration::ZERO), Score::default());
        stats.input(Duration::ZERO, false);
        let instant = stats.finish(Duration::ZERO);
        near(instant.wpm, 0.);
        near(instant.raw, 0.);
        near(instant.consistency, 0.);
        let long = stats.finish(at(86400.));
        assert!(long.samples.len() <= 300);
        near(long.samples.last().unwrap().raw, long.raw);
        near(long.consistency, 0.);
    }
}
