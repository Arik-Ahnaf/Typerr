use std::time::{Duration, Instant};
use typerr::layout::CaretTarget;

// Monkeytype's medium speed and Anime.js inOut(1.25) power easing:
// https://github.com/monkeytypegame/monkeytype/blob/master/frontend/src/ts/elements/caret.ts
// Keep presentation tuning here; typing and measured layout do not depend on it.
const DURATION: Duration = Duration::from_millis(100);
const EASING_POWER: f32 = 1.25;

/// Panel-relative presentation coordinates, independent of logical progress.
#[derive(Default)]
pub struct Caret {
    pub target: CaretTarget,
    pub rendered: CaretTarget,
    start: CaretTarget,
    started_at: Option<Instant>,
}

impl Caret {
    pub fn snap_to(&mut self, target: CaretTarget) {
        self.target = target;
        self.rendered = target;
        self.start = target;
        self.started_at = None;
    }

    pub fn retarget(&mut self, target: CaretTarget, now: Instant) {
        // Layout and countdown updates must not prolong an existing transition.
        if target == self.target {
            return;
        }
        self.advance(now);
        self.start = self.rendered;
        self.target = target;
        self.started_at = (self.start != target).then_some(now);
    }

    /// Sample elapsed time, returning whether another animation frame is needed.
    pub fn advance(&mut self, now: Instant) -> bool {
        let Some(started_at) = self.started_at else {
            return false;
        };
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed >= DURATION {
            self.snap_to(self.target);
            return false;
        }
        let progress = ease(elapsed.as_secs_f32() / DURATION.as_secs_f32());
        self.rendered = CaretTarget {
            x: self.start.x + (self.target.x - self.start.x) * progress,
            y: self.start.y + (self.target.y - self.start.y) * progress,
        };
        true
    }
}

fn ease(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    if t < 0.5 {
        0.5 * (2.0 * t).powf(EASING_POWER)
    } else {
        1.0 - 0.5 * (2.0 - 2.0 * t).powf(EASING_POWER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> CaretTarget {
        CaretTarget { x, y }
    }

    #[test]
    fn easing_is_bounded_symmetric_and_monotonic() {
        assert_eq!(ease(-1.0), 0.0);
        assert_eq!(ease(0.5), 0.5);
        assert_eq!(ease(2.0), 1.0);
        assert!((ease(0.25) - 0.210_224_1).abs() < 0.000_001);
        for step in 1..=1000 {
            let t = step as f32 / 1000.0;
            assert!(ease(t) >= ease(t - 0.001));
            assert!((ease(t) + ease(1.0 - t) - 1.0).abs() < 0.000_001);
        }
    }

    #[test]
    fn elapsed_time_controls_motion_and_completion_without_frame_counting() {
        let now = Instant::now();
        let mut caret = Caret::default();
        caret.snap_to(point(10.0, 20.0));
        caret.retarget(point(30.0, 40.0), now);
        assert!(caret.advance(now - Duration::from_millis(1)));
        assert_eq!(caret.rendered, point(10.0, 20.0));
        assert!(caret.advance(now + DURATION / 2));
        assert_eq!(caret.rendered, point(20.0, 30.0));
        assert!(!caret.advance(now + DURATION));
        assert_eq!(caret.rendered, caret.target);
        caret.retarget(point(60.0, 0.0), now + DURATION);
        assert!(!caret.advance(now + Duration::from_secs(5)));
        assert_eq!(caret.rendered, point(60.0, 0.0));
    }

    #[test]
    fn retargeting_samples_current_motion_and_reverses_without_jumping() {
        let now = Instant::now();
        let mut caret = Caret::default();
        caret.retarget(point(100.0, 20.0), now);
        caret.retarget(point(-50.0, 10.0), now + DURATION / 2);
        assert_eq!(caret.rendered, point(50.0, 10.0));
        assert!(caret.advance(now + DURATION));
        assert_eq!(caret.rendered, point(0.0, 10.0));
        assert!(!caret.advance(now + DURATION + DURATION / 2));
        assert_eq!(caret.rendered, point(-50.0, 10.0));
    }

    #[test]
    fn rapid_targets_converge_to_the_latest_without_a_queue() {
        let now = Instant::now();
        let mut caret = Caret::default();
        for step in 1..=100 {
            let time = now + Duration::from_millis(step);
            caret.advance(time);
            let previous = caret.rendered;
            caret.retarget(point(step as f32 * 10.0, 0.0), time);
            assert_eq!(caret.rendered, previous);
        }
        assert!(!caret.advance(now + Duration::from_millis(200)));
        assert_eq!(caret.rendered, point(1000.0, 0.0));
    }

    #[test]
    fn repeated_targets_do_not_restart_and_snap_cancels_motion() {
        let now = Instant::now();
        let mut caret = Caret::default();
        caret.retarget(point(100.0, 0.0), now);
        caret.retarget(point(100.0, 0.0), now + DURATION / 2);
        assert!(!caret.advance(now + DURATION));
        assert_eq!(caret.rendered.x, 100.0);
        caret.retarget(point(0.0, 0.0), now + DURATION);
        caret.snap_to(point(25.0, 30.0));
        assert!(!caret.advance(now + Duration::from_secs(1)));
        assert_eq!(caret.rendered, point(25.0, 30.0));
    }
}
