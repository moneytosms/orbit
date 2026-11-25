//! Multi-line capture: the same rules as the app's paste splitter (`app/src/lib/pasteLines.ts`),
//! ported so `orbit add -` and `orbit add --lines` read a pasted list the way the app does.

/// One non-empty line: its text with any list marker removed, and how deep it nests (0 to 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub depth: usize,
}

const MAX_DEPTH: usize = 2;

/// Indentation width: a space counts 1, a tab 2.
fn indent_of(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        match c {
            ' ' => n += 1,
            '\t' => n += 2,
            _ => break,
        }
    }
    n
}

/// Removes one leading list marker: `- `, `* `, `+ `, `• `, `1. `, `2) `, `[ ] ` or `- [x] `.
/// Each marker needs whitespace (or the end of the line) after it, so `-buy` keeps its dash.
fn strip_marker(s: &str) -> &str {
    // The rest of the line after a marker that needs whitespace (or the end) following it.
    fn after_space(rest: &str) -> Option<&str> {
        match rest.chars().next() {
            None => Some(""),
            Some(c) if c.is_whitespace() => Some(rest.trim_start()),
            _ => None,
        }
    }
    // The rest after an optional `[ ]`, `[x]` or `[X]` checkbox.
    fn checkbox(rest: &str) -> Option<&str> {
        let bytes = rest.as_bytes();
        if bytes.len() >= 3
            && bytes[0] == b'['
            && bytes[2] == b']'
            && matches!(bytes[1], b' ' | b'x' | b'X')
        {
            after_space(&rest[3..])
        } else {
            None
        }
    }
    let first = s.chars().next();
    match first {
        Some('-' | '*' | '+' | '•') => {
            let rest = &s[first.map_or(0, char::len_utf8)..];
            match after_space(rest) {
                Some(rest) => checkbox(rest).unwrap_or(rest),
                None => s,
            }
        }
        Some(c) if c.is_ascii_digit() => {
            let digits = s.bytes().take_while(u8::is_ascii_digit).count();
            let rest = &s[digits..];
            match rest.chars().next() {
                Some('.' | ')') => after_space(&rest[1..]).unwrap_or(s),
                _ => s,
            }
        }
        Some('[') => checkbox(s).unwrap_or(s),
        _ => s,
    }
}

/// Splits pasted text into lines. Blank lines drop. Deeper indentation than the line above nests
/// one level, capped at two levels of subtasks.
pub fn split(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    // Indents of the lines still open above the current one.
    let mut stack: Vec<usize> = Vec::new();
    for raw in text.lines() {
        if raw.trim().is_empty() {
            continue;
        }
        let indent = indent_of(raw);
        let line = strip_marker(raw.trim()).trim();
        if line.is_empty() {
            continue;
        }
        while stack.last().is_some_and(|top| *top >= indent) {
            stack.pop();
        }
        let depth = stack.len().min(MAX_DEPTH);
        if stack.len() <= MAX_DEPTH {
            stack.push(indent);
        }
        out.push(Line {
            text: line.to_string(),
            depth,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(lines: &[Line]) -> Vec<(&str, usize)> {
        lines.iter().map(|l| (l.text.as_str(), l.depth)).collect()
    }

    #[test]
    fn plain_lines_drop_blanks_and_keep_order() {
        let lines = split("buy milk\n\n   \ncall mum\r\n");
        assert_eq!(texts(&lines), vec![("buy milk", 0), ("call mum", 0)]);
    }

    #[test]
    fn list_markers_are_stripped() {
        let lines = split("- eggs\n* bread\n+ tea\n• jam\n1. rice\n2) beans\n[ ] soap\n- [x] salt\n- [X] pepper");
        let got: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            got,
            vec!["eggs", "bread", "tea", "jam", "rice", "beans", "soap", "salt", "pepper"]
        );
    }

    #[test]
    fn marker_needs_a_space_after_it() {
        let lines = split("-buy milk\n2026 plan\n1.5 litres\n[x]y");
        let got: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(got, vec!["-buy milk", "2026 plan", "1.5 litres", "[x]y"]);
    }

    #[test]
    fn marker_only_lines_are_dropped() {
        assert!(split("-\n*   \n1.\n[ ]").is_empty());
    }

    #[test]
    fn indented_lines_nest_under_the_line_above() {
        let lines = split("trip\n  passport\n  tickets\nlater");
        assert_eq!(
            texts(&lines),
            vec![("trip", 0), ("passport", 1), ("tickets", 1), ("later", 0)]
        );
    }

    #[test]
    fn nesting_is_capped_at_two_levels() {
        let lines = split("a\n  b\n    c\n      d\n        e\n  f");
        assert_eq!(
            texts(&lines),
            vec![("a", 0), ("b", 1), ("c", 2), ("d", 2), ("e", 2), ("f", 1)]
        );
    }

    #[test]
    fn tabs_count_two_columns() {
        let lines = split("a\n\tb\n  c");
        assert_eq!(texts(&lines), vec![("a", 0), ("b", 1), ("c", 1)]);
    }

    #[test]
    fn dedent_returns_to_a_shallower_parent() {
        let lines = split("a\n  b\n    c\n  d\ne");
        assert_eq!(
            texts(&lines),
            vec![("a", 0), ("b", 1), ("c", 2), ("d", 1), ("e", 0)]
        );
    }

    #[test]
    fn marker_and_indent_together() {
        let lines = split("- [ ] groceries\n  - [x] eggs\n    - milk");
        assert_eq!(
            texts(&lines),
            vec![("groceries", 0), ("eggs", 1), ("milk", 2)]
        );
    }
}
