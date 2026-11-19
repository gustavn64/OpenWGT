//! Windows-1252, the game's text as the resources, the saved widgets, the
//! puzzle file and the help hold it.

/// The characters 0x80..=0x9f of Windows-1252 (undefined ones as `?`).
const HIGH: [char; 32] = [
    '€', '?', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '?', 'Ž', '?', '?', '‘', '’', '“', '”', '•', '–',
    '—', '˜', '™', 'š', '›', 'œ', '?', 'ž', 'Ÿ',
];

/// The character of the byte `b`.
#[must_use]
pub fn char_of(b: u8) -> char {
    match b {
        0x80..=0x9f => HIGH[usize::from(b - 0x80)],
        _ => char::from(b),
    }
}

/// The text of `b`.
#[must_use]
pub fn decode(b: &[u8]) -> String {
    b.iter().map(|&c| char_of(c)).collect()
}

/// `s` as Windows-1252 bytes; what it can't hold becomes `?`.
#[must_use]
pub fn encode(s: &str) -> Vec<u8> {
    s.chars()
        .map(|ch| match u32::from(ch) {
            c @ (0..0x80 | 0xa0..0x100) => u8::try_from(c).unwrap_or(b'?'),
            _ => HIGH
                .iter()
                .position(|&h| h == ch && h != '?')
                .and_then(|i| u8::try_from(0x80 + i).ok())
                .unwrap_or(b'?'),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let b = b"\xb0Celsius \x93to\x94 \x80 c\x9cur";
        assert_eq!(decode(b), "°Celsius “to” € cœur");
        assert_eq!(encode(&decode(b)), b);
        assert_eq!(encode("« é » – …"), b"\xab \xe9 \xbb \x96 \x85");
        assert_eq!(encode("Œ€ア"), b"\x8c\x80?");
    }
}
