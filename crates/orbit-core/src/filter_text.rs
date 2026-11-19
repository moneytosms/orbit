//! Typed filter syntax: text <-> the filter tree (`view::Node`) the visual editor edits.
//!
//! Grammar (keywords are case-insensitive, names are matched case-insensitively):
//! - `&` and, `|` or (and binds tighter), `!` not, `( )` groups, at most `view::MAX_DEPTH` deep.
//!   The whole text is a group: `a & b` is an all-group, `a | b` an any-group, a lone `a`
//!   an all-group of one. Each `( )` is a nested group.
//! - `!` flips `is`/`is not`, and De Morgan's over a group; other conditions can't be negated.
//! - `p1`..`p4` priority; `#Label` (`#"Deep work"`); `$Goal` (goal name); `completed`, `open`.
//! - `project: Name`, `area: Name`, `goal: Name` (`##Project` is not used); `search: text` or
//!   bare words: title contains. Quote names with specials: `project: "A & B"`.
//! - `due: D`, `due before: D`, `due after: D`, `due within: D to D`; the same for `deadline`,
//!   `created`, `touched`. `no due`, `no label`, ... : is empty.
//! - `where <path> <op> <value>`: any condition at all (the fallback the printer uses for what
//!   has no sugar). Ops are the snake_case names; strings are quoted; `true`/`false`/`null`/numbers
//!   are bare. `within` takes `<from> to <to>`.
//! - Dates stay relative, as view.rs stores them: `today`, `tomorrow`, `yesterday`, `+3d`, `-2w`,
//!   `YYYY-MM-DD`. `next week` is `+1w` (a week from today, not quick-add's next Monday),
//!   `in 3 days` is `+3d`, a weekday is `+Nd` to its next occurrence, `dec 5` an absolute date.
//!   Printing uses `next week` for `+1w`; other values print as stored.
//!
//! `@Place` is not supported: it can't tell an Area from a Project. Labels are stored by id, so
//! parsing needs `Known`; projects, areas and goals are filtered by name and need no lookup.
use crate::parse::{date_phrase, fmt_date, parse_ymd, priority_word, weekday_name};
use crate::view::{CondOp, MAX_DEPTH, Node, depth, resolve_date};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Named {
    pub id: String,
    pub name: String,
}

/// What names resolve to ids.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Known {
    #[serde(default)]
    pub labels: Vec<Named>,
}

/// A parse error. `start..end` are char offsets of the bad token.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FilterError {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

type Span = (usize, usize);

fn err<T>(span: Span, message: &str) -> Result<T, FilterError> {
    Err(FilterError {
        start: span.0,
        end: span.1,
        message: message.into(),
    })
}

// ---- Lexing ----

struct Word {
    text: String,
    start: usize,
    end: usize,
    /// The word began with a quote: never a keyword.
    lead_quote: bool,
    /// Some part was quoted.
    quoted: bool,
}

enum Tok {
    Open(Span),
    Close(Span),
    And(Span),
    Or(Span),
    Not(Span),
    Word(Word),
}

fn lex(input: &str) -> Result<Vec<Tok>, FilterError> {
    let cs: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let span = (i, i + 1);
        let op = match c {
            '(' => Some(Tok::Open(span)),
            ')' => Some(Tok::Close(span)),
            '&' => Some(Tok::And(span)),
            '|' => Some(Tok::Or(span)),
            '!' => Some(Tok::Not(span)),
            _ => None,
        };
        if let Some(t) = op {
            out.push(t);
            i += 1;
            continue;
        }
        let start = i;
        let mut text = String::new();
        let mut quoted = false;
        while i < cs.len() {
            let c = cs[i];
            if c.is_whitespace() || "()&|".contains(c) {
                break;
            }
            if c == '"' {
                quoted = true;
                i += 1;
                let mut closed = false;
                while i < cs.len() {
                    let d = cs[i];
                    i += 1;
                    match d {
                        '\\' if i < cs.len() => {
                            text.push(cs[i]);
                            i += 1;
                        }
                        '"' => {
                            closed = true;
                            break;
                        }
                        _ => text.push(d),
                    }
                }
                if !closed {
                    return err((start, cs.len()), "unterminated quote");
                }
                continue;
            }
            text.push(c);
            i += 1;
        }
        out.push(Tok::Word(Word {
            text,
            start,
            end: i,
            lead_quote: cs[start] == '"',
            quoted,
        }));
    }
    Ok(out)
}

fn tok_span(t: &Tok) -> Span {
    match t {
        Tok::Open(s) | Tok::Close(s) | Tok::And(s) | Tok::Or(s) | Tok::Not(s) => *s,
        Tok::Word(w) => (w.start, w.end),
    }
}

// ---- Parsing ----

const DATE_FIELDS: [(&str, &str); 4] = [
    ("due", "due"),
    ("deadline", "deadline"),
    ("created", "created_at"),
    ("touched", "touched_at"),
];
const PLACE_FIELDS: [(&str, &str); 3] = [
    ("project", "project.name"),
    ("area", "area.name"),
    ("goal", "goal.name"),
];
const NO_FIELDS: [(&str, &str); 11] = [
    ("due", "due"),
    ("deadline", "deadline"),
    ("label", "label"),
    ("created", "created_at"),
    ("touched", "touched_at"),
    ("title", "title"),
    ("priority", "priority"),
    ("project", "project.name"),
    ("area", "area.name"),
    ("goal", "goal.name"),
    ("parent", "parent"),
];
const OPS: [(&str, CondOp); 7] = [
    ("is", CondOp::Is),
    ("is_not", CondOp::IsNot),
    ("contains", CondOp::Contains),
    ("is_empty", CondOp::IsEmpty),
    ("before", CondOp::Before),
    ("after", CondOp::After),
    ("within", CondOp::Within),
];

fn cond(path: &str, op: CondOp, value: Value) -> Node {
    Node::Cond {
        path: path.into(),
        op,
        value,
    }
}

fn group(all: bool, children: Vec<Node>) -> Node {
    Node::Group { all, children }
}

fn lookup<'a>(table: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, p)| *p)
}

fn wspan(w: &Word) -> Span {
    (w.start, w.end)
}

fn negate(n: Node, span: Span) -> Result<Node, FilterError> {
    match n {
        Node::Group { all, children } if !children.is_empty() => Ok(group(
            !all,
            children
                .into_iter()
                .map(|c| negate(c, span))
                .collect::<Result<_, _>>()?,
        )),
        Node::Cond {
            path,
            op: CondOp::Is,
            value,
        } => Ok(cond(&path, CondOp::IsNot, value)),
        Node::Cond {
            path,
            op: CondOp::IsNot,
            value,
        } => Ok(cond(&path, CondOp::Is, value)),
        _ => err(span, "this can't be negated"),
    }
}

struct P<'a> {
    toks: &'a [Tok],
    pos: usize,
    known: &'a Known,
    today: i64,
}

impl P<'_> {
    fn expr(&mut self, open: Option<Span>) -> Result<Node, FilterError> {
        let first = self.toks.get(self.pos).map_or((0, 0), tok_span);
        let blame = open.unwrap_or(first);
        if matches!(self.toks.get(self.pos), None | Some(Tok::Close(_))) {
            return Ok(group(true, vec![]));
        }
        let mut alts: Vec<Vec<Node>> = Vec::new();
        let mut cur = Vec::new();
        let mut ctx = blame;
        loop {
            cur.push(self.term(ctx)?);
            match self.toks.get(self.pos) {
                Some(Tok::And(s)) => {
                    ctx = *s;
                    self.pos += 1;
                }
                Some(Tok::Or(s)) => {
                    ctx = *s;
                    self.pos += 1;
                    alts.push(std::mem::take(&mut cur));
                }
                _ => break,
            }
        }
        alts.push(cur);
        let node = if alts.len() == 1 {
            group(true, alts.pop().unwrap_or_default())
        } else {
            group(
                false,
                alts.into_iter()
                    .map(|mut a| {
                        if a.len() == 1 {
                            a.remove(0)
                        } else {
                            group(true, a)
                        }
                    })
                    .collect(),
            )
        };
        if depth(&node) > MAX_DEPTH {
            return err(blame, "groups nest too deep");
        }
        Ok(node)
    }

    fn term(&mut self, ctx: Span) -> Result<Node, FilterError> {
        match self.toks.get(self.pos) {
            Some(Tok::Not(s)) => {
                self.pos += 1;
                let inner = self.term(*s)?;
                negate(inner, *s)
            }
            Some(Tok::Open(s)) => {
                self.pos += 1;
                let g = self.expr(Some(*s))?;
                if let Some(Tok::Close(_)) = self.toks.get(self.pos) {
                    self.pos += 1;
                    Ok(g)
                } else {
                    err(*s, "unclosed parenthesis")
                }
            }
            Some(Tok::Word(_)) => {
                let mut ws: Vec<&Word> = Vec::new();
                while let Some(Tok::Word(w)) = self.toks.get(self.pos) {
                    ws.push(w);
                    self.pos += 1;
                }
                self.atom(&ws)
            }
            _ => err(ctx, "expected a filter"),
        }
    }

    fn date(&self, ws: &[&Word], i: usize) -> Result<(String, usize), FilterError> {
        let Some(w) = ws.get(i) else {
            return err(wspan(ws[i - 1]), "expected a date");
        };
        let lower: Vec<String> = ws[i..].iter().map(|w| w.text.to_lowercase()).collect();
        let l0 = lower[0].as_str();
        if resolve_date(l0, 0).is_some() {
            return Ok((l0.into(), 1));
        }
        if l0 == "next" && lower.get(1).map(String::as_str) == Some("week") {
            return Ok(("+1w".into(), 2));
        }
        if let Some((day, used)) = date_phrase(&lower, self.today) {
            let value = if l0 == "in" {
                let unit = if lower[2].starts_with('w') { 'w' } else { 'd' };
                format!("+{}{unit}", lower[1])
            } else if weekday_name(l0).is_some() {
                format!("+{}d", day - self.today)
            } else {
                fmt_date(day)
            };
            return Ok((value, used));
        }
        err(wspan(w), "not a date")
    }

    fn only(&self, ws: &[&Word], n: usize) -> Result<(), FilterError> {
        match ws.get(n) {
            Some(w) => err(wspan(w), "unexpected word"),
            None => Ok(()),
        }
    }

    fn label(&self, name: &str, span: Span) -> Result<Node, FilterError> {
        let lower = name.to_lowercase();
        let id = self
            .known
            .labels
            .iter()
            .find(|l| l.name.to_lowercase() == lower)
            .map(|l| l.id.clone());
        match id {
            Some(id) => Ok(cond("label", CondOp::Is, json!(id))),
            None => err(span, "unknown label"),
        }
    }

    fn atom(&self, ws: &[&Word]) -> Result<Node, FilterError> {
        let w0 = ws[0];
        let search = |text: String| cond("title", CondOp::Contains, json!(text));
        if w0.lead_quote {
            return Ok(search(join(ws, None)));
        }
        let l0 = w0.text.to_lowercase();
        if let Some(n) = priority_word(&l0) {
            self.only(ws, 1)?;
            return Ok(cond("priority", CondOp::Is, json!(n)));
        }
        match l0.as_str() {
            "completed" => {
                self.only(ws, 1)?;
                return Ok(cond("completed", CondOp::Is, json!(true)));
            }
            "open" => {
                self.only(ws, 1)?;
                return Ok(cond("completed", CondOp::Is, json!(false)));
            }
            _ => {}
        }
        let sigil = w0.text.chars().next();
        if matches!(sigil, Some('#' | '$')) && w0.text.chars().count() > 1 {
            self.only(ws, 1)?;
            let name: String = w0.text.chars().skip(1).collect();
            return if sigil == Some('#') {
                self.label(&name, wspan(w0))
            } else {
                Ok(cond("goal.name", CondOp::Is, json!(name)))
            };
        }
        if let Some((key, after)) = l0.split_once(':') {
            let glued: String = w0.text.chars().skip(key.chars().count() + 1).collect();
            let _ = after;
            let rest = join(ws, Some(glued));
            if key == "search" || lookup(&PLACE_FIELDS, key).is_some() {
                if rest.is_empty() {
                    return err(wspan(w0), "expected a name");
                }
                return Ok(match lookup(&PLACE_FIELDS, key) {
                    Some(path) => cond(path, CondOp::Is, json!(rest)),
                    None => search(rest),
                });
            }
        }
        if l0 == "no" && ws.len() >= 2 {
            let Some(path) = lookup(&NO_FIELDS, &ws[1].text.to_lowercase()) else {
                return err(wspan(ws[1]), "not a field");
            };
            self.only(ws, 2)?;
            return Ok(cond(path, CondOp::IsEmpty, Value::Null));
        }
        if l0 == "where" && ws.len() >= 2 {
            return self.generic(ws);
        }
        let field = l0.strip_suffix(':').unwrap_or(&l0);
        if let Some(path) = lookup(&DATE_FIELDS, field)
            && (l0.ends_with(':') || ws.len() >= 2)
        {
            return self.date_atom(ws, path, l0.ends_with(':'));
        }
        Ok(search(join(ws, None)))
    }

    fn date_atom(&self, ws: &[&Word], path: &str, colon: bool) -> Result<Node, FilterError> {
        let (op, at) = if colon {
            (CondOp::Is, 1)
        } else {
            let op = match ws[1].text.to_lowercase().as_str() {
                "before:" => CondOp::Before,
                "after:" => CondOp::After,
                "within:" => CondOp::Within,
                _ => return err(wspan(ws[1]), "expected before:, after: or within:"),
            };
            (op, 2)
        };
        let (a, n) = self.date(ws, at)?;
        if op != CondOp::Within {
            self.only(ws, at + n)?;
            return Ok(cond(path, op, json!(a)));
        }
        let to = at + n;
        match ws.get(to) {
            Some(w) if w.text.eq_ignore_ascii_case("to") => {}
            Some(w) => return err(wspan(w), "expected 'to'"),
            None => return err(wspan(ws[to - 1]), "expected 'to' and a date"),
        }
        let (b, m) = self.date(ws, to + 1)?;
        self.only(ws, to + 1 + m)?;
        Ok(cond(path, op, json!({ "from": a, "to": b })))
    }

    fn generic(&self, ws: &[&Word]) -> Result<Node, FilterError> {
        let Some(opw) = ws.get(2) else {
            return err(wspan(ws[1]), "expected an operator");
        };
        let Some((_, op)) = OPS.iter().find(|(n, _)| *n == opw.text.to_lowercase()) else {
            return err(wspan(opw), "not an operator");
        };
        let path = ws[1].text.as_str();
        let need = |i: usize| {
            ws.get(i).copied().ok_or_else(|| FilterError {
                start: opw.start,
                end: opw.end,
                message: "expected a value".into(),
            })
        };
        let value = match op {
            CondOp::IsEmpty => {
                self.only(ws, 3)?;
                Value::Null
            }
            CondOp::Within => {
                let (a, b) = (need(3)?, need(5)?);
                match ws.get(4) {
                    Some(t) if t.text.eq_ignore_ascii_case("to") => {}
                    Some(t) => return err(wspan(t), "expected 'to'"),
                    None => return err(wspan(a), "expected 'to' and a value"),
                }
                self.only(ws, 6)?;
                json!({ "from": a.text, "to": b.text })
            }
            _ => {
                let v = need(3)?;
                self.only(ws, 4)?;
                scalar(v)
            }
        };
        Ok(cond(path, *op, value))
    }
}

fn scalar(w: &Word) -> Value {
    if w.quoted {
        return json!(w.text);
    }
    match w.text.as_str() {
        "true" => json!(true),
        "false" => json!(false),
        "null" => Value::Null,
        t => t
            .parse::<i64>()
            .map(|n| json!(n))
            .or_else(|_| t.parse::<f64>().map(|n| json!(n)))
            .unwrap_or_else(|_| json!(t)),
    }
}

fn join(ws: &[&Word], glued: Option<String>) -> String {
    let mut parts: Vec<&str> = Vec::new();
    match &glued {
        Some(g) => {
            if !g.is_empty() {
                parts.push(g);
            }
            parts.extend(ws[1..].iter().map(|w| w.text.as_str()));
        }
        None => parts.extend(ws.iter().map(|w| w.text.as_str())),
    }
    parts.join(" ")
}

/// Parses typed filter `text` to a filter tree. `today` is the caller's local `YYYY-MM-DD`.
pub fn parse(text: &str, known: &Known, today: &str) -> Result<Node, FilterError> {
    let Some(today) = parse_ymd(today) else {
        return err((0, 0), "bad date for today");
    };
    let toks = lex(text)?;
    let mut p = P {
        toks: &toks,
        pos: 0,
        known,
        today,
    };
    let node = p.expr(None)?;
    match toks.get(p.pos) {
        None => Ok(node),
        Some(t) => err(tok_span(t), "unexpected )"),
    }
}

// ---- Printing ----

/// Text safe to write unquoted: single-spaced words free of operators, quotes and a leading `!`.
fn plain(s: &str) -> bool {
    !s.is_empty()
        && s.split(' ').all(|w| {
            !w.is_empty()
                && !w.starts_with('!')
                && !w
                    .chars()
                    .any(|c| c.is_whitespace() || "()&|\"\\".contains(c))
        })
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn text(s: &str) -> String {
    if plain(s) { s.into() } else { quote(s) }
}

/// `#name` / `$name`: one word, quoted after the sigil when it has specials.
fn sigil(c: char, name: &str) -> String {
    if plain(name) && !name.contains(' ') {
        format!("{c}{name}")
    } else {
        format!("{c}{}", quote(name))
    }
}

fn date_text(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    resolve_date(s, 0)?;
    Some(if s == "+1w" {
        "next week".into()
    } else {
        s.into()
    })
}

fn sugar(path: &str, op: CondOp, v: &Value, known: &Known) -> Option<String> {
    let bang = if op == CondOp::IsNot { "!" } else { "" };
    let is = matches!(op, CondOp::Is | CondOp::IsNot);
    if op == CondOp::IsEmpty {
        let word = NO_FIELDS.iter().find(|(_, p)| *p == path)?.0;
        return Some(format!("no {word}"));
    }
    match path {
        "priority" if is => {
            let n = v.as_u64().filter(|n| (1..=4).contains(n))?;
            Some(format!("{bang}p{n}"))
        }
        "label" if is => {
            let id = v.as_str()?;
            let name = known.labels.iter().find(|l| l.id == id)?.name.as_str();
            Some(format!("{bang}{}", sigil('#', name)))
        }
        "completed" if is => {
            let word = if v.as_bool()? { "completed" } else { "open" };
            Some(format!("{bang}{word}"))
        }
        "project.name" | "area.name" | "goal.name" if is => {
            let s = v.as_str().filter(|s| !s.is_empty())?;
            let key = PLACE_FIELDS.iter().find(|(_, p)| *p == path)?.0;
            Some(format!("{bang}{key}: {}", text(s)))
        }
        "title" if op == CondOp::Contains => {
            let s = v.as_str().filter(|s| !s.is_empty())?;
            Some(format!("search: {}", text(s)))
        }
        _ => {
            let key = DATE_FIELDS.iter().find(|(_, p)| *p == path)?.0;
            match op {
                CondOp::Before => Some(format!("{key} before: {}", date_text(v)?)),
                CondOp::After => Some(format!("{key} after: {}", date_text(v)?)),
                CondOp::Is | CondOp::IsNot => Some(format!("{bang}{key}: {}", date_text(v)?)),
                CondOp::Within => Some(format!(
                    "{key} within: {} to {}",
                    date_text(v.get("from")?)?,
                    date_text(v.get("to")?)?
                )),
                _ => None,
            }
        }
    }
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => quote(s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".into(),
        other => quote(&other.to_string()),
    }
}

fn generic(path: &str, op: CondOp, v: &Value) -> String {
    let name = OPS.iter().find(|(_, o)| *o == op).map_or("is", |(n, _)| n);
    let path = if path.is_empty()
        || path
            .chars()
            .any(|c| c.is_whitespace() || "()&|\"\\".contains(c))
    {
        quote(path)
    } else {
        path.into()
    };
    let value = match op {
        CondOp::IsEmpty => String::new(),
        CondOp::Within => {
            let part = |k: &str| value_text(v.get(k).unwrap_or(&Value::Null));
            format!(" {} to {}", part("from"), part("to"))
        }
        _ => format!(" {}", value_text(v)),
    };
    format!("where {path} {name}{value}")
}

fn atom(path: &str, op: CondOp, v: &Value, known: &Known) -> String {
    sugar(path, op, v, known).unwrap_or_else(|| generic(path, op, v))
}

fn inner(children: &[Node], all: bool, known: &Known) -> String {
    children
        .iter()
        .map(|c| match c {
            Node::Group { all, children } => format!("({})", inner(children, *all, known)),
            Node::Cond { path, op, value } => atom(path, *op, value, known),
        })
        .collect::<Vec<_>>()
        .join(if all { " & " } else { " | " })
}

/// Prints a filter tree as typed filter text that `parse` reads back.
pub fn print(node: &Node, known: &Known) -> String {
    match node {
        Node::Group { all, children } => inner(children, *all, known),
        Node::Cond { path, op, value } => atom(path, *op, value, known),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-11-20";

    fn known() -> Known {
        let l = |id: &str, name: &str| Named {
            id: id.into(),
            name: name.into(),
        };
        Known {
            labels: vec![
                l("0190b2c0-0000-7000-8000-000000000001", "Career"),
                l("0190b2c0-0000-7000-8000-000000000002", "Deep work"),
                l("0190b2c0-0000-7000-8000-000000000003", "a&b"),
            ],
        }
    }

    fn p(t: &str) -> Node {
        parse(t, &known(), TODAY).unwrap_or_else(|e| panic!("{t}: {e:?}"))
    }

    #[test]
    fn round_trips() {
        let table = [
            "",
            "p1",
            "p1 & #Career & due before: next week",
            "p1 | p2 | p3",
            "p1 & (p2 | p3)",
            "p1 & p2 | p3 & p4",
            "(p1 | p2) & (#Career | #\"Deep work\")",
            "((p1 | p2) & p3) | p4",
            "!p1",
            "!#Career & !completed",
            "!(p1 | #Career)",
            "!(p1 & due: today)",
            "completed",
            "open",
            "!open",
            "()",
            "p1 & ()",
            "#\"a&b\"",
            "due: today",
            "due: tomorrow",
            "due: yesterday",
            "due: +3d",
            "due: -2w",
            "due: 2026-12-01",
            "due before: today",
            "due after: +7d",
            "due within: today to +7d",
            "deadline before: 2026-12-01",
            "deadline after: tomorrow",
            "deadline: today",
            "deadline within: -1w to next week",
            "created after: -8d",
            "created before: today",
            "created within: -7d to today",
            "touched before: -14d",
            "no due",
            "no deadline",
            "no label",
            "no title",
            "no priority",
            "no project",
            "no area",
            "no goal",
            "no created",
            "no touched",
            "no parent",
            "project: Orbit",
            "project: Big plan",
            "project: \"A & B\"",
            "area: Work",
            "goal: Ship v1",
            "$Ship",
            "$\"Ship v1\"",
            "!project: Orbit",
            "!goal: x",
            "search: fix bug",
            "search: \"a | b\"",
            "search: \"!x\"",
            "where title is \"x\"",
            "where priority before 3",
            "where completed is_not true",
            "where due within \"a\" to \"b\"",
            "where due before \"\"",
            "where x is_empty",
            "where label contains \"id\"",
            "where \"my path\" is null",
            "p1 & (p2 | (p3 & p4))",
        ];
        for t in table {
            let n = p(t);
            let s = print(&n, &known());
            let n2 = p(&s);
            assert_eq!(n, n2, "{t} -> {s}");
            assert_eq!(print(&n2, &known()), s, "{t}");
        }
        // Texts already in printed form come back verbatim.
        for t in [
            "p1 & #Career & due before: next week",
            "p1 & (p2 | p3)",
            "(p1 | p2) & (#Career | #\"Deep work\")",
            "!#Career & !completed",
            "due within: today to +7d",
            "no due & project: Big plan & search: fix bug",
            "where priority before 3",
        ] {
            assert_eq!(print(&p(t), &known()), t);
        }
    }

    #[test]
    fn tree_shapes() {
        assert_eq!(p("p1 & p2 | p3").clone(), p("(p1 & p2) | p3"));
        assert_eq!(
            p("p1 | p2"),
            group(
                false,
                vec![
                    cond("priority", CondOp::Is, json!(1)),
                    cond("priority", CondOp::Is, json!(2)),
                ]
            )
        );
        assert_eq!(
            p("!(p1 | completed)"),
            group(
                true,
                vec![group(
                    true,
                    vec![
                        cond("priority", CondOp::IsNot, json!(1)),
                        cond("completed", CondOp::IsNot, json!(true)),
                    ]
                )]
            )
        );
        assert_eq!(
            p("#career"),
            group(
                true,
                vec![cond(
                    "label",
                    CondOp::Is,
                    json!("0190b2c0-0000-7000-8000-000000000001")
                )]
            )
        );
    }

    #[test]
    fn dates_stay_relative() {
        let due = |t: &str| match p(t) {
            Node::Group { children, .. } => children[0].clone(),
            n => n,
        };
        let v = |t: &str, val: Value| cond("due", CondOp::Before, val.clone()) == due(t);
        assert!(v("due before: next week", json!("+1w")));
        assert!(v("due before: in 3 days", json!("+3d")));
        assert!(v("due before: in 2 weeks", json!("+2w")));
        assert!(v("due before: today", json!("today")));
        // 2026-11-20 is a Friday: Monday is 3 days on, Friday itself a week on.
        assert!(v("due before: monday", json!("+3d")));
        assert!(v("due before: friday", json!("+7d")));
        assert!(v("due before: dec 5", json!("2026-12-05")));
    }

    #[test]
    fn built_trees_print_parseable_text() {
        let tree = group(
            false,
            vec![
                cond("title", CondOp::Is, json!("")),
                cond("priority", CondOp::Is, Value::Null),
                cond("priority", CondOp::Is, json!(2)),
                cond("completed", CondOp::Is, json!(false)),
                cond("completed", CondOp::IsNot, json!(false)),
                cond("label", CondOp::Is, json!("")),
                cond(
                    "label",
                    CondOp::IsNot,
                    json!("0190b2c0-0000-7000-8000-000000000002"),
                ),
                cond(
                    "label",
                    CondOp::Is,
                    json!("0190b2c0-0000-7000-8000-0000000000ff"),
                ),
                cond("due", CondOp::Within, json!({"from": "", "to": ""})),
                cond("due", CondOp::Within, json!({"from": "today", "to": "+7d"})),
                cond("due", CondOp::Is, json!("whenever")),
                cond("deadline", CondOp::IsEmpty, json!("")),
                cond("project.name", CondOp::Contains, json!("x y")),
                cond("area.name", CondOp::Is, json!("")),
                group(
                    true,
                    vec![cond("goal.name", CondOp::Is, json!("Ship \"it\""))],
                ),
                cond("title", CondOp::Contains, json!("a (b) & c | !d")),
                cond("title", CondOp::Contains, json!("two  spaces")),
            ],
        );
        let k = known();
        let s = print(&tree, &k);
        let back = parse(&s, &k, TODAY).unwrap_or_else(|e| panic!("{s}: {e:?}"));
        let Node::Group { children, .. } = &back else {
            panic!()
        };
        let Node::Group { children: orig, .. } = &tree else {
            panic!()
        };
        assert_eq!(children.len(), orig.len(), "{s}");
        // Everything but the is_empty value (null) and the unknown-id label survives exactly.
        for (i, (a, b)) in orig.iter().zip(children).enumerate() {
            if i == 7 {
                continue;
            }
            if let Node::Cond {
                op: CondOp::IsEmpty,
                path,
                ..
            } = a
            {
                assert_eq!(b, &cond(path, CondOp::IsEmpty, Value::Null));
                continue;
            }
            assert_eq!(a, b, "{s}");
        }
    }

    #[test]
    fn errors_point_at_the_bad_token() {
        let e = |t: &str| parse(t, &known(), TODAY).unwrap_err();
        let at = |t: &str, bad: &str| {
            let err = e(t);
            let got: String = t
                .chars()
                .skip(err.start)
                .take(err.end - err.start)
                .collect();
            assert_eq!(got, bad, "{t}: {}", err.message);
        };
        at("p1 & #Nope", "#Nope");
        at("p1 & & p2", "&");
        at("p1 &", "&");
        at("p1 & (p2", "(");
        at("p1 )", ")");
        at("due before: someday", "someday");
        at("due before: today extra", "extra");
        at("due sometime: today", "sometime:");
        at("due within: today", "today");
        at("due within: today 5", "5");
        at("no flavour", "flavour");
        at("p1 extra", "extra");
        at("project:", "project:");
        at("search: \"open", "\"open");
        at("!due before: today", "!");
        at("where x nope 1", "nope");
        at("(((p1 & (p2 | p3))))", "(");
        let ok = p("((p1 & p2))");
        assert_eq!(depth(&ok), 3);
        assert!(parse("p1", &known(), "nope").is_err());
        assert_eq!(e("p1 & #Nope").message, "unknown label");
    }
}
