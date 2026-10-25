use crate::ui::typing_panel::TypingPanel;
use iced::{
    Element, Event, keyboard,
    widget::{container, stack},
    window,
};
use typerr::{engine::TypingSession, words};

pub struct App {
    main: MainTyping,
    focused: bool,
}

struct MainTyping {
    session: TypingSession,
    /// Changes only when an edit succeeds, restoring the viewport after inspection.
    revision: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    Native(Event),
    OpenConnection,
}

impl Default for App {
    fn default() -> Self {
        Self {
            main: MainTyping {
                session: TypingSession::new(words::generate(&mut rand::rng())),
                revision: 0,
            },
            focused: true,
        }
    }
}

impl App {
    pub fn update(&mut self, message: Message) {
        match message {
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
                        self.main.session = TypingSession::new(words::generate(&mut rand::rng()));
                        self.main.revision = self.main.revision.wrapping_add(1);
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
        let typing = container(TypingPanel::new(
            &self.main.session,
            self.main.revision,
            Message::Native,
        ))
        .padding(iced::Padding {
            top: 125.0,
            bottom: 125.0,
            left: 30.0,
            right: 30.0,
        })
        .width(iced::Fill)
        .height(iced::Fill);
        stack![
            typing,
            crate::ui::chrome::header(),
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
                49
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
}
