//! The relay's messages (`relay/PROTOCOL.md`, version 1): one flat JSON
//! object per WebSocket text frame, at most [`MAX_BYTES`]. Written and read
//! here by hand, with a reader just big enough for them.

use super::{NetValue, Peer};

/// The protocol's version, said in the hello.
pub const VERSION: u32 = 1;
/// The longest message either side sends.
pub const MAX_BYTES: usize = 256;

/// A part's side of a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Send,
    Recv,
}

/// What a part tells the relay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientMsg {
    /// The first message: the part's side, and whether its widget runs.
    Hello { role: Role, run: bool },
    /// The widget started (GO) or stopped.
    Run(bool),
    /// A Sender's value.
    Val(NetValue),
    /// Are you there? (every 20 s; the relay answers [`RelayMsg::Pong`]).
    Ping,
}

impl ClientMsg {
    /// The message as sent; `None` for a number that isn't finite, which
    /// JSON can't carry.
    #[must_use]
    pub fn encode(self) -> Option<String> {
        Some(match self {
            ClientMsg::Hello { role, run } => {
                let role = match role {
                    Role::Send => "send",
                    Role::Recv => "recv",
                };
                format!(r#"{{"t":"hello","role":"{role}","run":{run},"v":{VERSION}}}"#)
            }
            ClientMsg::Run(run) => format!(r#"{{"t":"run","run":{run}}}"#),
            ClientMsg::Val(NetValue::Bool(b)) => format!(r#"{{"t":"val","k":"b","v":{b}}}"#),
            ClientMsg::Val(NetValue::Number(x)) => {
                if !x.is_finite() {
                    return None;
                }
                // The shortest digits that read back as `x`, with an
                // exponent where it's shorter (`1e300`): valid JSON.
                format!(r#"{{"t":"val","k":"n","v":{x:?}}}"#)
            }
            ClientMsg::Val(NetValue::Picture(n)) => format!(r#"{{"t":"val","k":"p","v":{n}}}"#),
            // Exactly so: the relay's runtime answers it without waking it.
            ClientMsg::Ping => r#"{"t":"ping"}"#.to_owned(),
        })
    }
}

/// What the relay tells a part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelayMsg {
    /// The other side's state.
    Peer(Peer),
    /// To a Sender: whether it waits while another holds the channel.
    Busy(bool),
    /// To a Receiver: the Sender's value.
    Val(NetValue),
    Pong,
}

/// A message from the relay; `None` for anything else (too long, not
/// JSON, an unknown kind, a value of the wrong type), which the part
/// ignores. Keys it doesn't know are skipped.
#[must_use]
pub fn parse(text: &str) -> Option<RelayMsg> {
    if text.len() > MAX_BYTES {
        return None;
    }
    let fields = Reader::new(text).object()?;
    let get = |key: &str| fields.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v);
    let Some(Json::Str(t)) = get("t") else {
        return None;
    };
    match t.as_str() {
        "peer" => match get("s")? {
            Json::Str(s) if s == "none" => Some(RelayMsg::Peer(Peer::None)),
            Json::Str(s) if s == "open" => Some(RelayMsg::Peer(Peer::Open)),
            Json::Str(s) if s == "run" => Some(RelayMsg::Peer(Peer::Run)),
            _ => None,
        },
        "busy" => match get("busy")? {
            Json::Bool(b) => Some(RelayMsg::Busy(*b)),
            _ => None,
        },
        "val" => {
            let v = get("v")?;
            let value = match (get("k")?, v) {
                (Json::Str(k), Json::Bool(b)) if k == "b" => NetValue::Bool(*b),
                (Json::Str(k), Json::Num(x)) if k == "n" && x.is_finite() => NetValue::Number(*x),
                (Json::Str(k), Json::Num(x)) if k == "p" => NetValue::Picture(picture(*x)?),
                _ => return None,
            };
            Some(RelayMsg::Val(value))
        }
        "pong" => Some(RelayMsg::Pong),
        _ => None,
    }
}

/// A picture's number: a whole number that fits a `u32`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn picture(x: f64) -> Option<u32> {
    (x.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&x)).then_some(x as u32)
}

/// A JSON value, as far as the messages need: nested ones are skipped.
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    /// An object or an array.
    Nested,
}

/// How deep nested values may go before the message is refused.
const MAX_DEPTH: usize = 8;

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Reader<'a> {
        Reader {
            b: text.as_bytes(),
            at: 0,
        }
    }

    /// The whole text as one object: its keys and values in order.
    fn object(mut self) -> Option<Vec<(String, Json)>> {
        self.space();
        let fields = self.members(0)?;
        self.space();
        (self.at == self.b.len()).then_some(fields)
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.at).copied()
    }

    fn eat(&mut self, c: u8) -> Option<()> {
        (self.peek()? == c).then(|| self.at += 1)
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// `{ "key": value, ... }`.
    fn members(&mut self, depth: usize) -> Option<Vec<(String, Json)>> {
        self.eat(b'{')?;
        let mut out = Vec::new();
        self.space();
        if self.eat(b'}').is_some() {
            return Some(out);
        }
        loop {
            self.space();
            let key = self.string()?;
            self.space();
            self.eat(b':')?;
            let value = self.value(depth)?;
            out.push((key, value));
            self.space();
            match self.peek()? {
                b',' => self.at += 1,
                b'}' => {
                    self.at += 1;
                    return Some(out);
                }
                _ => return None,
            }
        }
    }

    /// `[ value, ... ]`, read and dropped.
    fn elements(&mut self, depth: usize) -> Option<()> {
        self.eat(b'[')?;
        self.space();
        if self.eat(b']').is_some() {
            return Some(());
        }
        loop {
            self.value(depth)?;
            self.space();
            match self.peek()? {
                b',' => self.at += 1,
                b']' => {
                    self.at += 1;
                    return Some(());
                }
                _ => return None,
            }
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        self.space();
        match self.peek()? {
            b'"' => self.string().map(Json::Str),
            b'{' | b'[' if depth + 1 >= MAX_DEPTH => None,
            b'{' => self.members(depth + 1).map(|_| Json::Nested),
            b'[' => self.elements(depth + 1).map(|()| Json::Nested),
            b't' => self.word("true").map(|()| Json::Bool(true)),
            b'f' => self.word("false").map(|()| Json::Bool(false)),
            b'n' => self.word("null").map(|()| Json::Null),
            _ => self.number().map(Json::Num),
        }
    }

    fn word(&mut self, w: &str) -> Option<()> {
        let end = self.at + w.len();
        (self.b.get(self.at..end)? == w.as_bytes()).then(|| self.at = end)
    }

    /// `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
    fn number(&mut self) -> Option<f64> {
        let start = self.at;
        let _ = self.eat(b'-');
        let digits = |r: &mut Reader<'_>| {
            let from = r.at;
            while r.peek().is_some_and(|c| c.is_ascii_digit()) {
                r.at += 1;
            }
            r.at - from
        };
        match self.peek()? {
            b'0' => self.at += 1,
            b'1'..=b'9' => {
                digits(self);
            }
            _ => return None,
        }
        if self.eat(b'.').is_some() && digits(self) == 0 {
            return None;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if digits(self) == 0 {
                return None;
            }
        }
        std::str::from_utf8(&self.b[start..self.at]).ok()?.parse().ok()
    }

    /// A string with its escapes undone.
    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let rest = std::str::from_utf8(self.b.get(self.at..)?).ok()?;
            let c = rest.chars().next()?;
            self.at += c.len_utf8();
            match c {
                '"' => return Some(out),
                '\\' => {
                    let e = self.peek()?;
                    self.at += 1;
                    out.push(match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hex = std::str::from_utf8(self.b.get(self.at..self.at + 4)?).ok()?;
                            self.at += 4;
                            // A lone half of a surrogate pair stands for
                            // nothing the messages use.
                            char::from_u32(u32::from_str_radix(hex, 16).ok()?).unwrap_or('\u{fffd}')
                        }
                        _ => return None,
                    });
                }
                c if u32::from(c) < 0x20 => return None,
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_each_message() {
        let e = |m: ClientMsg| m.encode().unwrap();
        assert_eq!(
            e(ClientMsg::Hello {
                role: Role::Send,
                run: false
            }),
            r#"{"t":"hello","role":"send","run":false,"v":1}"#
        );
        assert_eq!(
            e(ClientMsg::Hello {
                role: Role::Recv,
                run: true
            }),
            r#"{"t":"hello","role":"recv","run":true,"v":1}"#
        );
        assert_eq!(e(ClientMsg::Run(true)), r#"{"t":"run","run":true}"#);
        assert_eq!(e(ClientMsg::Run(false)), r#"{"t":"run","run":false}"#);
        assert_eq!(
            e(ClientMsg::Val(NetValue::Bool(true))),
            r#"{"t":"val","k":"b","v":true}"#
        );
        assert_eq!(
            e(ClientMsg::Val(NetValue::Number(42.0))),
            r#"{"t":"val","k":"n","v":42.0}"#
        );
        assert_eq!(
            e(ClientMsg::Val(NetValue::Number(-0.25))),
            r#"{"t":"val","k":"n","v":-0.25}"#
        );
        assert_eq!(
            e(ClientMsg::Val(NetValue::Number(1e300))),
            r#"{"t":"val","k":"n","v":1e300}"#
        );
        assert_eq!(
            e(ClientMsg::Val(NetValue::Number(-1.5e-7))),
            r#"{"t":"val","k":"n","v":-1.5e-7}"#
        );
        assert_eq!(
            e(ClientMsg::Val(NetValue::Picture(1050))),
            r#"{"t":"val","k":"p","v":1050}"#
        );
        assert_eq!(e(ClientMsg::Ping), r#"{"t":"ping"}"#);
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(ClientMsg::Val(NetValue::Number(x)).encode(), None);
        }
    }

    #[test]
    fn numbers_survive_the_trip() {
        for x in [
            0.0,
            -0.0,
            1.0 / 3.0,
            1e-300,
            f64::MAX,
            f64::MIN_POSITIVE,
            123_456_789.125,
        ] {
            let text = ClientMsg::Val(NetValue::Number(x)).encode().unwrap();
            assert!(text.len() <= MAX_BYTES);
            // A relay's echo is the same object.
            let back = parse(&text).unwrap();
            assert_eq!(back, RelayMsg::Val(NetValue::Number(x)), "{text}");
        }
    }

    #[test]
    fn reads_each_message() {
        assert_eq!(parse(r#"{"t":"peer","s":"none"}"#), Some(RelayMsg::Peer(Peer::None)));
        assert_eq!(parse(r#"{"t":"peer","s":"open"}"#), Some(RelayMsg::Peer(Peer::Open)));
        assert_eq!(parse(r#"{"t":"peer","s":"run"}"#), Some(RelayMsg::Peer(Peer::Run)));
        assert_eq!(parse(r#"{"t":"busy","busy":true}"#), Some(RelayMsg::Busy(true)));
        assert_eq!(parse(r#"{"t":"busy","busy":false}"#), Some(RelayMsg::Busy(false)));
        assert_eq!(
            parse(r#"{"t":"val","k":"b","v":false}"#),
            Some(RelayMsg::Val(NetValue::Bool(false)))
        );
        assert_eq!(
            parse(r#"{"t":"val","k":"n","v":-12.5e1}"#),
            Some(RelayMsg::Val(NetValue::Number(-125.0)))
        );
        assert_eq!(
            parse(r#"{"t":"val","k":"p","v":4294967295}"#),
            Some(RelayMsg::Val(NetValue::Picture(u32::MAX)))
        );
        assert_eq!(parse(r#"{"t":"pong"}"#), Some(RelayMsg::Pong));
        // Spaces, keys in any order, escapes.
        assert_eq!(
            parse(" {\n \"s\" : \"r\\u0075n\" , \"t\":\"peer\" } "),
            Some(RelayMsg::Peer(Peer::Run))
        );
    }

    #[test]
    fn unknown_keys_are_skipped() {
        assert_eq!(
            parse(r#"{"t":"peer","s":"open","x":[1,{"y":null}],"z":"\"","w":null}"#),
            Some(RelayMsg::Peer(Peer::Open))
        );
        assert_eq!(
            parse(r#"{"t":"busy","busy":true,"since":12}"#),
            Some(RelayMsg::Busy(true))
        );
    }

    #[test]
    fn bad_input_is_ignored() {
        for bad in [
            "",
            "null",
            "[]",
            "{}",
            r#"{"t":"peer"}"#,
            r#"{"t":"peer","s":"gone"}"#,
            r#"{"t":"peer","s":1}"#,
            r#"{"t":"busy","busy":"yes"}"#,
            r#"{"t":"busy","busy":1}"#,
            r#"{"t":"hello"}"#,
            r#"{"t":"nope"}"#,
            r#"{"t":"val","k":"n","v":"1"}"#,
            r#"{"t":"val","k":"n","v":1e999}"#,
            r#"{"t":"val","k":"n","v":true}"#,
            r#"{"t":"val","k":"b","v":0}"#,
            r#"{"t":"val","k":"p","v":1.5}"#,
            r#"{"t":"val","k":"p","v":-1}"#,
            r#"{"t":"val","k":"p","v":4294967296}"#,
            r#"{"t":"val","k":"x","v":1}"#,
            r#"{"t":"val","v":1}"#,
            r#"{"t":"pong"} x"#,
            r#"{"t":"pong",}"#,
            r#"{"t":"pong""#,
            r"{'t':'pong'}",
            r#"{"t":"val","k":"n","v":01}"#,
            r#"{"t":"val","k":"n","v":1.}"#,
            r#"{"t":"val","k":"n","v":.5}"#,
            r#"{"t":"val","k":"n","v":+1}"#,
            r#"{"t":"val","k":"n","v":NaN}"#,
            "{\"t\":\"po\nng\"}",
            r#"{"t":"pong","x":"\q"}"#,
            r#"{"t":"pong","x":[[[[[[[[[[1]]]]]]]]]]}"#,
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
        let long = format!(r#"{{"t":"pong","pad":"{}"}}"#, "x".repeat(MAX_BYTES));
        assert_eq!(parse(&long), None);
    }
}
