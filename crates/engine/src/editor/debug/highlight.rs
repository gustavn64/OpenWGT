//! The console's syntax colours: a line of JavaScript, of the built-in
//! command language or a value cut into its kinds of words, which the
//! console draws each in its colour. Lines are cut one at a time; a
//! comment or a template string still open at a line's end carries over
//! to the next ([`State`]).

use std::ops::Range;

/// What a piece of a line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tok {
    /// A keyword (`let`, `await`...), or a command's name.
    Keyword,
    /// `true`, `null`, `undefined`...
    Literal,
    Number,
    Str,
    Comment,
    /// A part by its number in the command language (`#7`).
    Part,
    /// The console's `> ` before what was typed.
    Prompt,
}

/// Which language a line is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// JavaScript, and the values the console shows.
    Js,
    /// The built-in commands ([`crate::script::cmd`]).
    Commands,
}

/// What's still open at a line's end.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Code,
    /// In `/* */`.
    Comment,
    /// In a `` ` `` string.
    Template,
}

const KEYWORDS: &[&str] = &[
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

const LITERALS: &[&str] = &["true", "false", "null", "undefined", "NaN", "Infinity", "this"];

/// The pieces of `line` in `syntax` that aren't plain, in order, with
/// what was open before it in `state` (and after it, on return).
#[must_use]
pub fn spans(line: &str, syntax: Syntax, state: &mut State) -> Vec<(Range<usize>, Tok)> {
    match syntax {
        Syntax::Js => js(line, state),
        Syntax::Commands => {
            *state = State::Code;
            commands(line)
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Where a string started at `from` (just past its quote `q`) ends: past
/// its closing quote, or the line's end; whether it closed.
fn string_end(line: &str, from: usize, q: char) -> (usize, bool) {
    let mut chars = line[from..].char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == q {
            return (from + i + c.len_utf8(), true);
        }
    }
    (line.len(), false)
}

/// Where a number starting at `from` ends: digits, letters (`0x1f`,
/// `1e3`, `10n`), `_`, a point, a sign after an exponent's `e`.
fn number_end(line: &str, from: usize) -> usize {
    let b = line.as_bytes();
    let hex = b.get(from) == Some(&b'0') && matches!(b.get(from + 1), Some(b'x' | b'X'));
    let mut i = from;
    while i < b.len() {
        let c = b[i];
        let exponent_sign = matches!(c, b'+' | b'-') && !hex && i > from && matches!(b[i - 1], b'e' | b'E');
        if !(c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || exponent_sign) {
            break;
        }
        i += 1;
    }
    i
}

fn js(line: &str, state: &mut State) -> Vec<(Range<usize>, Tok)> {
    let mut out = Vec::new();
    let mut i = 0;
    match *state {
        State::Comment => {
            let Some(e) = line.find("*/") else {
                out.push((0..line.len(), Tok::Comment));
                return out;
            };
            out.push((0..e + 2, Tok::Comment));
            i = e + 2;
            *state = State::Code;
        }
        State::Template => {
            let (e, closed) = string_end(line, 0, '`');
            out.push((0..e, Tok::Str));
            if !closed {
                return out;
            }
            i = e;
            *state = State::Code;
        }
        State::Code => {}
    }
    // The last character that wasn't a space: after a `.` a name is a
    // property's, not a keyword.
    let mut after_dot = false;
    while i < line.len() {
        let rest = &line[i..];
        let c = rest.chars().next().unwrap_or(' ');
        if rest.starts_with("//") {
            out.push((i..line.len(), Tok::Comment));
            break;
        }
        if let Some(body) = rest.strip_prefix("/*") {
            let Some(e) = body.find("*/") else {
                out.push((i..line.len(), Tok::Comment));
                *state = State::Comment;
                break;
            };
            let end = i + 2 + e + 2;
            out.push((i..end, Tok::Comment));
            i = end;
            continue;
        }
        if matches!(c, '"' | '\'' | '`') {
            let (e, closed) = string_end(line, i + 1, c);
            out.push((i..e, Tok::Str));
            if c == '`' && !closed {
                *state = State::Template;
            }
            i = e;
            after_dot = false;
            continue;
        }
        let digit_next = rest[c.len_utf8()..].starts_with(|d: char| d.is_ascii_digit());
        if c.is_ascii_digit() || (c == '.' && digit_next && !after_dot) {
            let e = number_end(line, i);
            out.push((i..e, Tok::Number));
            i = e;
            after_dot = false;
            continue;
        }
        if is_ident_start(c) {
            let e = rest.find(|d: char| !is_ident(d)).map_or(line.len(), |n| i + n);
            let word = &line[i..e];
            if !after_dot {
                if KEYWORDS.contains(&word) {
                    out.push((i..e, Tok::Keyword));
                } else if LITERALS.contains(&word) {
                    out.push((i..e, Tok::Literal));
                }
            }
            i = e;
            after_dot = false;
            continue;
        }
        if !c.is_whitespace() {
            after_dot = c == '.';
        }
        i += c.len_utf8();
    }
    out
}

/// The command language: the first word the command's, `#` a comment
/// unless a digit follows (a part), `"strings"`, numbers.
fn commands(line: &str) -> Vec<(Range<usize>, Tok)> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut first = true;
    // A line typed after `:` in the console.
    if line.trim_start().starts_with(':') {
        i = line.find(':').map_or(0, |n| n + 1);
    }
    while i < line.len() {
        let rest = &line[i..];
        let c = rest.chars().next().unwrap_or(' ');
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if c == '#' {
            if rest[1..].starts_with(|d: char| d.is_ascii_digit()) {
                let e = rest[1..]
                    .find(|d: char| !d.is_ascii_digit())
                    .map_or(line.len(), |n| i + 1 + n);
                out.push((i..e, Tok::Part));
                i = e;
                first = false;
                continue;
            }
            out.push((i..line.len(), Tok::Comment));
            break;
        }
        if c == '"' {
            let (e, _) = string_end(line, i + 1, '"');
            out.push((i..e, Tok::Str));
            i = e;
            first = false;
            continue;
        }
        let e = rest
            .find(|d: char| d.is_whitespace() || d == '"' || d == '#')
            .map_or(line.len(), |n| i + n);
        let word = &line[i..e];
        if first {
            out.push((i..e, Tok::Keyword));
        } else if word.starts_with(|d: char| d.is_ascii_digit() || d == '-') && word.parse::<f64>().is_ok() {
            out.push((i..e, Tok::Number));
        } else if matches!(word, "on" | "off") {
            out.push((i..e, Tok::Literal));
        }
        first = false;
        i = e;
    }
    out
}

/// `spans` over `len` bytes as a run of pieces covering them all, the
/// plain ones `None`; only the part in `range`, from its start.
#[must_use]
pub fn pieces(spans: &[(Range<usize>, Tok)], range: Range<usize>) -> Vec<(Range<usize>, Option<Tok>)> {
    let mut out = Vec::new();
    let mut at = range.start;
    for (r, t) in spans {
        let (s, e) = (r.start.max(range.start), r.end.min(range.end));
        if s >= e {
            continue;
        }
        if s > at {
            out.push((at - range.start..s - range.start, None));
        }
        out.push((s - range.start..e - range.start, Some(*t)));
        at = e;
    }
    if at < range.end {
        out.push((at - range.start..range.end - range.start, None));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pieces as `(text, kind)`, for reading.
    fn cut(line: &str, syntax: Syntax, state: &mut State) -> Vec<(String, Tok)> {
        spans(line, syntax, state)
            .into_iter()
            .map(|(r, t)| (line[r].to_owned(), t))
            .collect()
    }

    fn js_cut(line: &str) -> Vec<(String, Tok)> {
        cut(line, Syntax::Js, &mut State::Code)
    }

    fn of(v: &[(&str, Tok)]) -> Vec<(String, Tok)> {
        v.iter().map(|(s, t)| ((*s).to_owned(), *t)).collect()
    }

    #[test]
    fn javascript_words() {
        use Tok::*;
        assert_eq!(
            js_cut("let n = await ticks(60); // wait"),
            of(&[
                ("let", Keyword),
                ("await", Keyword),
                ("60", Number),
                ("// wait", Comment)
            ])
        );
        assert_eq!(
            js_cut(r#"log("a \"b\"", 'c', x.new, null)"#),
            of(&[(r#""a \"b\"""#, Str), ("'c'", Str), ("null", Literal)])
        );
        assert_eq!(
            js_cut("0x1F + 1e-3 + .5 + 10n + a1"),
            of(&[("0x1F", Number), ("1e-3", Number), (".5", Number), ("10n", Number)])
        );
        assert_eq!(js_cut("åsa = true"), of(&[("true", Literal)]));
    }

    #[test]
    fn comments_and_templates_go_on() {
        use Tok::*;
        let mut st = State::Code;
        assert_eq!(cut("a /* one", Syntax::Js, &mut st), of(&[("/* one", Comment)]));
        assert_eq!(st, State::Comment);
        assert_eq!(
            cut("two */ if", Syntax::Js, &mut st),
            of(&[("two */", Comment), ("if", Keyword)])
        );
        assert_eq!(st, State::Code);
        assert_eq!(cut("s = `x ${y}", Syntax::Js, &mut st), of(&[("`x ${y}", Str)]));
        assert_eq!(st, State::Template);
        assert_eq!(cut("z` + 1", Syntax::Js, &mut st), of(&[("z`", Str), ("1", Number)]));
        assert_eq!(st, State::Code);
    }

    #[test]
    fn command_words() {
        use Tok::*;
        let mut st = State::Code;
        assert_eq!(
            cut(r#"press "Big lamp".in2 #7 3.5 on # done"#, Syntax::Commands, &mut st),
            of(&[
                ("press", Keyword),
                (r#""Big lamp""#, Str),
                ("#7", Part),
                ("3.5", Number),
                ("on", Literal),
                ("# done", Comment)
            ])
        );
        assert_eq!(
            cut(":step 5", Syntax::Commands, &mut st),
            of(&[("step", Keyword), ("5", Number)])
        );
    }

    #[test]
    fn pieces_cover_a_range() {
        let sp = vec![(2..4, Tok::Number), (6..9, Tok::Str)];
        assert_eq!(
            pieces(&sp, 3..8),
            vec![(0..1, Some(Tok::Number)), (1..3, None), (3..5, Some(Tok::Str))]
        );
        assert_eq!(pieces(&[], 0..3), vec![(0..3, None)]);
    }
}
