use crate::ui::typing_panel::TypingPanel;
use iced::{
    Element, Event, keyboard,
    widget::{container, stack},
    window,
};
use std::time::{Duration, Instant};
use typerr::{engine::TypingSession, words};

const TIME_LIMIT_SECONDS: u64 = 30;
const WORD_TARGET: usize = 50;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Words,
    Time,
}

pub struct App {
    main: MainTyping,
    focused: bool,
}

struct MainTyping {
    session: TypingSession,
    mode: Mode,
    options: words::Options,
    deadline: Option<Instant>,
    seconds_remaining: u64,
    /// Changes on accepted edits or explicit session resets, restoring the viewport.
    revision: u64,
}

impl MainTyping {
    fn restart(&mut self) {
        self.session = TypingSession::new(words::generate(&mut rand::rng(), self.options));
        self.deadline = None;
        self.seconds_remaining = TIME_LIMIT_SECONDS;
        self.revision = self.revision.wrapping_add(1);
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
                self.session.completed_words().min(WORD_TARGET),
                WORD_TARGET
            ),
            Mode::Time => format!("{}s", self.seconds_remaining),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Native(Event),
    OpenConnection,
    SelectMode(Mode),
    TogglePunctuation,
    ToggleNumbers,
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
                deadline: None,
                seconds_remaining: TIME_LIMIT_SECONDS,
                revision: 0,
            },
            focused: true,
        }
    }
}

impl App {
    pub fn update(&mut self, message: Message) {
        self.update_at(message, Instant::now());
    }

    fn update_at(&mut self, message: Message, now: Instant) {
        match message {
            Message::TogglePunctuation => {
                self.main.options.punctuation = !self.main.options.punctuation;
                self.main.restart();
            }
            Message::ToggleNumbers => {
                self.main.options.numbers = !self.main.options.numbers;
                self.main.restart();
            }
            Message::SelectMode(mode) => {
                if self.main.mode != mode {
                    self.main.mode = mode;
                    self.main.restart();
                }
            }
            Message::Native(Event::Window(window::Event::RedrawRequested(now))) => {
                self.main.tick(now);
            }
            Message::OpenConnection => open_connection(),
            Message::Native(Event::Window(window::Event::Focused)) => self.focused = true,
            Message::Native(Event::Window(window::Event::Unfocused)) => self.focused = false,
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
                    }
                    return;
                }
                let changed = if key == keyboard::Key::Named(keyboard::key::Named::Backspace) {
                    self.main.session.backspace()
                } else {
                    let mut changed = false;
                    if let Some(text) = text {
                        for c in text.chars() {
                            changed |= self.main.session.input(c);
                        }
                    }
                    if changed && self.main.mode == Mode::Time && self.main.deadline.is_none() {
                        self.main.deadline = Some(now + Duration::from_secs(TIME_LIMIT_SECONDS));
                    }
                    changed
                };
                if changed {
                    self.main.revision = self.main.revision.wrapping_add(1);
                }
            }
            _ => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let panel = container(TypingPanel::new(
            &self.main.session,
            self.main.revision,
            self.main.monitor(),
            self.main.next_tick(),
            Message::Native,
        ))
        .max_width(crate::ui::theme::TYPING_MAX_WIDTH)
        .max_height(crate::ui::theme::TYPING_MAX_HEIGHT);
        let typing = container(panel)
            .padding(iced::Padding {
                top: 125.0,
                bottom: 125.0,
                left: 30.0,
                right: 30.0,
            })
            .width(iced::Fill)
            .align_x(iced::Center)
            .align_y(iced::Center)
            .height(iced::Fill);
        stack![
            typing,
            crate::ui::chrome::header(self.main.mode, self.main.options),
            container(crate::ui::chrome::footer())
                .height(iced::Fill)
                .align_y(iced::Bottom),
        ]
        .into()
    }
}

fn open_connection() {
    // Hand off to the OS browser off the input thread and reap the launcher.
    std::thread::spawn(|| {
        #[cfg(target_os = "windows")]
        let result = {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("cmd")
                .args(["/C", "start", "", "https://google.com"])
                .creation_flags(0x0800_0000)
                .status()
        };
        #[cfg(not(target_os = "windows"))]
        let result = std::process::Command::new("xdg-open")
            .arg("https://google.com")
            .status();
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
                deadline: None,
                seconds_remaining: TIME_LIMIT_SECONDS,
                revision: 0,
            },
            focused: true,
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
    fn timer_expiry_preserves_session_and_allows_typing() {
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
        assert_eq!(app.main.session.entered(), &['a', 'b']);
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
    fn word_target_caps_counter_without_finishing_the_buffer() {
        let mut app = app(&vec!["ab"; 500].join(" "));
        assert_eq!(app.main.monitor(), "0/50");
        assert_eq!(app.main.session.word_count(), 500);
        for completed in 1..=51 {
            // Incorrect input still completes a target word.
            app.update(key(
                Key::Character("xx".into()),
                Some("xx"),
                Modifiers::empty(),
            ));
            assert_eq!(app.main.session.completed_words(), completed);
            assert_eq!(app.main.monitor(), format!("{}/50", completed.min(50)));
            assert_eq!(
                app.main.session.status(),
                typerr::engine::SessionStatus::Active
            );
            app.update(key(
                Key::Character(" ".into()),
                Some(" "),
                Modifiers::empty(),
            ));
        }
        // Editing above the target stays capped; deleting the 50th word's last
        // letter brings the displayed progress below the target again.
        for _ in 0..4 {
            app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
            assert_eq!(app.main.monitor(), "50/50");
        }
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.monitor(), "49/50");
        app.update(key(Key::Named(Named::Tab), None, Modifiers::empty()));
        assert_eq!(app.main.monitor(), "0/50");
        assert_eq!(app.main.session.word_count(), 500);
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
