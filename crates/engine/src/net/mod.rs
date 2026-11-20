//! The network parts: a Sender sends what is wired into it (on/off, a
//! number or a picture) to the Receivers with the same channel name in
//! other players' widgets, through a relay (`relay/`, its `PROTOCOL.md`).
//! The port's own: the original has nothing like them.
//!
//! The engine stays free of input and output here as elsewhere: the parts
//! keep what they are to send and what came in, [`proto`] reads and writes
//! the relay's messages, and the host does the talking. Their pictures,
//! dialogs and texts are the port's own too ([`pack`]), there whether the
//! game's data is a CD's or OpenWDATA.

pub mod pack;
pub mod parts;
pub mod proto;
pub mod wgt;

/// A value on the wire: what a Sender sends and a Receiver gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetValue {
    /// On or off.
    Bool(bool),
    /// A number (finite).
    Number(f64),
    /// A picture, by the sheet it's frame 1 of; 0 is no picture.
    Picture(u32),
}

/// The other side's state, as the relay tells it (`peer`): a Receiver's
/// Sender, or a Sender's best Receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Peer {
    /// Nobody there.
    #[default]
    None,
    /// There, its widget stopped.
    Open,
    /// There, its widget running.
    Run,
}

/// A part's link, as its lamp and its display show it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Status {
    /// Connected to the relay (not connecting or waiting to try again).
    pub online: bool,
    /// The other side.
    pub peer: Peer,
    /// A Sender waiting while another holds the channel.
    pub busy: bool,
}

impl Status {
    /// The lamp: the frame of the part's sheet to draw. 1 grey (not
    /// connected), 2 red (nobody there), 3 amber (there, stopped), 4 green
    /// (there, running).
    #[must_use]
    pub fn lamp(self) -> u16 {
        match (self.online, self.peer) {
            (false, _) => 1,
            (true, Peer::None) => 2,
            (true, Peer::Open) => 3,
            (true, Peer::Run) => 4,
        }
    }
}

/// The longest channel name.
pub const CHANNEL_MAX: usize = 20;

/// A channel name as typed made one the relay takes: lowercase letters,
/// digits and single hyphens, at most [`CHANNEL_MAX`]. Spaces and
/// underscores become hyphens, anything else goes, and hyphens don't
/// start, end or double, so `Cat Sun_Oak!` is `cat-sun-oak`. Empty if
/// nothing is left.
#[must_use]
pub fn clean_channel(typed: &str) -> String {
    let mut out = String::new();
    for c in typed.chars() {
        let c = match c {
            'A'..='Z' => c.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => c,
            ' ' | '_' | '-' => '-',
            _ => continue,
        };
        if c == '-' && (out.is_empty() || out.ends_with('-')) {
            continue;
        }
        out.push(c);
    }
    out.truncate(CHANNEL_MAX);
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// A new random channel name: three words of [`pack::words`], joined
/// with hyphens (`cat-sun-oak`), one of 2048³. The randomness is the
/// system's (the browser's `crypto.getRandomValues`), so that two players
/// don't come upon the same name.
#[must_use]
pub fn random_channel() -> String {
    let words = pack::words();
    let Ok(n) = u64::try_from(words.len()) else {
        return String::new();
    };
    if n == 0 {
        return String::new();
    }
    let mut r = random_u64();
    let mut pick = || {
        let i = usize::try_from(r % n).unwrap_or(0);
        r /= n;
        words[i]
    };
    let (a, b, c) = (pick(), pick(), pick());
    format!("{a}-{b}-{c}")
}

/// 64 random bits from the system; should it have none to give, from a
/// generator of its own (names then repeat between runs, not within one).
fn random_u64() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static FALLBACK: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);
    getrandom::u64().unwrap_or_else(|_| {
        // SplitMix64.
        let mut z = FALLBACK
            .fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed)
            .wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_cleaned_as_the_relay_wants_them() {
        for (typed, clean) in [
            ("Kiwi Sol 47!", "kiwi-sol-47"),
            ("Katt Sol 47", "katt-sol-47"),
            ("katt-sol-47", "katt-sol-47"),
            ("cat_sun__oak", "cat-sun-oak"),
            ("  -cat - sun-  ", "cat-sun"),
            ("Kiwi-42!", "kiwi-42"),
            ("räksmörgås", "rksmrgs"),
            ("abcdefghij-klmnopqrstuvwxyz", "abcdefghij-klmnopqrs"),
            // Cut at a hyphen: it goes too.
            ("abcdefghijklmnopqrs-t", "abcdefghijklmnopqrs"),
            ("!!!", ""),
            ("", ""),
        ] {
            assert_eq!(clean_channel(typed), clean, "{typed:?}");
        }
        for c in ["", "a", "cat-sun-oak", "kiwi42"] {
            assert_eq!(clean_channel(c), c);
        }
    }

    #[test]
    fn random_channels_are_three_words() {
        let words = pack::words();
        assert_eq!(words.len(), 2048);
        assert!(words.windows(2).all(|w| w[0] < w[1]), "sorted, each once");
        assert!(
            words
                .iter()
                .all(|w| (3..=5).contains(&w.len()) && w.bytes().all(|c| c.is_ascii_lowercase()))
        );
        let names: Vec<String> = (0..50).map(|_| random_channel()).collect();
        for name in &names {
            let parts: Vec<&str> = name.split('-').collect();
            assert_eq!(parts.len(), 3, "{name}");
            assert!(parts.iter().all(|p| words.binary_search(p).is_ok()), "{name}");
            assert_eq!(&clean_channel(name), name);
            assert!(name.len() <= CHANNEL_MAX);
        }
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert!(unique.len() > 45, "{names:?}");
    }

    #[test]
    fn the_lamp_follows_the_link() {
        let s = |online, peer| Status {
            online,
            peer,
            busy: false,
        };
        assert_eq!(Status::default().lamp(), 1);
        assert_eq!(s(false, Peer::Run).lamp(), 1);
        assert_eq!(s(true, Peer::None).lamp(), 2);
        assert_eq!(s(true, Peer::Open).lamp(), 3);
        assert_eq!(s(true, Peer::Run).lamp(), 4);
    }
}
