//! Other machines' Quarterdecks. Each machine serves its own data and tools over HTTP (the same tools
//! as the MCP server); the UI merges what peers serve, and edits to their records go back to them.
//! The one thing every machine shares is the areas list ([`areas`]), which says where schedules live
//! ([`handoff`]).

pub mod addresses;
pub mod areas;
pub mod client;
pub mod handoff;
pub mod health;
pub mod reach;
pub mod server;

use std::io::Read as _;

use crate::error::AppResult;

/// 1024 short, common English words: each one picked is 10 bits.
const WORDS: &str = include_str!("words.txt");
/// Five words, about 50 bits: plenty for a token that only works on this network.
const TOKEN_WORDS: usize = 5;

/// A random token for `[sync] token` that a person can read off one screen and type on another,
/// e.g. `maple-river-copper-lantern-swift`.
pub fn new_token() -> AppResult<String> {
    let words: Vec<&str> = WORDS.lines().filter(|w| !w.is_empty()).collect();
    let mut bytes = [0_u8; TOKEN_WORDS * 2];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    // The list has exactly 1024 words, so masking to 10 bits picks one without bias.
    let picked: Vec<&str> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|&[hi, lo]| {
            words
                .get(usize::from(u16::from_be_bytes([hi, lo]) & 0x03ff))
                .copied()
        })
        .collect();
    Ok(picked.join("-"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn tokens_are_five_listed_words_and_differ() {
        let words: Vec<&str> = super::WORDS.lines().collect();
        assert_eq!(words.len(), 1024);
        let (a, b) = (super::new_token().unwrap(), super::new_token().unwrap());
        assert_eq!(a.split('-').count(), 5);
        assert!(a.split('-').all(|w| words.contains(&w)));
        assert_ne!(a, b);
    }
}
