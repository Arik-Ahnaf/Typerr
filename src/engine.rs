//! Character-based typing rules, independent of the graphical frontend.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterState {
    Pending,
    Correct,
    Incorrect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Complete,
}

#[derive(Debug, Clone)]
pub struct TypingSession {
    target: Vec<char>,
    entered: Vec<char>,
}

impl TypingSession {
    pub fn new(target: impl AsRef<str>) -> Self {
        Self {
            target: target.as_ref().chars().collect(),
            entered: Vec::new(),
        }
    }

    pub fn target(&self) -> &[char] {
        &self.target
    }
    pub fn entered(&self) -> &[char] {
        &self.entered
    }
    pub fn position(&self) -> usize {
        self.entered.len()
    }

    pub fn status(&self) -> SessionStatus {
        if self.position() == self.target.len() {
            SessionStatus::Complete
        } else {
            SessionStatus::Active
        }
    }

    pub fn character_state(&self, index: usize) -> Option<CharacterState> {
        self.target
            .get(index)
            .map(|expected| match self.entered.get(index) {
                None => CharacterState::Pending,
                Some(actual) if actual == expected => CharacterState::Correct,
                Some(_) => CharacterState::Incorrect,
            })
    }

    /// Returns whether an input was accepted. Completion is terminal.
    pub fn input(&mut self, character: char) -> bool {
        if character.is_control() || self.status() == SessionStatus::Complete {
            return false;
        }
        self.entered.push(character);
        true
    }

    pub fn backspace(&mut self) -> bool {
        self.status() == SessionStatus::Active && self.entered.pop().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_wrong_and_unicode_inputs_use_character_positions() {
        let mut s = TypingSession::new("abcde");
        for c in ['a', 'X', 'é', ' '] {
            assert!(s.input(c));
        }
        assert_eq!(s.position(), 4);
        assert_eq!(s.entered(), &['a', 'X', 'é', ' ']);
        assert_eq!(s.character_state(0), Some(CharacterState::Correct));
        for i in 1..4 {
            assert_eq!(s.character_state(i), Some(CharacterState::Incorrect));
        }
        assert_eq!(s.character_state(4), Some(CharacterState::Pending));
        assert_eq!(s.character_state(5), None);
    }

    #[test]
    fn spaces_advance_even_when_wrong_and_backspace_crosses_them() {
        let mut s = TypingSession::new("a bc");
        assert!(!s.backspace());
        for c in ['a', 'x', 'b'] {
            s.input(c);
        }
        assert_eq!(s.character_state(1), Some(CharacterState::Incorrect));
        assert!(s.backspace());
        assert!(s.backspace());
        assert_eq!(s.position(), 1);
        s.input(' ');
        assert_eq!(s.character_state(1), Some(CharacterState::Correct));
        assert_eq!(s.character_state(2), Some(CharacterState::Pending));
    }

    #[test]
    fn completion_locks_correct_and_incorrect_final_input() {
        for last in ['b', 'x'] {
            let mut s = TypingSession::new("ab");
            s.input('a');
            s.input(last);
            assert_eq!(s.status(), SessionStatus::Complete);
            assert!(!s.input('c'));
            assert!(!s.backspace());
            assert_eq!(s.entered(), &['a', last]);
        }
    }

    #[test]
    fn controls_and_empty_target_do_not_advance() {
        let mut s = TypingSession::new("a");
        for c in ['\n', '\t', '\r', '\u{7f}'] {
            assert!(!s.input(c));
        }
        assert_eq!(s.position(), 0);
        let mut empty = TypingSession::new("");
        assert_eq!(empty.status(), SessionStatus::Complete);
        assert!(!empty.input('a'));
        assert!(!empty.backspace());
    }
}
