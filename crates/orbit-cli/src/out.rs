//! Output: human tables, the `--json` envelope (`"v":1`), and failures with their exit codes.
use serde_json::{Value, json};
use std::fmt::Display;

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
pub fn table(head: &[&str], rows: &[Vec<String>]) -> String {
    let mut width: Vec<usize> = head.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            width[i] = width[i].max(cell.chars().count());
        }
    }
    let line = |cells: Vec<&str>| {
        let mut s = String::new();
        for (i, cell) in cells.iter().enumerate() {
            if i + 1 == cells.len() {
                s.push_str(cell);
            } else {
                s.push_str(&format!("{:<w$}  ", cell, w = width[i]));
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
