use crate::ui::typing_panel::TypingPanel;
use iced::{
    Element, Event, keyboard,
    widget::{Space, container, mouse_area, stack},
    window,
};
use std::time::{Duration, Instant};
use typerr::{
    engine::TypingSession,
    statistics::{Score, Statistics},
    words,
};

const TIME_LIMIT_SECONDS: u64 = 30;
const WORD_TARGET: usize = 50;
const FADE_DURATION: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    Typing,
    Transition { started: Instant, now: Instant },
    Score,
}
impl Page {
    fn score_opacity(self) -> f32 {
        match self {
            Self::Typing => 0.0,
            Self::Score => 1.0,
            Self::Transition { started, now } => {
                let t = (now.saturating_duration_since(started).as_secs_f32()
                    / FADE_DURATION.as_secs_f32())
                .clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Words,
    Time,
}

impl Mode {
    pub const fn amounts(self) -> [u16; 4] {
        match self {
            Self::Words => [10, 25, 50, 100],
            Self::Time => [15, 30, 60, 120],
        }
    }
}

pub struct App {
    main: MainTyping,
    page: Page,
    focused: bool,
    settings: crate::settings::Settings,
}

struct MainTyping {
    session: TypingSession,
    mode: Mode,
    options: words::Options,
    word_target: usize,
    time_limit_seconds: u64,
    amount_picker: Option<Mode>,
    deadline: Option<Instant>,
    started: Option<Instant>,
    statistics: Statistics,
    score: Score,
    seconds_remaining: u64,
    /// Changes on accepted edits or explicit session resets, restoring the viewport.
    revision: u64,
    /// Changes only when a session is replaced, even if its text is identical.
    generation: u64,
}

impl MainTyping {
    fn restart(&mut self) {
        self.replace(words::generate(&mut rand::rng(), self.options));
    }

    fn replace(&mut self, target: String) {
        self.session = TypingSession::new(target);
        self.deadline = None;
        self.started = None;
        self.statistics = Statistics::default();
        self.score = Score::default();
        self.seconds_remaining = self.time_limit_seconds;
        self.amount_picker = None;
        self.revision = self.revision.wrapping_add(1);
        self.generation = self.generation.wrapping_add(1);
    }

    fn elapsed(&self, now: Instant) -> Duration {
        self.started
            .map_or(Duration::ZERO, |start| now.saturating_duration_since(start))
    }

    fn tick(&mut self, now: Instant) {
        if let Some(deadline) = self.deadline {
            let remaining = deadline.saturating_duration_since(now);
            self.seconds_remaining = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
        }
    }

    fn next_tick(&self) -> Option<Instant> {
        self.seconds_remaining.checked_sub(1).and_then(|seconds| {
            self.deadline
                .map(|deadline| deadline - Duration::from_secs(seconds))
        })
    }

    fn monitor(&self) -> String {
        match self.mode {
            Mode::Words => format!(
                "{}/{}",
                self.session.completed_words().min(self.word_target),
                self.word_target
            ),
            Mode::Time => format!("{}s", self.seconds_remaining),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Native(Event),
    OpenConnection(&'static str),
    SelectMode(Mode),
    ToggleAmountPicker(Mode),
    SelectAmount(Mode, u16),
    DismissAmountPicker,
    TogglePunctuation,
    ToggleNumbers,
    Next,
    Retry,
}

impl Default for App {
    fn default() -> Self {
        Self {
            main: MainTyping {
                session: TypingSession::new(words::generate(
                    &mut rand::rng(),
                    words::Options::default(),
                )),
                mode: Mode::default(),
                options: words::Options::default(),
                word_target: WORD_TARGET,
                time_limit_seconds: TIME_LIMIT_SECONDS,
                amount_picker: None,
                deadline: None,
                started: None,
                statistics: Statistics::default(),
                score: Score::default(),
                seconds_remaining: TIME_LIMIT_SECONDS,
                revision: 0,
                generation: 0,
            },
            page: Page::Typing,
            focused: true,
            settings: crate::settings::Settings::default(),
        }
    }
}

impl App {
    pub fn new(settings: crate::settings::Settings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }

    pub fn update(&mut self, message: Message) {
        self.update_at(message, Instant::now());
    }

    fn advance(&mut self, now: Instant) {
        if self.page == Page::Typing {
            self.main.tick(now);
            if self.main.mode == Mode::Time
                && self.main.deadline.is_some_and(|deadline| now >= deadline)
            {
                self.finish(self.main.deadline.expect("expired deadline"));
            }
        }
        if let Page::Transition { started, .. } = self.page {
            self.page = if now.saturating_duration_since(started) >= FADE_DURATION {
                Page::Score
            } else {
                Page::Transition { started, now }
            };
        }
    }

    fn finish(&mut self, now: Instant) {
        if self.page == Page::Typing {
            self.main.amount_picker = None;
            self.main.score = self.main.statistics.finish(self.main.elapsed(now));
            self.page = Page::Transition { started: now, now };
        }
    }

    fn finished(&self) -> bool {
        self.main.session.status() == typerr::engine::SessionStatus::Complete
            || (self.main.mode == Mode::Words
                && self.main.session.completed_words() >= self.main.word_target)
    }

    fn update_at(&mut self, message: Message, now: Instant) {
        let now = if let Message::Native(Event::Window(window::Event::RedrawRequested(timestamp))) =
            &message
        {
            *timestamp
        } else {
            now
        };
        // Enforce the time limit even if an input arrives before a delayed redraw.
        self.advance(now);
        match message {
            Message::Next if self.page == Page::Score => {
                self.main.restart();
                self.page = Page::Typing;
            }
            Message::Retry if self.page == Page::Score => {
                self.main
                    .replace(self.main.session.target().iter().collect());
                self.page = Page::Typing;
            }
            Message::TogglePunctuation if self.page == Page::Typing => {
                self.main.options.punctuation = !self.main.options.punctuation;
                self.main.restart();
            }
            Message::ToggleNumbers if self.page == Page::Typing => {
                self.main.options.numbers = !self.main.options.numbers;
                self.main.restart();
            }
            Message::SelectMode(mode) if self.page == Page::Typing => {
                self.main.amount_picker = None;
                if self.main.mode != mode {
                    self.main.mode = mode;
                    self.main.restart();
                }
            }
            Message::ToggleAmountPicker(mode) if self.page == Page::Typing => {
                self.main.amount_picker = (self.main.amount_picker != Some(mode)).then_some(mode);
            }
            Message::SelectAmount(mode, amount)
                if self.page == Page::Typing
                    && self.main.amount_picker == Some(mode)
                    && mode.amounts().contains(&amount) =>
            {
                match mode {
                    Mode::Words => self.main.word_target = usize::from(amount),
                    Mode::Time => self.main.time_limit_seconds = u64::from(amount),
                }
                self.main.mode = mode;
                self.main.restart();
            }
            Message::DismissAmountPicker => self.main.amount_picker = None,
            Message::OpenConnection(url) if self.page == Page::Typing => open_connection(url),
            Message::Native(Event::Window(window::Event::Focused)) => self.focused = true,
            Message::Native(Event::Window(window::Event::Unfocused)) => {
                self.focused = false;
                self.main.amount_picker = None;
            }
            Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                repeat,
                ..
            })) if self.focused => {
                if modifiers.control() || modifiers.alt() || modifiers.logo() {
                    return;
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Tab) {
                    if !repeat && !modifiers.shift() {
                        self.main.restart();
                        self.page = Page::Typing;
                    }
                    return;
                }
                if self.page != Page::Typing {
                    return;
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                    self.main.amount_picker = None;
                    return;
                }
                let changed = if key == keyboard::Key::Named(keyboard::key::Named::Backspace) {
                    let correct = self
                        .main
                        .session
                        .position()
                        .checked_sub(1)
                        .and_then(|i| self.main.session.character_state(i))
                        == Some(typerr::engine::CharacterState::Correct);
                    let changed = self.main.session.backspace();
                    if changed {
                        self.main
                            .statistics
                            .backspace(self.main.elapsed(now), correct);
                    }
                    changed
                } else {
                    let mut changed = false;
                    if let Some(text) = text {
                        for c in text.chars() {
                            let correct =
                                self.main.session.target().get(self.main.session.position())
                                    == Some(&c);
                            if self.main.session.input(c) {
                                self.main.started.get_or_insert(now);
                                self.main.statistics.input(self.main.elapsed(now), correct);
                                if self.main.mode == Mode::Time && self.main.deadline.is_none() {
                                    self.main.deadline = Some(
                                        now + Duration::from_secs(self.main.time_limit_seconds),
                                    );
                                }
                                changed = true;
                            }
                            if self.finished() {
                                self.finish(now);
                                break;
                            }
                        }
                    }
                    changed
                };
                if changed {
                    self.main.amount_picker = None;
                    self.main.revision = self.main.revision.wrapping_add(1);
                }
            }
            _ => {}
        }
    }

    fn typing_view(&self, opacity: f32, enabled: bool) -> Element<'_, Message> {
        let panel = container(
            TypingPanel::new(
                &self.main.session,
                self.main.revision,
                self.main.generation,
                self.main.monitor(),
                self.settings,
            )
            .presentation(opacity, enabled),
        )
        .max_width(crate::ui::theme::TYPING_MAX_WIDTH)
        .max_height(crate::ui::theme::typing_height(self.settings.line_height()));
        let typing = container(panel)
            .padding(iced::Padding {
                top: crate::ui::theme::VERTICAL_CLEARANCE,
                bottom: crate::ui::theme::VERTICAL_CLEARANCE,
                left: 30.0,
                right: 30.0,
            })
            .width(iced::Fill)
            .align_x(iced::Center)
            .align_y(iced::Center)
            .height(iced::Fill);
        let dismiss: Element<'_, Message> = if enabled && self.main.amount_picker.is_some() {
            mouse_area(Space::new().width(iced::Fill).height(iced::Fill))
                .on_press(Message::DismissAmountPicker)
                .on_right_press(Message::DismissAmountPicker)
                .into()
        } else {
            Space::new().into()
        };
        stack![
            typing,
            container(crate::ui::chrome::footer(opacity, enabled))
                .height(iced::Fill)
                .align_y(iced::Bottom),
            dismiss,
            crate::ui::chrome::header(
                self.main.mode,
                self.main.options,
                self.main.amount_picker,
                opacity,
                enabled
            ),
        ]
        .into()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let opacity = self.page.score_opacity();
        let typing: Element<'_, Message> = if self.page == Page::Score {
            Space::new().width(iced::Fill).height(iced::Fill).into()
        } else {
            self.typing_view(1.0 - opacity, self.page == Page::Typing)
        };
        let score: Element<'_, Message> = if self.page == Page::Typing {
            Space::new().into()
        } else {
            container(crate::ui::score::view(
                &self.main.score,
                opacity,
                self.page == Page::Score,
            ))
            .center_x(iced::Fill)
            .center_y(iced::Fill)
            .into()
        };
        crate::ui::page_host::PageHost::new(
            stack![typing, score, crate::ui::chrome::brand()],
            if self.page == Page::Typing {
                self.main.next_tick()
            } else {
                None
            },
            matches!(self.page, Page::Transition { .. }),
        )
        .into()
    }
}

fn open_connection(url: &'static str) {
    // Hand off to the OS browser off the input thread and reap the launcher.
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let result = {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .creation_flags(0x0800_0000)
                .status()
        };
        #[cfg(not(target_os = "windows"))]
        let result = std::process::Command::new("xdg-open").arg(url).status();
        if let Err(error) = result {
            eprintln!("Could not open connection: {error}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::{
        Key, Location, Modifiers,
        key::{Named, NativeCode, Physical},
    };

    fn key(key: Key, text: Option<&str>, modifiers: Modifiers) -> Message {
        Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers,
            text: text.map(Into::into),
            repeat: false,
        }))
    }

    fn app(target: &str) -> App {
        App {
            main: MainTyping {
                session: TypingSession::new(target),
                mode: Mode::default(),
                options: words::Options::default(),
                word_target: WORD_TARGET,
                time_limit_seconds: TIME_LIMIT_SECONDS,
                amount_picker: None,
                deadline: None,
                started: None,
                statistics: Statistics::default(),
                score: Score::default(),
                seconds_remaining: TIME_LIMIT_SECONDS,
                revision: 0,
                generation: 0,
            },
            page: Page::Typing,
            focused: true,
            settings: crate::settings::Settings::default(),
        }
    }

    #[test]
    fn score_tracks_corrections_unicode_spaces_and_freezes_at_completion() {
        let mut app = app("ab c");
        let start = Instant::now();
        let send = |app: &mut App, text: &str, seconds| {
            app.update_at(
                key(Key::Character("x".into()), Some(text), Modifiers::empty()),
                start + Duration::from_secs(seconds),
            )
        };
        send(&mut app, "a", 0);
        send(&mut app, "é", 1);
        app.update_at(
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            start + Duration::from_secs(2),
        );
        send(&mut app, "b", 3);
        send(&mut app, "x", 4); // Wrong expected separator counts as a mistake.
        send(&mut app, "cSURPLUS", 6);
        let score = app.main.score.clone();
        assert_eq!(score.elapsed, Duration::from_secs(6));
        assert_eq!(score.wpm, 6.); // Three correct retained scalars.
        assert_eq!(score.raw, 8.);
        assert_eq!(score.accuracy, 60.); // Three correct attempts out of five.
        assert_eq!(score.samples.iter().map(|s| s.errors).sum::<usize>(), 2);
        assert_eq!(score.samples.last().unwrap().wpm, score.wpm);
        send(&mut app, "late", 20);
        assert_eq!(app.main.score, score);
        app.update_at(Message::Retry, start + Duration::from_secs(21));
        assert_eq!(app.main.score, Score::default());
        assert!(app.main.started.is_none());
        send(&mut app, "ab", 22);
        send(&mut app, " c", 24);
        assert_eq!(app.main.score.accuracy, 100.);
        assert_eq!(app.main.score.wpm, 24.);
        assert_eq!(app.main.score.elapsed, Duration::from_secs(2));
    }

    #[test]
    fn timed_score_uses_exact_deadline_including_idle_unfocused_time() {
        for seconds in Mode::Time.amounts() {
            let mut app = app("abc def");
            app.main.mode = Mode::Time;
            app.main.time_limit_seconds = u64::from(seconds);
            let start = Instant::now();
            app.update_at(
                key(Key::Named(Named::Backspace), None, Modifiers::empty()),
                start,
            );
            app.update_at(
                key(Key::Character("x".into()), Some("\n"), Modifiers::empty()),
                start,
            );
            assert!(app.main.started.is_none());
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                start,
            );
            app.update_at(
                Message::Native(Event::Window(window::Event::Unfocused)),
                start + Duration::from_secs(1),
            );
            tick(
                &mut app,
                start + Duration::from_secs(u64::from(seconds) + 10),
            );
            assert_eq!(
                app.main.score.elapsed,
                Duration::from_secs(u64::from(seconds))
            );
            assert_eq!(app.main.score.wpm, 12. / f64::from(seconds));
            assert_eq!(app.main.score.accuracy, 100.);
            assert_eq!(app.main.score.consistency, 0.);
            assert_eq!(app.main.score.samples.len(), usize::from(seconds));
        }
    }

    #[test]
    fn all_session_replacements_clear_measurements() {
        for message in [
            Message::Next,
            Message::Retry,
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            Message::SelectMode(Mode::Time),
            Message::ToggleNumbers,
            Message::TogglePunctuation,
            Message::SelectAmount(Mode::Words, 10),
        ] {
            let mut app = app("abc def");
            let start = Instant::now();
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                start,
            );
            if matches!(message, Message::Next | Message::Retry) {
                app.finish(start + Duration::from_secs(1));
                tick(&mut app, start + Duration::from_secs(2));
            }
            if matches!(message, Message::SelectAmount(..)) {
                app.main.amount_picker = Some(Mode::Words);
            }
            app.update_at(message, start + Duration::from_secs(3));
            assert!(app.main.started.is_none());
            assert_eq!(app.main.score, Score::default());
            assert_eq!(app.main.statistics.finish(Duration::ZERO), Score::default());
        }
    }

    fn select_amount(app: &mut App, mode: Mode, amount: u16) {
        app.update(Message::ToggleAmountPicker(mode));
        app.update(Message::SelectAmount(mode, amount));
    }

    #[test]
    fn every_word_amount_finishes_at_its_final_character_including_punctuation() {
        for amount in Mode::Words.amounts() {
            let mut app = App::default();
            select_amount(&mut app, Mode::Words, amount);
            assert_eq!(app.main.session.word_count(), 500);
            app.main.replace(vec!["ab,"; 500].join(" "));
            let now = Instant::now();
            let prefix = format!("{}xx", "xxx ".repeat(usize::from(amount) - 1));
            app.update_at(
                key(
                    Key::Character("x".into()),
                    Some(&prefix),
                    Modifiers::empty(),
                ),
                now,
            );
            assert_eq!(app.page, Page::Typing);
            assert_eq!(app.main.monitor(), format!("{}/{amount}", amount - 1));
            app.update_at(
                key(
                    Key::Character("x".into()),
                    Some("x surplus"),
                    Modifiers::empty(),
                ),
                now,
            );
            assert!(matches!(app.page, Page::Transition { .. }));
            assert_eq!(app.main.monitor(), format!("{amount}/{amount}"));
            assert_eq!(app.main.session.position(), usize::from(amount) * 4 - 1);
        }
    }

    #[test]
    fn every_time_amount_starts_on_input_and_expires_before_late_edits() {
        for amount in Mode::Time.amounts() {
            let mut app = App::default();
            select_amount(&mut app, Mode::Time, amount);
            assert_eq!(app.main.monitor(), format!("{amount}s"));
            assert_eq!(app.main.deadline, None);
            let now = Instant::now();
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                now,
            );
            assert_eq!(app.main.next_tick(), Some(now + Duration::from_secs(1)));
            let deadline = now + Duration::from_secs(u64::from(amount));
            assert_eq!(app.main.deadline, Some(deadline));
            tick(&mut app, deadline - Duration::from_millis(1));
            assert_eq!(app.main.monitor(), "1s");
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                deadline,
            );
            assert_eq!(app.main.session.position(), 1);
            assert_eq!(app.main.monitor(), "0s");
            assert!(matches!(app.page, Page::Transition { .. }));
        }
    }

    #[test]
    fn amounts_are_remembered_independently_across_modes_toggles_tab_next_and_retry() {
        let mut app = App::default();
        select_amount(&mut app, Mode::Words, 25);
        select_amount(&mut app, Mode::Time, 120);
        app.update(Message::SelectMode(Mode::Words));
        assert_eq!(app.main.monitor(), "0/25");
        app.update(Message::SelectMode(Mode::Time));
        assert_eq!(app.main.monitor(), "120s");
        for action in [
            Message::ToggleNumbers,
            Message::TogglePunctuation,
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            Message::Next,
            Message::Retry,
        ] {
            let target = app.main.session.target().to_vec();
            let retry = matches!(action, Message::Retry);
            if matches!(action, Message::Next | Message::Retry) {
                app.page = Page::Score;
            }
            app.update(action);
            assert_eq!(app.main.word_target, 25);
            assert_eq!(app.main.time_limit_seconds, 120);
            assert_eq!(app.main.monitor(), "120s");
            assert_eq!(app.main.deadline, None);
            assert_eq!(app.main.session.position(), 0);
            if retry {
                assert_eq!(app.main.session.target(), target);
            }
        }
    }

    #[test]
    fn picker_opening_and_dismissal_preserve_session_and_deadline() {
        let mut app = App::default();
        app.update(Message::SelectMode(Mode::Time));
        app.update(key(
            Key::Character("x".into()),
            Some("x"),
            Modifiers::empty(),
        ));
        let generation = app.main.generation;
        let deadline = app.main.deadline;
        for dismiss in [
            Message::DismissAmountPicker,
            Message::ToggleAmountPicker(Mode::Words),
            key(Key::Named(Named::Escape), None, Modifiers::empty()),
            Message::SelectMode(Mode::Time),
            Message::Native(Event::Window(window::Event::Unfocused)),
        ] {
            app.update(Message::ToggleAmountPicker(Mode::Words));
            assert_eq!(app.main.amount_picker, Some(Mode::Words));
            assert_eq!(app.main.mode, Mode::Time);
            app.update(dismiss);
            assert_eq!(app.main.amount_picker, None);
            assert_eq!(app.main.generation, generation);
            assert_eq!(app.main.deadline, deadline);
            assert_eq!(app.main.session.position(), 1);
        }
    }

    #[test]
    fn choosing_amount_resets_once_and_ignores_invalid_or_stale_choices() {
        let mut app = App::default();
        app.update(Message::ToggleAmountPicker(Mode::Time));
        for action in [
            Message::SelectAmount(Mode::Words, 25),
            Message::SelectAmount(Mode::Time, 0),
        ] {
            app.update(action);
            assert_eq!(app.main.generation, 0);
        }
        app.update(Message::SelectAmount(Mode::Time, 60));
        assert_eq!(app.main.generation, 1);
        assert_eq!(app.main.amount_picker, None);
        assert_eq!(app.main.mode, Mode::Time);
        assert_eq!(app.main.monitor(), "60s");
        app.update(Message::SelectAmount(Mode::Time, 60));
        assert_eq!(app.main.generation, 1);
        let now = Instant::now();
        app.finish(now);
        for action in [
            Message::ToggleAmountPicker(Mode::Words),
            Message::SelectAmount(Mode::Words, 10),
        ] {
            app.update_at(action, now);
            assert_eq!(app.main.amount_picker, None);
            assert_eq!(app.main.generation, 1);
        }
    }

    #[test]
    fn native_pointer_events_open_select_dismiss_and_reuse_mode_amounts() {
        use iced::advanced::{
            Layout, Shell, clipboard, layout, mouse, renderer::Headless, widget::Tree,
        };
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            crate::ui::theme::FONT,
            16.0.into(),
            Some("tiny-skia"),
        ))
        .expect("software renderer");
        let mut app = App::default();
        let click = |app: &mut App, x, y, button| {
            let mut view = app.view();
            let mut tree = Tree::new(&view);
            let size = iced::Size::new(700.0, 400.0);
            let node = view.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(iced::Size::ZERO, size),
            );
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(button),
                mouse::Event::ButtonReleased(button),
            ] {
                view.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(iced::Point::new(x, y)),
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &iced::Rectangle::with_size(size),
                );
            }
            drop(view);
            for message in messages {
                app.update(message);
            }
        };
        click(&mut app, 597.0, 45.0, mouse::Button::Right);
        assert_eq!(app.main.amount_picker, Some(Mode::Words));
        assert_eq!(app.main.generation, 0);
        click(&mut app, 570.0, 80.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "0/25");
        assert_eq!(app.main.generation, 1);
        assert_eq!(app.main.amount_picker, None);
        click(&mut app, 562.0, 45.0, mouse::Button::Right);
        assert_eq!(app.main.amount_picker, Some(Mode::Time));
        click(&mut app, 625.0, 80.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "120s");
        click(&mut app, 597.0, 45.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "0/25");
        assert_eq!(app.main.amount_picker, None);
        let generation = app.main.generation;
        click(&mut app, 562.0, 45.0, mouse::Button::Right);
        click(&mut app, 300.0, 200.0, mouse::Button::Left);
        assert_eq!(app.main.amount_picker, None);
        assert_eq!(app.main.generation, generation);
        click(&mut app, 562.0, 45.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "120s");
    }

    #[test]
    fn burst_edits_preserve_generation_and_only_session_resets_change_it() {
        let mut app = app(&"a".repeat(2000));
        for _ in 0..1000 {
            app.update(key(
                Key::Character("a".into()),
                Some("a"),
                Modifiers::empty(),
            ));
        }
        assert_eq!(app.main.session.position(), 1000);
        assert_eq!(app.main.generation, 0);
        for _ in 0..1000 {
            app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        }
        assert_eq!(app.main.session.position(), 0);
        assert_eq!(app.main.generation, 0);
        for (index, message) in [
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            Message::SelectMode(Mode::Time),
            Message::TogglePunctuation,
            Message::ToggleNumbers,
        ]
        .into_iter()
        .enumerate()
        {
            app.update(message);
            assert_eq!(app.main.generation, index as u64 + 1);
            let generation = app.main.generation;
            app.update(Message::SelectMode(app.main.mode));
            app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
            tick(&mut app, Instant::now());
            assert_eq!(app.main.generation, generation);
        }
    }

    #[test]
    fn native_text_is_consumed_once_and_shortcuts_controls_are_ignored() {
        let mut app = app("ab cd");
        app.update(key(
            Key::Character("a".into()),
            Some("a"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.session.position(), 1);
        for modifier in [Modifiers::CTRL, Modifiers::ALT, Modifiers::LOGO] {
            app.update(key(Key::Character("b".into()), Some("b"), modifier));
        }
        app.update(key(
            Key::Named(Named::Enter),
            Some("\r"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.revision, 1);
        app.update(key(Key::Character("B".into()), Some("B"), Modifiers::SHIFT));
        assert_eq!(app.main.session.entered(), &['a', 'B']);
    }

    #[test]
    fn focus_and_terminal_state_gate_edits_and_noops_preserve_revision() {
        let mut app = app("ab");
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.revision, 0);
        app.update(Message::Native(Event::Window(window::Event::Unfocused)));
        app.update(key(
            Key::Character("a".into()),
            Some("a"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.session.position(), 0);
        app.update(Message::Native(Event::Window(window::Event::Focused)));
        app.update(key(
            Key::Character("a".into()),
            Some("abextra"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.session.entered(), &['a', 'b']);
        assert_eq!(app.main.revision, 1);
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.revision, 1);
    }

    #[test]
    fn tab_restarts_active_and_completed_races() {
        for input in ["a", "ab"] {
            let mut app = app("ab");
            app.update(key(
                Key::Character(input.into()),
                Some(input),
                Modifiers::empty(),
            ));
            app.update(key(Key::Named(Named::Tab), Some("\t"), Modifiers::empty()));
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(
                app.main
                    .session
                    .target()
                    .iter()
                    .filter(|c| **c == ' ')
                    .count(),
                499
            );
            assert_eq!(app.main.revision, 2);
            app.update(key(
                Key::Character("a".into()),
                Some("a"),
                Modifiers::empty(),
            ));
            assert_eq!(app.main.session.position(), 1);
        }
    }

    #[test]
    fn tab_ignores_shortcuts_repeat_and_unfocused_windows() {
        let mut app = app("original");
        for modifiers in [
            Modifiers::CTRL,
            Modifiers::ALT,
            Modifiers::LOGO,
            Modifiers::SHIFT,
        ] {
            app.update(key(Key::Named(Named::Tab), None, modifiers));
        }
        let mut held = key(Key::Named(Named::Tab), None, Modifiers::empty());
        if let Message::Native(Event::Keyboard(keyboard::Event::KeyPressed { repeat, .. })) =
            &mut held
        {
            *repeat = true;
        }
        app.update(held);
        app.update(Message::Native(Event::Window(window::Event::Unfocused)));
        app.update(key(Key::Named(Named::Tab), None, Modifiers::empty()));
        assert_eq!(
            app.main.session.target(),
            &"original".chars().collect::<Vec<_>>()
        );
        assert_eq!(app.main.revision, 0);
    }
    fn tick(app: &mut App, now: Instant) {
        app.update_at(
            Message::Native(Event::Window(window::Event::RedrawRequested(now))),
            now,
        );
    }

    #[test]
    fn time_mode_waits_for_printable_input_and_uses_elapsed_time() {
        let mut app = app("abcdef");
        let start = Instant::now();
        app.update_at(Message::SelectMode(Mode::Time), start);
        assert_eq!(app.main.monitor(), "30s");
        assert_eq!(app.main.next_tick(), None);
        for message in [
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            key(Key::Named(Named::Enter), Some("\r"), Modifiers::empty()),
            key(Key::Character("a".into()), Some("a"), Modifiers::CTRL),
        ] {
            app.update_at(message, start);
        }
        tick(&mut app, start + Duration::from_secs(60));
        assert_eq!(app.main.monitor(), "30s");
        assert_eq!(app.main.deadline, None);
        app.update_at(
            key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
            start,
        );
        assert_eq!(app.main.next_tick(), Some(start + Duration::from_secs(1)));
        tick(&mut app, start + Duration::from_millis(999));
        assert_eq!(app.main.monitor(), "30s");
        tick(&mut app, start + Duration::from_secs(1));
        assert_eq!(app.main.monitor(), "29s");
        app.update_at(
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            start + Duration::from_secs(2),
        );
        assert_eq!(app.main.session.position(), 0);
        app.update(Message::Native(Event::Window(window::Event::Unfocused)));
        tick(&mut app, start + Duration::from_millis(12_500));
        assert_eq!(app.main.monitor(), "18s");
        assert_eq!(app.main.next_tick(), Some(start + Duration::from_secs(13)));
        assert_eq!(app.main.deadline, Some(start + Duration::from_secs(30)));
    }

    #[test]
    fn timer_expiry_freezes_session_and_opens_score() {
        let mut app = app("abcdef");
        app.main.mode = Mode::Time;
        let start = Instant::now();
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            start,
        );
        let revision = app.main.revision;
        tick(&mut app, start + Duration::from_secs(30));
        assert_eq!(app.main.monitor(), "0s");
        assert_eq!(app.main.next_tick(), None);
        assert_eq!(app.main.session.entered(), &['a']);
        assert_eq!(app.main.revision, revision);
        tick(&mut app, start + Duration::from_secs(100));
        assert_eq!(app.main.monitor(), "0s");
        app.update_at(
            key(Key::Character("b".into()), Some("b"), Modifiers::empty()),
            start + Duration::from_secs(101),
        );
        assert_eq!(app.main.session.entered(), &['a']);
        assert_eq!(app.page, Page::Score);
        assert_eq!(app.main.monitor(), "0s");
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.session.entered(), &['a']);
    }

    #[test]
    fn mode_changes_and_tab_reset_sessions_but_active_mode_click_is_noop() {
        let mut app = App::default();
        assert_eq!(app.main.mode, Mode::Words);
        assert_eq!(app.main.monitor(), "0/50");
        for mode in [Mode::Time, Mode::Words, Mode::Time] {
            app.update(Message::SelectMode(mode));
            assert_eq!(app.main.mode, mode);
            assert_eq!(app.main.session.word_count(), 500);
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(app.main.deadline, None);
            let start = Instant::now();
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                start,
            );
            tick(&mut app, start + Duration::from_secs(10));
            assert_eq!(
                app.main.seconds_remaining,
                if mode == Mode::Time { 20 } else { 30 }
            );
            let revision = app.main.revision;
            app.update(Message::SelectMode(mode));
            assert_eq!(app.main.session.position(), 1);
            assert_eq!(app.main.revision, revision);
            app.update(key(Key::Named(Named::Tab), None, Modifiers::empty()));
            assert_eq!(app.main.mode, mode);
            assert_eq!(app.main.session.word_count(), 500);
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(app.main.seconds_remaining, 30);
            assert_eq!(app.main.deadline, None);
            assert_eq!(app.main.revision, revision + 1);
            // Leave progress before changing modes on the next iteration.
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                start,
            );
        }
    }
    #[test]
    fn word_target_finishes_on_fiftieth_final_character_and_discards_remaining_text() {
        let mut app = app(&vec!["ab,"; 500].join(" "));
        let now = Instant::now();
        app.update_at(
            key(
                Key::Character("x".into()),
                Some(&"xxx ".repeat(49)),
                Modifiers::empty(),
            ),
            now,
        );
        assert_eq!(app.main.session.completed_words(), 49);
        app.update_at(
            key(Key::Character("x".into()), Some("xx"), Modifiers::empty()),
            now,
        );
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.monitor(), "49/50");
        app.update_at(
            key(
                Key::Character("x".into()),
                Some("x ignored extra"),
                Modifiers::empty(),
            ),
            now,
        );
        assert_eq!(app.main.session.completed_words(), 50);
        assert_eq!(app.main.session.position(), 199);
        assert!(matches!(app.page, Page::Transition { .. }));
        let revision = app.main.revision;
        app.update_at(
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            now,
        );
        app.update_at(Message::ToggleNumbers, now);
        app.update_at(Message::Next, now);
        assert_eq!(app.main.revision, revision);
        assert_eq!(app.main.session.position(), 199);
    }

    #[test]
    fn crossfade_uses_elapsed_time_and_tab_can_cancel() {
        let mut app = app("a");
        let now = Instant::now();
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            now,
        );
        assert_eq!(app.page.score_opacity(), 0.);
        tick(&mut app, now + FADE_DURATION / 2);
        assert!((app.page.score_opacity() - 0.5).abs() < 0.0001);
        app.update_at(
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            now + FADE_DURATION / 2,
        );
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.generation, 1);
        app.finish(now);
        tick(&mut app, now + FADE_DURATION);
        assert_eq!(app.page, Page::Score);
        assert_eq!(app.page.score_opacity(), 1.);
        tick(&mut app, now + Duration::from_secs(60));
        assert_eq!(app.page, Page::Score);
    }

    #[test]
    fn expiry_precedes_input_and_catches_up_after_focus_loss() {
        for elapsed in [Duration::from_secs(30), Duration::from_secs(60)] {
            let mut app = app("abcdef");
            app.main.mode = Mode::Time;
            let now = Instant::now();
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                now,
            );
            app.update_at(
                key(Key::Character("b".into()), Some("b"), Modifiers::empty()),
                now + elapsed,
            );
            assert_eq!(app.main.session.entered(), &['a']);
            assert_ne!(app.page, Page::Typing);
            app.update_at(
                Message::Native(Event::Window(window::Event::Unfocused)),
                now + elapsed,
            );
            tick(&mut app, now + elapsed + FADE_DURATION);
            assert_eq!(app.page, Page::Score);
        }
    }

    #[test]
    fn score_actions_preserve_options_and_reset_all_session_state() {
        for action in [
            Message::Next,
            Message::Retry,
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
        ] {
            let mut app = app("same text,");
            app.main.mode = Mode::Time;
            app.main.options = words::Options {
                punctuation: true,
                numbers: true,
            };
            let now = Instant::now();
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                now,
            );
            tick(&mut app, now + Duration::from_secs(31));
            let revision = app.main.revision;
            let retry = matches!(action, Message::Retry);
            app.update_at(action, now + Duration::from_secs(32));
            assert_eq!(app.page, Page::Typing);
            assert_eq!(app.main.mode, Mode::Time);
            assert!(app.main.options.punctuation && app.main.options.numbers);
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(app.main.deadline, None);
            assert_eq!(app.main.seconds_remaining, 30);
            assert_eq!(app.main.revision, revision + 1);
            assert_eq!(app.main.generation, 1);
            if retry {
                assert_eq!(
                    app.main.session.target().iter().collect::<String>(),
                    "same text,"
                );
            } else {
                assert_eq!(app.main.session.word_count(), 500);
            }
        }
    }
    #[test]
    fn feature_toggles_are_independent_reset_progress_and_survive_restarts_and_modes() {
        for mode in [Mode::Words, Mode::Time] {
            let mut app = App::default();
            app.update(Message::SelectMode(mode));
            for (toggle, punctuation, numbers) in [
                (Message::TogglePunctuation, true, false),
                (Message::ToggleNumbers, true, true),
                (Message::TogglePunctuation, false, true),
                (Message::ToggleNumbers, false, false),
            ] {
                let now = Instant::now();
                app.update_at(
                    key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                    now,
                );
                tick(&mut app, now + Duration::from_secs(5));
                let revision = app.main.revision;
                app.update(toggle);
                let expected = words::Options {
                    punctuation,
                    numbers,
                };
                assert_eq!(app.main.options, expected);
                assert_eq!(app.main.mode, mode);
                assert_eq!(app.main.revision, revision + 1);
                assert_eq!(app.main.session.position(), 0);
                assert_eq!(app.main.deadline, None);
                assert_eq!(app.main.seconds_remaining, 30);
                assert_eq!(
                    app.main.monitor(),
                    if mode == Mode::Words { "0/50" } else { "30s" }
                );
                for action in [
                    key(Key::Named(Named::Tab), None, Modifiers::empty()),
                    Message::SelectMode(if mode == Mode::Words {
                        Mode::Time
                    } else {
                        Mode::Words
                    }),
                    Message::SelectMode(mode),
                ] {
                    app.update(action);
                    assert_eq!(app.main.options, expected);
                    assert_eq!(app.main.session.word_count(), 500);
                    let target = app.main.session.target();
                    assert_eq!(target.iter().any(|c| c.is_ascii_punctuation()), punctuation);
                    assert_eq!(target.iter().any(|c| c.is_ascii_digit()), numbers);
                }
            }
        }
    }

    #[test]
    fn punctuation_and_numbers_use_committed_text_and_word_boundaries() {
        let mut app = app("word, 42! next");
        app.update(key(
            Key::Character("word".into()),
            Some("word"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.monitor(), "0/50");
        app.update(key(
            Key::Character(",".into()),
            Some(","),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.monitor(), "1/50");
        app.update(key(
            Key::Character(" 42".into()),
            Some(" 42"),
            Modifiers::empty(),
        ));
        app.update(key(Key::Character("!".into()), Some("!"), Modifiers::SHIFT));
        assert_eq!(app.main.monitor(), "2/50");
        for index in 0..app.main.session.position() {
            assert_eq!(
                app.main.session.character_state(index),
                Some(typerr::engine::CharacterState::Correct)
            );
        }
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.monitor(), "1/50");
    }
}
