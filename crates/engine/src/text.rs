//! The game's text is Windows-1252, as the resources and saved widgets
//! hold it (the Mac's text was converted for the Windows build, the saved
//! widgets' too as far as their labels go: both versions share them).

pub use formats::cp1252::{decode, encode};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let b = b"\xb0Celsius \x93to\x94 \x80";
        assert_eq!(decode(b), "°Celsius “to” €");
        assert_eq!(encode(&decode(b)), b);
    }
}
