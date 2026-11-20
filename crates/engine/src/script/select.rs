//! Finding parts: selectors as text (`#7`, `Lamp`, `"Big lamp".in2`,
//! `LightBulb[2]`, `@120,80`), and which part one means, with
//! suggestions when none or several do.
//!
//! A text is looked for as a name first, then as a label, then as a kind
//! (`LightBulb`, `Light Bulb`) or tag (`bulb`), each ignoring case; the
//! first that finds any decides. `[n]` picks the `n`th (from 1) of several,
//! in reading order.

use std::fmt::Write as _;

use super::{ErrorKind, HostError, HostResult, KnotInfo, KnotSel, PartInfo, PartSel, Selector, value};

/// Reads a selector.
///
/// # Errors
/// What's wrong with it.
pub fn parse(s: &str) -> Result<Selector, String> {
    let (part, rest) = if let Some(q) = s.strip_prefix('"') {
        let end = q.find('"').ok_or_else(|| format!("{s}: the quote isn't closed"))?;
        let text = &q[..end];
        if text.is_empty() {
            return Err("an empty name".into());
        }
        (
            PartSel::Text {
                text: text.to_owned(),
                nth: None,
            },
            &q[end + 1..],
        )
    } else {
        let end = s.find(['.', '[']).unwrap_or(s.len());
        let (head, rest) = s.split_at(end);
        let part = if let Some(id) = head.strip_prefix('#') {
            PartSel::Id(
                id.parse()
                    .map_err(|_| format!("{head}: an id is # and a number, such as #7"))?,
            )
        } else if let Some(at) = head.strip_prefix('@') {
            let (x, y) = at
                .split_once(',')
                .and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)))
                .ok_or_else(|| format!("{head}: a point is @ and x,y, such as @120,80"))?;
            PartSel::At(x, y)
        } else if head.is_empty() {
            return Err(format!("{s}: no part named"));
        } else {
            PartSel::Text {
                text: head.to_owned(),
                nth: None,
            }
        };
        (part, rest)
    };
    let (part, rest) = match rest.strip_prefix('[') {
        Some(r) => {
            let end = r.find(']').ok_or_else(|| format!("{s}: the [ isn't closed"))?;
            let n: u32 = r[..end]
                .parse()
                .ok()
                .filter(|&n| n > 0)
                .ok_or_else(|| format!("{s}: [n] counts from 1"))?;
            let part = match part {
                PartSel::Text { text, .. } => PartSel::Text { text, nth: Some(n) },
                _ => return Err(format!("{s}: [n] goes after a name, label or kind")),
            };
            (part, &r[end + 1..])
        }
        None => (part, rest),
    };
    let knot = match rest {
        "" => None,
        r => Some(
            parse_knot(r.strip_prefix('.').ok_or_else(|| format!("{s}: unexpected {r}"))?)
                .map_err(|e| format!("{s}: {e}"))?,
        ),
    };
    Ok(Selector { part, knot })
}

/// `in`, `in2`, `out`, `out3` or a knot's number.
fn parse_knot(k: &str) -> Result<KnotSel, String> {
    let nth = |n: &str| -> Result<u8, String> {
        if n.is_empty() {
            return Ok(0);
        }
        n.parse::<u8>()
            .ok()
            .filter(|&n| n > 0)
            .map(|n| n - 1)
            .ok_or_else(|| format!("{k}: inputs and outputs count from 1"))
    };
    if let Some(n) = k.strip_prefix("in") {
        Ok(KnotSel::In(nth(n)?))
    } else if let Some(n) = k.strip_prefix("out") {
        Ok(KnotSel::Out(nth(n)?))
    } else {
        k.parse()
            .map(KnotSel::Index)
            .map_err(|_| format!("knot {k}: say in, in2, out, out2... or the knot's number"))
    }
}

/// `sel` as a script would write it.
#[must_use]
pub fn show(sel: &Selector) -> String {
    let mut s = show_part(&sel.part);
    match sel.knot {
        None => {}
        Some(KnotSel::In(0)) => s.push_str(".in"),
        Some(KnotSel::In(n)) => {
            let _ = write!(s, ".in{}", n + 1);
        }
        Some(KnotSel::Out(0)) => s.push_str(".out"),
        Some(KnotSel::Out(n)) => {
            let _ = write!(s, ".out{}", n + 1);
        }
        Some(KnotSel::Index(n)) => {
            let _ = write!(s, ".{n}");
        }
    }
    s
}

#[must_use]
pub fn show_part(p: &PartSel) -> String {
    match p {
        PartSel::Id(id) => format!("#{id}"),
        PartSel::At(x, y) => format!("@{x},{y}"),
        PartSel::Text { text, nth } => {
            let mut s = value::quote(text);
            if let Some(n) = nth {
                let _ = write!(s, "[{n}]");
            }
            s
        }
    }
}

/// `w` for several: `Light Bulbs`, `Switches`.
#[must_use]
pub fn plural(w: &str) -> String {
    if w.ends_with(['s', 'x', 'z']) || w.ends_with("ch") || w.ends_with("sh") {
        format!("{w}es")
    } else {
        format!("{w}s")
    }
}

/// `LightBulb` in words: `Light Bulb`; `RGBDisplay`: `RGB Display`.
#[must_use]
pub fn words(kind: &str) -> String {
    let c: Vec<char> = kind.chars().collect();
    let mut out = String::new();
    for (i, &ch) in c.iter().enumerate() {
        if i > 0 {
            let prev = c[i - 1];
            let next_lower = c.get(i + 1).is_some_and(char::is_ascii_lowercase);
            let gap = (ch.is_ascii_uppercase() && (prev.is_ascii_lowercase() || prev.is_ascii_digit()))
                || (ch.is_ascii_uppercase() && prev.is_ascii_uppercase() && next_lower)
                || (ch.is_ascii_digit() && prev.is_ascii_alphabetic());
            if gap {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
}

/// A kind written as a kind, its words or its tag, for comparing:
/// lower case, no spaces.
fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// How a part matched a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum By {
    Name,
    Label,
    Kind,
}

/// The parts `text` finds, and how.
fn find<'a>(parts: &'a [PartInfo], text: &str) -> (Vec<&'a PartInfo>, By) {
    let exact: Vec<&PartInfo> = parts.iter().filter(|p| p.name.as_deref() == Some(text)).collect();
    if !exact.is_empty() {
        return (exact, By::Name);
    }
    let low = text.to_lowercase();
    let named: Vec<&PartInfo> = parts
        .iter()
        .filter(|p| p.name.as_ref().is_some_and(|n| n.to_lowercase() == low))
        .collect();
    if !named.is_empty() {
        return (named, By::Name);
    }
    let labelled: Vec<&PartInfo> = parts
        .iter()
        .filter(|p| p.label.as_ref().is_some_and(|n| n.to_lowercase() == low))
        .collect();
    if !labelled.is_empty() {
        return (labelled, By::Label);
    }
    let sq = squash(text);
    let kinds = parts
        .iter()
        .filter(|p| squash(&p.kind) == sq || p.tag.to_lowercase() == low)
        .collect();
    (kinds, By::Kind)
}

/// The part `sel` names among `parts` (in reading order).
///
/// # Errors
/// [`ErrorKind::UnknownPart`] with the nearest names, labels and kinds,
/// or [`ErrorKind::Ambiguous`] with how to say each candidate.
pub fn resolve<'a>(parts: &'a [PartInfo], sel: &PartSel) -> HostResult<&'a PartInfo> {
    match sel {
        PartSel::Id(id) => parts.iter().find(|p| p.id.0 == *id).ok_or_else(|| {
            HostError::new(
                ErrorKind::UnknownPart {
                    query: format!("#{id}"),
                    suggestions: Vec::new(),
                },
                format!("no part #{id} in this widget"),
            )
        }),
        PartSel::At(x, y) => parts
            .iter()
            .rev()
            .find(|p| {
                let (x0, y0, x1, y1) = p.rect;
                (x0..x1).contains(x) && (y0..y1).contains(y)
            })
            .ok_or_else(|| {
                HostError::new(
                    ErrorKind::UnknownPart {
                        query: format!("@{x},{y}"),
                        suggestions: Vec::new(),
                    },
                    format!("no part at {x},{y}"),
                )
            }),
        PartSel::Text { text, nth } => {
            let (found, by) = find(parts, text);
            match (found.len(), nth) {
                (0, _) => Err(unknown(parts, text)),
                (1, None | Some(1)) => Ok(found[0]),
                (n, Some(k)) => found
                    .get(usize::try_from(*k).unwrap_or(usize::MAX) - 1)
                    .copied()
                    .ok_or_else(|| {
                        HostError::new(
                            ErrorKind::UnknownPart {
                                query: show_part(sel),
                                suggestions: Vec::new(),
                            },
                            format!(
                                "there {} only {}, so no {}",
                                if n == 1 { "is" } else { "are" },
                                many(n, &found, by, text),
                                show_part(sel)
                            ),
                        )
                    }),
                (n, None) => Err(ambiguous(text, &found, by, n)),
            }
        }
    }
}

/// `3 Light Bulbs`, `2 parts labelled "Lamp"`.
fn many(n: usize, found: &[&PartInfo], by: By, text: &str) -> String {
    match by {
        By::Kind => {
            let w = found.first().map_or_else(|| text.to_owned(), |p| p.kind_words());
            if n == 1 {
                format!("1 {w}")
            } else {
                format!("{n} {}", plural(&w))
            }
        }
        By::Label => format!(
            "{n} part{} labelled {}",
            if n == 1 { "" } else { "s" },
            value::quote(text)
        ),
        By::Name => format!("{n} part{} named {}", if n == 1 { "" } else { "s" }, value::quote(text)),
    }
}

fn ambiguous(text: &str, found: &[&PartInfo], by: By, n: usize) -> HostError {
    let list: Vec<String> = found
        .iter()
        .map(|p| {
            let at = format!("{} at ({},{})", p.id, p.rect.0, p.rect.1);
            if by == By::Kind {
                at
            } else {
                format!("{} {at}", p.kind_words())
            }
        })
        .collect();
    let first = match by {
        By::Kind => found[0].kind.clone(),
        By::Label | By::Name => value::quote(text),
    };
    // Quoted for the message, unless it's quoted already.
    let q = |s: String| if s.starts_with('"') { s } else { format!("\"{s}\"") };
    HostError::new(
        ErrorKind::Ambiguous {
            candidates: found.iter().map(|p| p.id.to_string()).collect(),
        },
        format!(
            "{}: {}. Say which: {}, {}, or name one",
            many(n, found, by, text),
            list.join(", "),
            q(format!("{first}[2]")),
            q(found[1].id.to_string())
        ),
    )
}

/// No part answers to `text`: the nearest names, labels and kinds.
fn unknown(parts: &[PartInfo], text: &str) -> HostError {
    let sq = squash(text);
    let low = text.to_lowercase();
    // A kind the game has, but not this widget.
    if let Some(c) = crate::gadgets::CLASSES.iter().find(|c| {
        let kind = c.name.trim_start_matches('c');
        squash(kind) == sq || c.tag.to_string().to_lowercase() == low
    }) {
        let w = words(c.name.trim_start_matches('c'));
        return HostError::new(
            ErrorKind::UnknownPart {
                query: text.to_owned(),
                suggestions: Vec::new(),
            },
            format!("there's no {w} in this widget"),
        );
    }
    let mut near: Vec<(usize, String, String)> = Vec::new();
    for p in parts {
        for (said, cost) in [
            (p.name.clone(), 0),
            (p.label.clone(), 0),
            (Some(p.kind.clone()), 0),
            (Some(p.tag.clone()), 1),
        ] {
            let Some(said) = said else { continue };
            let d = distance(&squash(&said), &sq) + cost;
            let limit = (sq.chars().count() / 3).max(1);
            if d <= limit && !near.iter().any(|(_, s, _)| *s == said) {
                let who = format!("{} {}", p.kind_words(), p.id);
                near.push((d, said, who));
            }
        }
    }
    near.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    near.truncate(3);
    let mut msg = format!("no part called \"{text}\"");
    if !near.is_empty() {
        let s: Vec<String> = near
            .iter()
            .map(|(_, said, who)| format!("\"{said}\" ({who})"))
            .collect();
        let _ = write!(msg, ". Did you mean {}?", s.join(" or "));
    }
    HostError::new(
        ErrorKind::UnknownPart {
            query: text.to_owned(),
            suggestions: near.into_iter().map(|(_, s, _)| s).collect(),
        },
        msg,
    )
}

/// The edit distance between `a` and `b` (Levenshtein, by characters).
#[must_use]
pub fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diag = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let up = row[j + 1];
            row[j + 1] = (up + 1).min(row[j] + 1).min(diag + usize::from(ca != cb));
            diag = up;
        }
    }
    row[b.len()]
}

/// Knot `k` of part `p`.
///
/// # Errors
/// [`ErrorKind::BadKnot`] if it has no such knot, saying which it has.
pub fn knot(p: &PartInfo, k: KnotSel) -> HostResult<&KnotInfo> {
    let found = match k {
        KnotSel::Index(i) => p.knots.get(usize::from(i)),
        KnotSel::In(i) => p.inputs().nth(usize::from(i)),
        KnotSel::Out(i) => p.outputs().nth(usize::from(i)),
    };
    found.ok_or_else(|| {
        let what = match k {
            KnotSel::Index(i) => format!("knot {i}"),
            KnotSel::In(i) => format!("input {}", i + 1),
            KnotSel::Out(i) => format!("output {}", i + 1),
        };
        HostError::new(
            ErrorKind::BadKnot,
            format!("{} has {}; {what} doesn't exist", p.describe(), knots_said(p)),
        )
    })
}

/// `1 input (in) and no outputs`.
#[must_use]
pub fn knots_said(p: &PartInfo) -> String {
    let count = |n: usize, one: &str, name: &str| match n {
        0 => format!("no {one}s"),
        1 => format!("1 {one} ({name})"),
        n => format!("{n} {one}s ({name} to {name}{n})"),
    };
    format!(
        "{} and {}",
        count(p.inputs().count(), "input", "in"),
        count(p.outputs().count(), "output", "out")
    )
}

/// Part `p`'s `n`th input or output index among its knots.
#[must_use]
pub fn nth_knot(p: &PartInfo, input: bool, n: u8) -> Option<u8> {
    p.knots
        .iter()
        .filter(|k| k.input == input)
        .nth(usize::from(n))
        .map(|k| k.index)
}
