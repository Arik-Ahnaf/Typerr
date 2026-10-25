//! Bundled word generation. The caller owns randomness for deterministic tests.
use rand::RngExt;

pub const WORD_COUNT: usize = 500;
pub const WORD_LIST: &str = include_str!("../assets/words/english.txt");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub punctuation: bool,
    pub numbers: bool,
}

pub fn generate(rng: &mut impl rand::Rng, options: Options) -> String {
    let words: Vec<_> = WORD_LIST.split_whitespace().collect();
    let punctuation = [',', '.', '!', '?', ';', ':'];
    let mut number_slot = 0;
    let mut punctuation_slot = 0;
    (0..WORD_COUNT)
        .map(|index| {
            // Random positions in small groups keep enabled features present
            // throughout the buffer, including the first visible lines.
            if options.numbers && index % 10 == 0 {
                number_slot = rng.random_range(0..10);
            }
            if options.punctuation && index % 4 == 0 {
                punctuation_slot = rng.random_range(0..4);
            }
            let mut token = if options.numbers && index % 10 == number_slot {
                rng.random_range(0..=9999).to_string()
            } else {
                words[rng.random_range(0..words.len())].to_owned()
            };
            if options.punctuation && index % 4 == punctuation_slot {
                token.push(punctuation[rng.random_range(0..punctuation.len())]);
            }
            token
        })
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
    fn five_hundred_words_and_seeded_generation_are_reproducible() {
        let a = generate(&mut StdRng::seed_from_u64(17), Options::default());
        let b = generate(&mut StdRng::seed_from_u64(17), Options::default());
        assert_eq!(a, b);
        assert_eq!(a.split_whitespace().count(), 500);
        assert_eq!(a.bytes().filter(|b| *b == b' ').count(), 499);
        assert!(!a.starts_with(' ') && !a.ends_with(' '));
        assert!(
            a.split(' ')
                .all(|w| WORD_LIST.lines().any(|source| source == w))
        );
    }
    #[test]
    fn all_option_combinations_preserve_tokens_and_seeded_randomness() {
        for punctuation in [false, true] {
            for numbers in [false, true] {
                let options = Options {
                    punctuation,
                    numbers,
                };
                for seed in 0..16 {
                    let generated = generate(&mut StdRng::seed_from_u64(seed), options);
                    assert_eq!(
                        generated,
                        generate(&mut StdRng::seed_from_u64(seed), options)
                    );
                    let tokens: Vec<_> = generated.split(' ').collect();
                    assert_eq!(tokens.len(), 500);
                    assert!(tokens.iter().all(|token| !token.is_empty()));
                    assert_eq!(generated.bytes().filter(|b| *b == b' ').count(), 499);
                    assert!(generated.is_ascii());
                    assert_eq!(
                        generated.chars().any(|c| c.is_ascii_punctuation()),
                        punctuation
                    );
                    assert_eq!(generated.chars().any(|c| c.is_ascii_digit()), numbers);
                    assert!(tokens.iter().all(|token| {
                        let core = token.trim_end_matches(|c: char| c.is_ascii_punctuation());
                        WORD_LIST.lines().any(|word| word == core)
                            || (numbers && core.parse::<u16>().is_ok_and(|number| number <= 9999))
                    }));
                    // Enabled features appear early, not just somewhere far below the viewport.
                    assert_eq!(
                        tokens[..10]
                            .iter()
                            .any(|token| token.chars().any(|c| c.is_ascii_digit())),
                        numbers
                    );
                    assert_eq!(
                        tokens[..4]
                            .iter()
                            .any(|token| token.chars().any(|c| c.is_ascii_punctuation())),
                        punctuation
                    );
                }
            }
        }
    }
}
