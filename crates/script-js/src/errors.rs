//! JavaScript errors as [`ScriptError`]s: where they happened (from the
//! `stack` QuickJS gives every error), the source line and how much of it
//! to underline.

use engine::script::runtime::{ErrKind, ScriptError};
use rquickjs::{Ctx, Value};

/// The file name of the API's own code, whose frames a script's errors
/// don't point at.
pub const PRELUDE: &str = "openwgt:prelude";

/// What the runtime tells scripts that ran too long.
pub const BUDGET: &str = "the script ran too long without waiting (use `await ticks(1)` in long loops); stopped";

/// A frame of a stack: function, file, line, column (columns as QuickJS
/// counts them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub func: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// The frames of a QuickJS `stack`: `    at f (file.js:2:10)` or, for a
/// syntax error, `    at file.js:2:10`; frames without a place
/// (`at map (native)`) are left out.
#[must_use]
pub fn frames(stack: &str) -> Vec<Frame> {
    stack
        .lines()
        .filter_map(|l| {
            let l = l.trim().strip_prefix("at ")?;
            let (func, place) = match l.strip_suffix(')').and_then(|l| l.rsplit_once(" (")) {
                Some((f, p)) => (f.to_owned(), p),
                None => (String::new(), l),
            };
            let mut it = place.rsplitn(3, ':');
            let column = it.next()?.parse().ok()?;
            let line = it.next()?.parse().ok()?;
            let file = it.next()?.to_owned();
            Some(Frame {
                func,
                file,
                line,
                column,
            })
        })
        .collect()
}

/// What a thrown value says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Thrown {
    /// `TypeError`, `WidgetError`...; empty for a value that isn't an
    /// error.
    pub name: String,
    pub message: String,
    pub stack: String,
    /// A `WidgetError`'s `code`.
    pub code: Option<String>,
    /// What a `WidgetError`'s part was asked for as (to underline it).
    pub query: Option<String>,
    /// `null`: what QuickJS throws when it can't even make the error (out
    /// of memory).
    pub null: bool,
}

impl Thrown {
    /// Reads a thrown value. Its properties are read as they are, getters
    /// and all; nothing here may hold the host.
    #[must_use]
    pub fn read<'js>(ctx: &Ctx<'js>, v: &Value<'js>) -> Thrown {
        if v.is_null() || v.is_undefined() {
            return Thrown {
                message: if v.is_null() { "null" } else { "undefined" }.into(),
                null: v.is_null(),
                ..Thrown::default()
            };
        }
        let Some(o) = v.as_object() else {
            return Thrown {
                message: text(ctx, v),
                ..Thrown::default()
            };
        };
        let s = |k: &str| -> Option<String> {
            let v: Value = o.get(k).ok()?;
            if v.is_undefined() || v.is_null() {
                None
            } else {
                Some(text(ctx, &v))
            }
        };
        let is_error = v.is_error() || s("stack").is_some();
        if !is_error {
            return Thrown {
                message: text(ctx, v),
                ..Thrown::default()
            };
        }
        let query = o
            .get::<_, Value>("part")
            .ok()
            .and_then(|p| p.as_string().and_then(|s| s.to_string().ok()));
        Thrown {
            name: s("name").unwrap_or_else(|| "Error".into()),
            message: s("message").unwrap_or_default(),
            stack: s("stack").unwrap_or_default(),
            code: s("code"),
            query,
            null: false,
        }
    }

    /// The error's kind, given whether the budget ran out meanwhile.
    #[must_use]
    pub fn kind(&self, budget_spent: bool) -> ErrKind {
        if budget_spent && self.name == "InternalError" && self.message == "interrupted" {
            return ErrKind::Budget;
        }
        if self.null || self.message == "out of memory" {
            return ErrKind::Memory;
        }
        match (self.name.as_str(), self.code.as_deref()) {
            ("SyntaxError", _) => ErrKind::Syntax,
            ("AssertionError", _) => ErrKind::Assert,
            ("WidgetError", Some("Timeout")) => ErrKind::Timeout,
            ("WidgetError", Some("Stopped")) => ErrKind::Cancelled,
            ("WidgetError", _) => ErrKind::Host,
            _ => ErrKind::Runtime,
        }
    }

    /// As a [`ScriptError`] in `file` (whose lines are `src`).
    #[must_use]
    pub fn error(&self, kind: ErrKind, file: &str, src: &[String], memory: usize) -> ScriptError {
        let message = match kind {
            ErrKind::Budget => BUDGET.to_owned(),
            ErrKind::Memory => format!(
                "the script took more memory than it may ({} MiB); stopped",
                memory >> 20
            ),
            ErrKind::Host | ErrKind::Timeout | ErrKind::Cancelled | ErrKind::Assert => self.message.clone(),
            _ if self.name.is_empty() => format!("uncaught {}", self.message),
            _ => format!("{}: {}", self.name, self.message),
        };
        at(kind, message, &self.stack, file, src, self.query.as_deref())
    }
}

/// `v` as text (`String(v)`), or how it failed.
fn text<'js>(ctx: &Ctx<'js>, v: &Value<'js>) -> String {
    if let Some(s) = v.as_string() {
        return s.to_string().unwrap_or_default();
    }
    let f: rquickjs::Result<rquickjs::Function> = ctx.globals().get("String");
    f.and_then(|f| f.call::<_, String>((v.clone(),)))
        .unwrap_or_else(|_| "(a value that can't be shown)".into())
}

/// An error at the first frame of `stack` in `file`: its line, the
/// column in characters, and what to underline (`query`, in quotes, if
/// the line has it there; else the word at the column).
#[must_use]
pub fn at(kind: ErrKind, message: String, stack: &str, file: &str, src: &[String], query: Option<&str>) -> ScriptError {
    at_call(kind, message, stack, file, src, query, None)
}

/// [`at`], underlining the call of `callee` the place is in (QuickJS
/// places a call at its last argument).
#[must_use]
pub fn at_call(
    kind: ErrKind,
    message: String,
    stack: &str,
    file: &str,
    src: &[String],
    query: Option<&str>,
    callee: Option<&str>,
) -> ScriptError {
    let all = frames(stack);
    let mine: Vec<&Frame> = all.iter().filter(|f| f.file == file).collect();
    let Some(first) = mine.first() else {
        return ScriptError::at(kind, message, file, 0, 0, 0, None);
    };
    let line = src.get(first.line.saturating_sub(1) as usize).map(|l| l.trim_end());
    let (column, width) = match line {
        Some(l) => callee
            .and_then(|c| call_place(l, first.column, c))
            .unwrap_or_else(|| place(l, first.column, query)),
        None => (first.column, 1),
    };
    let mut e = ScriptError::at(kind, message, file, first.line, column, width, line);
    if mine.len() > 1 {
        e.stack = mine[1..]
            .iter()
            .map(|f| {
                let col = src
                    .get(f.line.saturating_sub(1) as usize)
                    .map_or(f.column, |l| place(l, f.column, None).0);
                (
                    if f.func.is_empty() {
                        "<script>".into()
                    } else {
                        f.func.clone()
                    },
                    f.line,
                    col,
                )
            })
            .collect();
    }
    e
}

/// Where the call of `callee` that QuickJS's column (in bytes) is in
/// starts: the last `callee(` before it.
fn call_place(line: &str, column: u32, callee: &str) -> Option<(u32, u32)> {
    let byte = (column as usize).min(line.len());
    let byte = (byte..=line.len()).find(|&b| line.is_char_boundary(b))?;
    let needle = format!("{callee}(");
    let at = line[..byte].rfind(&needle).or_else(|| line.find(&needle))?;
    let count = |s: &str| u32::try_from(s.chars().count()).unwrap_or(u32::MAX);
    Some((count(&line[..at]) + 1, count(callee)))
}

/// QuickJS's column (from 1, in bytes) in `line` as characters, and how
/// many to underline.
fn place(line: &str, column: u32, query: Option<&str>) -> (u32, u32) {
    let byte = (column.saturating_sub(1) as usize).min(line.len());
    let byte = (0..=byte).rev().find(|&b| line.is_char_boundary(b)).unwrap_or(0);
    let count = |s: &str| u32::try_from(s.chars().count()).unwrap_or(u32::MAX);
    // The part asked for, quoted, at or after the place.
    if let Some(q) = query.filter(|q| !q.is_empty()) {
        for quote in ['"', '\'', '`'] {
            let needle = format!("{quote}{q}{quote}");
            if let Some(i) = line[byte..].find(&needle) {
                return (count(&line[..byte + i]) + 1, count(&needle));
            }
        }
    }
    let word = line[byte..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
        .count();
    (count(&line[..byte]) + 1, u32::try_from(word.max(1)).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacks_read() {
        let f =
            frames("    at f (rt.js:2:10)\n    at map (native)\n    at <eval> (dir/a b.js:7:1)\n    at syn.js:2:10\n");
        assert_eq!(f.len(), 3);
        assert_eq!(
            (f[0].func.as_str(), f[0].file.as_str(), f[0].line, f[0].column),
            ("f", "rt.js", 2, 10)
        );
        assert_eq!((f[1].func.as_str(), f[1].file.as_str()), ("<eval>", "dir/a b.js"));
        assert_eq!((f[2].func.as_str(), f[2].file.as_str(), f[2].line), ("", "syn.js", 2));
    }

    #[test]
    fn the_place_is_underlined() {
        let src = vec![String::new(), "const lamp = part(\"Lamb\");".to_owned()];
        let stack =
            format!("    at h ({PRELUDE}:30:7)\n    at part ({PRELUDE}:200:5)\n    at <eval> (check.js:2:14)\n");
        let e = at(
            ErrKind::Host,
            "no part called \"Lamb\"".into(),
            &stack,
            "check.js",
            &src,
            Some("Lamb"),
        );
        assert_eq!((e.line, e.column, e.width), (2, 19, 6));
        assert_eq!(
            e.to_string(),
            "check.js:2:19  no part called \"Lamb\"\n 2 | const lamp = part(\"Lamb\");\n   |                   ^^^^^^"
        );
        // Characters, not bytes; the word at the place.
        let src = vec!["let å = ö(1);".to_owned()];
        let e = at(
            ErrKind::Runtime,
            "x".into(),
            "    at <eval> (a.js:1:10)\n",
            "a.js",
            &src,
            None,
        );
        assert_eq!((e.column, e.width), (9, 1));
    }
}
