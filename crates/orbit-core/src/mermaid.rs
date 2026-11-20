//! Mermaid import: a gantt or flowchart becomes a plan of Phases and items.
//!
//! Only the subset that maps onto a Roadmap is understood; styling and layout directives are
//! skipped. Errors carry the 1-based line they were found on.
use crate::parse::{civil_from_days, days_from_civil, days_in_month};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ParseError {
    /// 1-based line in the pasted text.
    pub line: usize,
    pub message: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PlanItem {
    /// The gantt task id or flowchart node id.
    pub key: String,
    pub title: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub milestone: bool,
    pub done: bool,
    /// Keys of the items this one points at.
    pub links: Vec<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PlanPhase {
    pub name: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub items: Vec<PlanItem>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Plan {
    pub title: Option<String>,
    pub phases: Vec<PlanPhase>,
}

fn err<T>(line: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError {
        line,
        message: message.into(),
    })
}

/// Parses a Mermaid `gantt`, `flowchart` or `graph` diagram.
pub fn parse(text: &str) -> Result<Plan, ParseError> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("%%"))
        .collect();
    let Some(&(n, head)) = lines.first() else {
        return err(1, "Nothing to import");
    };
    let word = head.split_whitespace().next().unwrap_or("");
    match word {
        "gantt" => gantt(&lines[1..]),
        "flowchart" | "graph" => flowchart(n, head, &lines[1..]),
        _ => err(
            n,
            format!("Unknown diagram type `{word}`; use gantt or flowchart"),
        ),
    }
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if !(digits(0..4) && digits(5..7) && digits(8..10)) {
        return false;
    }
    let (y, m, d): (i64, i64, i64) = (
        s[0..4].parse().unwrap_or(0),
        s[5..7].parse().unwrap_or(0),
        s[8..10].parse().unwrap_or(0),
    );
    (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m)
}

fn day_of(s: &str) -> i64 {
    days_from_civil(
        s[0..4].parse().unwrap_or(0),
        s[5..7].parse().unwrap_or(0),
        s[8..10].parse().unwrap_or(0),
    )
}

fn add_days(s: &str, n: i64) -> String {
    let (y, m, d) = civil_from_days(day_of(s) + n);
    format!("{y:04}-{m:02}-{d:02}")
}

/// `3d` or `2w` as a number of days.
fn duration(s: &str) -> Option<i64> {
    let (n, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: i64 = n.parse().ok().filter(|n| (0..=36_500).contains(n))?;
    match unit {
        "d" => Some(n),
        "w" => Some(n * 7),
        _ => None,
    }
}

fn gantt(lines: &[(usize, &str)]) -> Result<Plan, ParseError> {
    const IGNORED: [&str; 11] = [
        "axisFormat",
        "excludes",
        "includes",
        "todayMarker",
        "tickInterval",
        "weekday",
        "weekend",
        "accTitle",
        "accDescr",
        "click",
        "inclusiveEndDates",
    ];
    let mut title = None;
    let mut phases: Vec<PlanPhase> = vec![];
    // key -> (phase index, item index)
    let mut keys: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut deps: Vec<(String, String)> = vec![];
    let mut last_end: Option<String> = None;
    let mut count = 0;
    for &(n, line) in lines {
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        match word {
            "title" => {
                title = Some(rest.to_string()).filter(|t| !t.is_empty());
                continue;
            }
            "dateFormat" => {
                if rest != "YYYY-MM-DD" {
                    return err(
                        n,
                        format!("Unsupported dateFormat `{rest}`; use YYYY-MM-DD"),
                    );
                }
                continue;
            }
            "section" => {
                if rest.is_empty() {
                    return err(n, "A section needs a name");
                }
                phases.push(PlanPhase {
                    name: rest.into(),
                    start: None,
                    end: None,
                    items: vec![],
                });
                continue;
            }
            w if IGNORED.contains(&w.trim_end_matches(':')) => continue,
            _ => {}
        }
        let Some((name, spec)) = line.split_once(':') else {
            return err(n, format!("Expected `name : start, end`, found `{line}`"));
        };
        let name = name.trim();
        if name.is_empty() {
            return err(n, "A task needs a name");
        }
        let mut parts: Vec<&str> = spec.split(',').map(str::trim).collect();
        let (mut milestone, mut done) = (false, false);
        while let Some(&tag) = parts.first() {
            match tag {
                "crit" | "active" => {}
                "done" => done = true,
                "milestone" => milestone = true,
                _ => break,
            }
            parts.remove(0);
        }
        count += 1;
        let (key, start_s, end_s) = match parts.as_slice() {
            [id, s, e] => (Some(*id), *s, *e),
            [s, e] => (None, *s, *e),
            [e] => (None, "", *e),
            _ => return err(n, "Expected `[id,] start, end` after the name"),
        };
        if key.is_some_and(|k| k.is_empty() || k.contains(char::is_whitespace)) {
            return err(n, "A task id can't be empty or contain spaces");
        }
        let key = key.map_or_else(|| format!("task{count}"), str::to_string);
        if keys.contains_key(&key) {
            return err(n, format!("Duplicate task id `{key}`"));
        }
        let mut after = vec![];
        let start = if start_s.is_empty() {
            match &last_end {
                Some(d) => d.clone(),
                None => return err(n, "No start date: give one or follow another task"),
            }
        } else if let Some(ids) = start_s.strip_prefix("after ") {
            let mut latest: Option<String> = None;
            for id in ids.split_whitespace() {
                let Some(&(p, i)) = keys.get(id) else {
                    return err(n, format!("Unknown task id `{id}` in `after`"));
                };
                let end = phases[p].items[i].end.clone().unwrap_or_default();
                if latest.as_ref().is_none_or(|l| end > *l) {
                    latest = Some(end);
                }
                after.push(id.to_string());
            }
            match latest {
                Some(d) => d,
                None => return err(n, "`after` needs a task id"),
            }
        } else if valid_date(start_s) {
            start_s.to_string()
        } else {
            return err(n, format!("Bad start date `{start_s}`; use YYYY-MM-DD"));
        };
        let end = if valid_date(end_s) {
            end_s.to_string()
        } else if let Some(days) = duration(end_s) {
            add_days(&start, days)
        } else {
            return err(
                n,
                format!("Bad end `{end_s}`; use YYYY-MM-DD or a length like 5d"),
            );
        };
        if end < start {
            return err(n, format!("End {end} is before start {start}"));
        }
        let end = if milestone { start.clone() } else { end };
        if phases.is_empty() {
            phases.push(PlanPhase {
                name: "Unsorted".into(),
                start: None,
                end: None,
                items: vec![],
            });
        }
        let p = phases.len() - 1;
        for a in after {
            deps.push((a, key.clone()));
        }
        last_end = Some(end.clone());
        keys.insert(key.clone(), (p, phases[p].items.len()));
        phases[p].items.push(PlanItem {
            key,
            title: name.into(),
            start: Some(start),
            end: Some(end),
            milestone,
            done,
            links: vec![],
        });
    }
    for (from, to) in deps {
        let (p, i) = keys[&from];
        phases[p].items[i].links.push(to);
    }
    for p in &mut phases {
        p.start = p.items.iter().filter_map(|i| i.start.clone()).min();
        p.end = p.items.iter().filter_map(|i| i.end.clone()).max();
    }
    phases.retain(|p| !p.items.is_empty());
    Ok(Plan { title, phases })
}

struct Node {
    title: Option<String>,
    milestone: bool,
    phase: Option<usize>,
    links: Vec<String>,
}

fn flowchart(n: usize, head: &str, lines: &[(usize, &str)]) -> Result<Plan, ParseError> {
    let dir = head
        .split_whitespace()
        .nth(1)
        .map(|d| d.trim_end_matches(';'));
    if let Some(d) = dir
        && !["TD", "TB", "BT", "RL", "LR"].contains(&d)
    {
        return err(
            n,
            format!("Unknown direction `{d}`; use TD, TB, BT, RL or LR"),
        );
    }
    let mut names: Vec<String> = vec![];
    let mut stack: Vec<(usize, usize)> = vec![]; // (phase index, line opened)
    let mut nodes: Vec<(String, Node)> = vec![];
    for &(n, line) in lines {
        let line = line.trim_end_matches(';').trim();
        let word = line.split_whitespace().next().unwrap_or("");
        match word {
            "subgraph" => {
                let name = line["subgraph".len()..].trim();
                let name = match (name.find('['), name.rfind(']')) {
                    (Some(a), Some(b)) if a < b => &name[a + 1..b],
                    _ => name,
                };
                let name = name.trim_matches('"').trim();
                if name.is_empty() {
                    return err(n, "A subgraph needs a name");
                }
                names.push(name.into());
                stack.push((names.len() - 1, n));
                continue;
            }
            "end" if line == "end" => {
                if stack.pop().is_none() {
                    return err(n, "`end` without a matching `subgraph`");
                }
                continue;
            }
            "direction" | "classDef" | "class" | "style" | "linkStyle" | "click" => continue,
            _ => {}
        }
        let phase = stack.last().map(|s| s.0);
        let mut s = Scan { s: line, n };
        let mut prev: Option<String> = None;
        loop {
            let (key, title, milestone) = s.node()?;
            let at = match nodes.iter().position(|(k, _)| *k == key) {
                Some(at) => at,
                None => {
                    nodes.push((
                        key.clone(),
                        Node {
                            title: None,
                            milestone: false,
                            phase,
                            links: vec![],
                        },
                    ));
                    nodes.len() - 1
                }
            };
            if title.is_some() {
                nodes[at].1.title = title;
                nodes[at].1.milestone = milestone;
            }
            if let Some(from) = prev.take()
                && from != key
            {
                let f = nodes.iter().position(|(k, _)| *k == from).unwrap_or(at);
                if !nodes[f].1.links.contains(&key) {
                    nodes[f].1.links.push(key.clone());
                }
            }
            s.ws();
            if s.s.is_empty() {
                break;
            }
            s.edge()?;
            prev = Some(key);
        }
    }
    if let Some(&(i, line)) = stack.last() {
        return err(line, format!("Unclosed subgraph `{}`", names[i]));
    }
    let mut phases: Vec<PlanPhase> = names
        .into_iter()
        .map(|name| PlanPhase {
            name,
            start: None,
            end: None,
            items: vec![],
        })
        .collect();
    let mut loose = vec![];
    for (key, node) in nodes {
        let item = PlanItem {
            title: node.title.unwrap_or_else(|| key.clone()),
            key,
            start: None,
            end: None,
            milestone: node.milestone,
            done: false,
            links: node.links,
        };
        match node.phase {
            Some(p) => phases[p].items.push(item),
            None => loose.push(item),
        }
    }
    phases.retain(|p| !p.items.is_empty());
    if !loose.is_empty() {
        phases.push(PlanPhase {
            name: "Unsorted".into(),
            start: None,
            end: None,
            items: loose,
        });
    }
    Ok(Plan {
        title: None,
        phases,
    })
}

/// A cursor over one flowchart statement.
struct Scan<'a> {
    s: &'a str,
    n: usize,
}

impl Scan<'_> {
    fn ws(&mut self) {
        self.s = self.s.trim_start();
    }

    /// `A`, `A[Text]`, `A(Text)` or `A{Text}`: the id, the text if any, and whether it is a diamond.
    fn node(&mut self) -> Result<(String, Option<String>, bool), ParseError> {
        self.ws();
        let end = self
            .s
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(self.s.len());
        if end == 0 {
            return err(
                self.n,
                format!(
                    "Expected a node id, found `{}`",
                    self.s.chars().next().map_or(String::new(), String::from)
                ),
            );
        }
        let key = self.s[..end].to_string();
        self.s = &self.s[end..];
        let (title, milestone) = match self.s.chars().next() {
            Some(open @ ('[' | '(' | '{')) => {
                let mut depth = 0i32;
                let mut quoted = false;
                let mut close = None;
                for (i, c) in self.s.char_indices() {
                    match c {
                        '"' => quoted = !quoted,
                        '[' | '(' | '{' if !quoted => depth += 1,
                        ']' | ')' | '}' if !quoted => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let Some(close) = close else {
                    return err(self.n, format!("Unclosed `{open}` after node `{key}`"));
                };
                let shape = &self.s[..=close];
                let k = shape.chars().take_while(|c| "[({".contains(*c)).count();
                let inner = shape[k..shape.len() - k]
                    .trim_matches(|c| "/\\\"".contains(c))
                    .trim()
                    .to_string();
                self.s = &self.s[close + 1..];
                (Some(inner).filter(|t| !t.is_empty()), open == '{')
            }
            _ => (None, false),
        };
        if let Some(rest) = self.s.strip_prefix(":::") {
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            self.s = &rest[end..];
        }
        Ok((key, title, milestone))
    }

    /// `-->`, `---`, `==>`, `-.->`, optionally `|label|` or `-- label -->`.
    fn edge(&mut self) -> Result<(), ParseError> {
        let end = self
            .s
            .find(|c: char| !"-=.<>".contains(c))
            .unwrap_or(self.s.len());
        let arrow = &self.s[..end];
        if arrow.len() < 2 {
            return err(
                self.n,
                format!("Expected an edge like `-->`, found `{}`", self.s),
            );
        }
        self.s = &self.s[end..];
        if (arrow == "--" || arrow == "==") && self.s.starts_with(' ') {
            let close = ["-->", "---", "==>", "==="]
                .iter()
                .filter_map(|c| self.s.find(c).map(|i| (i, c.len())))
                .min();
            let Some((i, len)) = close else {
                return err(self.n, "Edge text is missing its closing arrow");
            };
            self.s = &self.s[i + len..];
        }
        self.ws();
        if let Some(rest) = self.s.strip_prefix('|') {
            let Some(i) = rest.find('|') else {
                return err(self.n, "Unclosed `|` in an edge label");
            };
            self.s = &rest[i + 1..];
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fail(text: &str) -> (usize, String) {
        let e = parse(text).unwrap_err();
        (e.line, e.message)
    }

    const GANTT: &str = "gantt
    title Launch
    dateFormat YYYY-MM-DD
    axisFormat %m/%d
    %% a comment
    section Build
    Design :done, des, 2026-01-01, 2026-01-10
    Code :active, code, after des, 5d
    Ship :milestone, ship, 2026-01-20, 0d
    section Test
    QA :crit, qa, after code ship, 2026-02-01
";

    #[test]
    fn gantt_sections_become_phases_with_min_max_dates() {
        let plan = parse(GANTT).unwrap();
        assert_eq!(plan.title.as_deref(), Some("Launch"));
        assert_eq!(plan.phases.len(), 2);
        let build = &plan.phases[0];
        assert_eq!(build.name, "Build");
        assert_eq!(build.start.as_deref(), Some("2026-01-01"));
        assert_eq!(build.end.as_deref(), Some("2026-01-20"));
        let [des, code, ship] = &build.items[..] else {
            panic!()
        };
        assert!(des.done && !des.milestone);
        assert_eq!(code.start.as_deref(), Some("2026-01-10"));
        assert_eq!(code.end.as_deref(), Some("2026-01-15"));
        assert!(ship.milestone);
        assert_eq!(ship.start, ship.end);
        // `after` makes a link from the dependency.
        assert_eq!(des.links, vec!["code"]);
        assert_eq!(code.links, vec!["qa"]);
        assert_eq!(ship.links, vec!["qa"]);
        let qa = &plan.phases[1].items[0];
        assert_eq!(qa.title, "QA");
        // `after code ship` starts at the later end.
        assert_eq!(qa.start.as_deref(), Some("2026-01-20"));
    }

    #[test]
    fn gantt_defaults_and_short_forms() {
        let plan = parse("gantt\nA : 2026-03-01, 2d\nB : 3d\nC : 2026-03-10, 2026-03-12").unwrap();
        assert_eq!(plan.phases[0].name, "Unsorted");
        let items = &plan.phases[0].items;
        assert_eq!(items[0].key, "task1");
        assert_eq!(items[0].end.as_deref(), Some("2026-03-03"));
        assert_eq!(items[1].start.as_deref(), Some("2026-03-03"));
        assert_eq!(items[1].end.as_deref(), Some("2026-03-06"));
        assert_eq!(plan.phases[0].end.as_deref(), Some("2026-03-12"));
        // Weeks and month ends.
        let plan = parse("gantt\nA : 2026-01-30, 1w").unwrap();
        assert_eq!(plan.phases[0].items[0].end.as_deref(), Some("2026-02-06"));
    }

    #[test]
    fn gantt_errors_name_the_line() {
        let cases = [
            ("gantt\nA : 2026-13-01, 2d", 2, "Bad start date"),
            ("gantt\nA : 2026-02-30, 2d", 2, "Bad start date"),
            ("gantt\nA : 2026-02-01, soon", 2, "Bad end"),
            ("gantt\nA : 2026-02-01, 2026-01-01", 2, "before start"),
            ("gantt\nA : after nope, 2d", 2, "Unknown task id `nope`"),
            ("gantt\ndateFormat DD/MM/YYYY", 2, "Unsupported dateFormat"),
            ("gantt\n\nsection\n", 3, "needs a name"),
            ("gantt\nno colon here", 2, "Expected `name"),
            ("gantt\nA : 3d", 2, "No start date"),
            ("gantt\nA : a, b, c, d", 2, "Expected `[id,] start, end`"),
            (
                "gantt\nA : x, 2026-01-01, 1d\nB : x, 2026-01-01, 1d",
                3,
                "Duplicate task id",
            ),
            ("gantt\nA : done", 2, "Expected `[id,] start, end`"),
        ];
        for (text, line, msg) in cases {
            let (l, m) = fail(text);
            assert_eq!(l, line, "{text}");
            assert!(m.contains(msg), "{text}: {m}");
        }
    }

    const FLOW: &str = "flowchart TD
    A[Draft] --> B(Review) --> C{Approve?}
    subgraph Build [Build phase]
      direction LR
      D[Code] -->|then| E
      E --- F[Test]
    end
    subgraph Ship
      G
    end
    C -- yes --> D
    classDef x fill:#fff
";

    #[test]
    fn flowchart_subgraphs_become_phases_and_edges_links() {
        let plan = parse(FLOW).unwrap();
        let names: Vec<_> = plan.phases.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Build phase", "Ship", "Unsorted"]);
        let build = &plan.phases[0].items;
        assert_eq!(
            build.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(),
            ["D", "E", "F"]
        );
        assert_eq!(build[0].links, vec!["E"]);
        assert_eq!(build[1].links, vec!["F"]);
        assert_eq!(build[2].title, "Test");
        assert_eq!(build[1].title, "E");
        let loose = &plan.phases[2].items;
        assert_eq!(loose[0].title, "Draft");
        assert_eq!(loose[0].links, vec!["B"]);
        assert_eq!(loose[1].links, vec!["C"]);
        assert!(loose[2].milestone);
        assert_eq!(loose[2].title, "Approve?");
        assert_eq!(loose[2].links, vec!["D"]);
        assert!(loose.iter().all(|i| i.start.is_none() && !i.done));
    }

    #[test]
    fn flowchart_accepts_graph_and_bare_nodes() {
        let plan = parse("graph LR\nA --> B\nA --> B\nA --> A").unwrap();
        let items = &plan.phases[0].items;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].links, vec!["B"]);
        let plan = parse("flowchart\nA[\"Quoted (text)\"]").unwrap();
        assert_eq!(plan.phases[0].items[0].title, "Quoted (text)");
    }

    #[test]
    fn flowchart_errors_name_the_line() {
        let cases = [
            (
                "flowchart TD\nsubgraph One\nA --> B",
                2,
                "Unclosed subgraph `One`",
            ),
            ("flowchart TD\nA --> B\nend", 3, "without a matching"),
            ("flowchart XY\nA", 1, "Unknown direction"),
            ("flowchart TD\nA --> ", 2, "Expected a node id"),
            ("flowchart TD\nA[Open --> B", 2, "Unclosed `[`"),
            ("flowchart TD\nA B", 2, "Expected an edge"),
            ("flowchart TD\nA -->|x B", 2, "Unclosed `|`"),
            ("flowchart TD\nsubgraph\nend", 2, "needs a name"),
        ];
        for (text, line, msg) in cases {
            let (l, m) = fail(text);
            assert_eq!(l, line, "{text}");
            assert!(m.contains(msg), "{text}: {m}");
        }
    }

    #[test]
    fn other_input_is_an_error() {
        assert_eq!(fail("pie\n\"a\": 1").0, 1);
        assert!(
            fail("sequenceDiagram\nA->>B: hi")
                .1
                .contains("Unknown diagram type")
        );
        assert_eq!(fail("%% only a comment\n\n  erDiagram").0, 3);
        assert_eq!(fail("").0, 1);
    }
}
