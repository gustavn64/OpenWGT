//! The network parts' own files, built into the port from
//! `crates/engine/opennet` (`tools/opennet`): the word list of the random
//! channel names.

use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/opennet_files.rs"));

/// The random channel names' words, in the pack.
pub const WORDS_FILE: &str = "words.txt";

/// The words random channel names are made of (`tools/opennet/src/words.txt`):
/// 2048 short English words, the same in every language, sorted.
#[must_use]
pub fn words() -> &'static [&'static str] {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        let text = BUILTIN
            .iter()
            .find(|(path, _)| *path == WORDS_FILE)
            .and_then(|(_, b)| std::str::from_utf8(b).ok())
            .unwrap_or_default();
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    })
}
