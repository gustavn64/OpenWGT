//! OpenWGT's own help format: plain text with a little markup, turned
//! into the same topics as `WIDGET.HLP` (`hlp::HelpFile`) for the help
//! viewer. Written for the port's data pack (`engine::opendata`), where it
//! stands in for the CD's `WIDGET.HLP`.
//!
//! ```text
//! title: OpenWGT Help             the file's first line
//!
//! = Contents                      a topic: its title (the first is the
//!                                 contents), {#id} names it for links,
//! = Wires {#wires}                else its title in lower case, dashes
//! keywords: wire, knot, connect   for spaces; then the Index's words
//!
//! A paragraph: lines up to a blank line, joined. **bold**, *italic*,
//! [a link](#wires) to a topic.
//!
//! ## A heading in the topic
//! - a bullet point
//! ```
//!
//! Text is UTF-8 here and Windows-1252 in the topics, as `WIDGET.HLP`'s.

use std::collections::BTreeMap;
use std::fmt;

use crate::hlp::{Align, Font, HelpFile, Inline, Keyword, Link, LinkAction, ParaFormat, Paragraph, Topic};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No `title:` line first.
    NotHelpText,
    /// A link to a topic that isn't there.
    NoTopic(String),
    NoTopics,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotHelpText => f.write_str("not an OpenWGT help text (no title: line first)"),
            Error::NoTopic(id) => write!(f, "a link to #{id}, which isn't a topic"),
            Error::NoTopics => f.write_str("the help text has no topics"),
        }
    }
}

impl std::error::Error for Error {}

/// Whether `data` is a help text (its first line is `title:`, after a
/// byte order mark and blanks, as [`parse`] reads it).
#[must_use]
pub fn is_helptext(data: &[u8]) -> bool {
    let data = data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data);
    data.trim_ascii_start().starts_with(b"title:")
}

/// The fonts: normal, bold, italic, the topic's title, a heading.
const NORMAL: u16 = 0;
const BOLD: u16 = 1;
const ITALIC: u16 = 2;
const TITLE: u16 = 3;
const HEADING: u16 = 4;

fn fonts() -> Vec<Font> {
    let f = |half_points, attributes| Font {
        face: "Arial".into(),
        half_points,
        family: 3,
        attributes,
        colour: None,
        background: None,
    };
    vec![f(20, 0), f(20, 1), f(20, 2), f(28, 1), f(22, 1)]
}

fn slug(title: &str) -> String {
    title.to_lowercase().split_whitespace().collect::<Vec<_>>().join("-")
}

struct Raw {
    id: String,
    title: String,
    keywords: Vec<String>,
    /// Paragraphs: (kind, text) with kind `p`, `h` (heading) or `-`.
    paras: Vec<(char, String)>,
}

/// Inline markup of `text`: `**bold**`, `*italic*`, `[text](#id)`;
/// `ids` gives a topic's number for a link.
fn inline(text: &str, base: u16, ids: &BTreeMap<String, u32>) -> Result<Vec<Inline>, Error> {
    let mut out = Vec::new();
    let mut rest = text;
    let mut font = base;
    let push = |out: &mut Vec<Inline>, s: &str, font: u16, link: Option<Link>| {
        if !s.is_empty() {
            out.push(Inline::Text {
                text: crate::cp1252::encode(s),
                font,
                link,
            });
        }
    };
    while !rest.is_empty() {
        let next = rest.find(['*', '[']).unwrap_or(rest.len());
        push(&mut out, &rest[..next], font, None);
        rest = &rest[next..];
        if let Some(r) = rest.strip_prefix("**") {
            font = if font == BOLD { base } else { BOLD };
            rest = r;
        } else if let Some(r) = rest.strip_prefix('*') {
            font = if font == ITALIC { base } else { ITALIC };
            rest = r;
        } else if let Some(r) = rest.strip_prefix('[') {
            let Some((label, after)) = r.split_once("](#") else {
                push(&mut out, "[", font, None);
                rest = r;
                continue;
            };
            let Some((id, after)) = after.split_once(')') else {
                push(&mut out, "[", font, None);
                rest = r;
                continue;
            };
            let target = *ids.get(id).ok_or_else(|| Error::NoTopic(id.to_owned()))?;
            push(
                &mut out,
                label,
                font,
                Some(Link {
                    action: LinkAction::Jump(target),
                    visible: true,
                }),
            );
            rest = after;
        }
    }
    Ok(out)
}

/// The topics of a help text's lines (after its `title:` line).
fn read_topics<'a>(lines: impl Iterator<Item = &'a str>) -> Result<Vec<Raw>, Error> {
    let mut topics: Vec<Raw> = Vec::new();
    let mut para = String::new();
    let mut kind = 'p';
    let flush = |topics: &mut Vec<Raw>, para: &mut String, kind: &mut char| {
        if let Some(t) = topics.last_mut()
            && !para.trim().is_empty()
        {
            t.paras.push((*kind, para.trim().to_owned()));
        }
        para.clear();
        *kind = 'p';
    };
    for line in lines {
        let l = line.trim_end();
        if let Some(head) = l.strip_prefix("= ") {
            flush(&mut topics, &mut para, &mut kind);
            let (t, id) = match head.rsplit_once("{#") {
                Some((t, id)) => (t.trim().to_owned(), id.trim_end_matches('}').to_owned()),
                None => (head.trim().to_owned(), slug(head)),
            };
            topics.push(Raw {
                id,
                title: t,
                keywords: Vec::new(),
                paras: Vec::new(),
            });
        } else if let Some(k) = l.strip_prefix("keywords:")
            && para.is_empty()
            && let Some(t) = topics.last_mut()
        {
            t.keywords
                .extend(k.split(',').map(|w| w.trim().to_owned()).filter(|w| !w.is_empty()));
        } else if l.trim().is_empty() {
            flush(&mut topics, &mut para, &mut kind);
        } else if let Some(h) = l.strip_prefix("## ") {
            flush(&mut topics, &mut para, &mut kind);
            topics
                .last_mut()
                .ok_or(Error::NoTopics)?
                .paras
                .push(('h', h.trim().to_owned()));
        } else if let Some(b) = l.strip_prefix("- ") {
            flush(&mut topics, &mut para, &mut kind);
            kind = '-';
            para.push_str(b);
        } else {
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(l.trim());
        }
    }
    flush(&mut topics, &mut para, &mut kind);
    Ok(topics)
}

/// Parses a help text.
///
/// # Errors
/// If it doesn't start with `title:`, has no topics, or links to a topic
/// that isn't there.
pub fn parse(text: &str) -> Result<HelpFile, Error> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let title = lines
        .by_ref()
        .find(|l| !l.trim().is_empty())
        .and_then(|l| l.trim_start().strip_prefix("title:"))
        .ok_or(Error::NotHelpText)?
        .trim()
        .to_owned();
    let topics = read_topics(lines)?;
    if topics.is_empty() {
        return Err(Error::NoTopics);
    }
    // Topic n has offset and context n + 1.
    let number = |i: usize| u32::try_from(i + 1).unwrap_or(u32::MAX);
    let ids: BTreeMap<String, u32> = topics
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.clone(), number(i)))
        .collect();
    let mut out = Vec::new();
    let mut keywords: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (i, t) in topics.iter().enumerate() {
        let mut scroll = Vec::new();
        for (k, text) in &t.paras {
            let (format, font, prefix) = match k {
                'h' => (
                    ParaFormat {
                        space_above: 10,
                        space_below: 2,
                        ..ParaFormat::default()
                    },
                    HEADING,
                    "",
                ),
                '-' => (
                    ParaFormat {
                        space_above: 2,
                        left_indent: 24,
                        first_line_indent: -12,
                        ..ParaFormat::default()
                    },
                    NORMAL,
                    "\u{2022} ",
                ),
                _ => (
                    ParaFormat {
                        space_above: 6,
                        ..ParaFormat::default()
                    },
                    NORMAL,
                    "",
                ),
            };
            scroll.push(Paragraph {
                format,
                items: inline(&format!("{prefix}{text}"), font, &ids)?,
            });
        }
        for w in t.keywords.iter().chain(std::iter::once(&t.title)) {
            keywords.entry(w.clone()).or_default().push(number(i));
        }
        out.push(Topic {
            number: number(i),
            offset: number(i),
            title: t.title.clone(),
            macros: Vec::new(),
            browse_prev: i.checked_sub(1).map(number),
            browse_next: (i + 1 < topics.len()).then(|| number(i + 1)),
            non_scroll: vec![Paragraph {
                format: ParaFormat {
                    space_above: 4,
                    space_below: 4,
                    align: Align::Left,
                    ..ParaFormat::default()
                },
                items: vec![Inline::Text {
                    text: crate::cp1252::encode(&t.title),
                    font: TITLE,
                    link: None,
                }],
            }],
            scroll,
        });
    }
    Ok(HelpFile {
        version: (0, 0),
        title,
        copyright: String::new(),
        contents: 1,
        macros: Vec::new(),
        windows: Vec::new(),
        fonts: fonts(),
        topics: out,
        contexts: (0..topics.len()).map(|i| (number(i), number(i))).collect(),
        map: Vec::new(),
        keywords: keywords
            .into_iter()
            .map(|(word, topics)| Keyword { word, topics })
            .collect(),
        baggage: BTreeMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "title: Test Help\n\n= Contents\nSee [wires](#wires) and **bold** *it*.\n\n= Wires {#wires}\nkeywords: knot, cable\n\n## Making one\n- drag from a knot\n- let go on another\n";

    #[test]
    fn parses_topics_links_and_lists() {
        let h = parse(DOC).unwrap();
        assert!(is_helptext(DOC.as_bytes()));
        assert_eq!(h.title, "Test Help");
        assert_eq!(h.topics.len(), 2);
        assert_eq!(h.contents_topic().unwrap().title, "Contents");
        let p = &h.topics[0].scroll[0];
        assert!(p.items.iter().any(|i| matches!(
            i,
            Inline::Text {
                link: Some(Link {
                    action: LinkAction::Jump(2),
                    ..
                }),
                ..
            }
        )));
        assert_eq!(h.topic_for_hash(2).unwrap().title, "Wires");
        // A byte order mark or an indented title: still one, both ways.
        for doc in [format!("\u{feff}{DOC}"), format!("\n  {DOC}")] {
            assert!(is_helptext(doc.as_bytes()));
            assert_eq!(parse(&doc).unwrap().title, "Test Help");
        }
        let w = &h.topics[1];
        assert_eq!(w.scroll.len(), 3);
        assert!(w.scroll[1].plain_text().starts_with('\u{95}') || w.scroll[1].plain_text().starts_with('•'));
        assert!(h.keywords.iter().any(|k| k.word == "cable" && k.topics == [2]));
    }

    #[test]
    fn a_link_to_nothing_is_an_error() {
        assert_eq!(
            parse("title: T\n= A\n[x](#nowhere)\n").map(|_| ()),
            Err(Error::NoTopic("nowhere".into()))
        );
    }
}
