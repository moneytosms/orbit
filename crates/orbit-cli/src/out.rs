//! Output: human tables, the `--json` envelope (`"v":1`), and failures with their exit codes.
use serde_json::{Value, json};
use std::fmt::Display;
use unicode_width::UnicodeWidthStr;

/// A failed command. `code` is stable for scripts; `exit` is 1 for a failed command, 2 for usage.
#[derive(Debug)]
pub struct Fail {
    pub code: &'static str,
    pub message: String,
    pub hint: Option<String>,
    pub exit: u8,
}

pub type R<T> = Result<T, Fail>;

/// What a command prints: the human text and the `--json` body (without `v`).
pub type Done = R<(String, Value)>;

pub fn fail(code: &'static str, message: impl Into<String>, hint: Option<&str>) -> Fail {
    Fail {
        code,
        message: message.into(),
        hint: hint.map(Into::into),
        exit: 1,
    }
}

pub fn usage(message: impl Into<String>) -> Fail {
    Fail {
        code: "usage",
        message: message.into(),
        hint: Some("run `orbit --help`".into()),
        exit: 2,
    }
}

/// An unexpected failure (I/O, SQLite, serde). Exit 1.
pub fn internal(e: impl Display) -> Fail {
    fail("internal", e.to_string(), None)
}

pub fn print_ok(json: bool, human: &str, mut body: Value) {
    if json {
        body["v"] = json!(1);
        println!("{body}");
    } else if !human.is_empty() {
        println!("{human}");
    }
}

pub fn print_err(json: bool, f: &Fail) {
    if json {
        let err = json!({ "code": f.code, "message": f.message, "hint": f.hint });
        println!("{}", json!({ "v": 1, "error": err }));
    } else {
        eprintln!("error: {}", f.message);
        if let Some(hint) = &f.hint {
            eprintln!("hint: {hint}");
        }
    }
}

/// Left-aligned columns separated by two spaces. The last column is not padded.
fn visible(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(visible(s).as_str())
}

fn truncate(s: &str, max: usize) -> String {
    if display_width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let plain = visible(s);
    let mut out = String::new();
    for c in plain.chars() {
        if UnicodeWidthStr::width(format!("{out}{c}…").as_str()) > max {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}

/// Renders a table within terminal columns, shrinking the title first.
pub fn table(head: &[&str], rows: &[Vec<String>], max_width: usize) -> String {
    let mut widths: Vec<usize> = head.iter().map(|h| UnicodeWidthStr::width(*h)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }
    let gaps = head.len().saturating_sub(1) * 2;
    let available = max_width.saturating_sub(gaps);
    while widths.iter().sum::<usize>() > available {
        let i = (1..widths.len()).max_by_key(|i| widths[*i]).unwrap_or(0);
        if widths[i] <= 1 {
            break;
        }
        widths[i] -= 1;
    }
    let line = |cells: Vec<&str>| {
        let mut s = String::new();
        for (i, cell) in cells.iter().enumerate() {
            let cell = truncate(cell, widths[i]);
            if i + 1 == cells.len() {
                s.push_str(&cell);
            } else {
                let pad = widths[i].saturating_sub(display_width(cell.as_str()));
                s.push_str(&cell);
                s.push_str(&" ".repeat(pad));
                s.push_str("  ");
            }
        }
        s.trim_end().to_string()
    };
    let mut lines = vec![line(head.to_vec())];
    lines.extend(
        rows.iter()
            .map(|r| line(r.iter().map(String::as_str).collect())),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_stays_within_unicode_terminal_widths() {
        let rows = vec![vec![
            "1".into(),
            "買い物 🛒 with a very long title that should wrap nowhere ".repeat(4),
            "\u{1b}[31m2026-11-22\u{1b}[0m".into(),
            "P1".into(),
            "Home / Project".into(),
        ]];
        for columns in [60, 80, 120] {
            let output = table(&["#", "TITLE", "DUE", "P", "CONTEXT"], &rows, columns);
            assert!(output.lines().all(|line| display_width(line) <= columns));
            assert!(output.contains('…'));
        }
    }
}
