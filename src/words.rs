//! Bundled word generation. The caller owns randomness for deterministic tests.
use rand::RngExt;

pub const WORD_COUNT: usize = 50;
pub const WORD_LIST: &str = include_str!("../assets/words/english.txt");

pub fn generate(rng: &mut impl rand::Rng) -> String {
    let words: Vec<_> = WORD_LIST.split_whitespace().collect();
    (0..WORD_COUNT)
        .map(|_| words[rng.random_range(0..words.len())])
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn bundled_words_are_unique_lowercase_ascii() {
        let words: Vec<_> = WORD_LIST.split_whitespace().collect();
        assert_eq!(words.len(), 200);
        assert!(
            words
                .iter()
                .all(|word| word.bytes().all(|c| c.is_ascii_lowercase()))
        );
        let unique: std::collections::HashSet<_> = words.iter().collect();
        assert_eq!(unique.len(), words.len());
    }

    #[test]
    fn fifty_words_and_seeded_generation_are_reproducible() {
        let a = generate(&mut StdRng::seed_from_u64(17));
        let b = generate(&mut StdRng::seed_from_u64(17));
        assert_eq!(a, b);
        assert_eq!(a.split_whitespace().count(), 50);
        assert_eq!(a.bytes().filter(|b| *b == b' ').count(), 49);
        assert!(!a.starts_with(' ') && !a.ends_with(' '));
        assert!(
            a.split(' ')
                .all(|w| WORD_LIST.lines().any(|source| source == w))
        );
    }
}
