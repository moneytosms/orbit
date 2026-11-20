//! In-memory search index: fuzzy titles, prefix full text, frecency.
//!
//! Titles use a compact fzf-style scorer written here instead of `nucleo-matcher`: the crate is
//! not in the offline registry, and a ~60-line DP over lowercase chars is all the 16 ms budget
//! needs. Full text is an inverted index (token -> ids) in a `BTreeMap`, so a prefix is a range scan.
//! The index is updated per row (`upsert`/`remove`) and never rebuilt per query.
use crate::{Row, Uuid};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// One search result.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Hit {
    pub id: Uuid,
    pub kind: &'static str,
    pub title: String,
    pub score: f64,
}

struct Entry {
    kind: &'static str,
    title: String,
    lower: Vec<char>,
    parent: Option<Uuid>,
    /// Trashed or archived itself.
    off: bool,
    tokens: Vec<String>,
}

#[derive(Default)]
pub struct Index {
    entries: HashMap<Uuid, Entry>,
    words: BTreeMap<String, BTreeSet<Uuid>>,
    /// id -> (open count, last open unix ms)
    opens: HashMap<Uuid, (u32, u64)>,
}

const NEG: i32 = i32::MIN / 2;
const MAX_TITLE: usize = 64;

fn word_start(prev: Option<char>) -> bool {
    prev.is_none_or(|c| !c.is_alphanumeric())
}

/// fzf-style score of `q` as a subsequence of `t` (both lowercase), or `None`.
fn fuzzy(t: &[char], q: &[char]) -> Option<i32> {
    let n = t.len().min(MAX_TITLE);
    let t = &t[..n];
    let mut j = 0;
    for &c in t {
        if j < q.len() && c == q[j] {
            j += 1;
        }
    }
    if j < q.len() {
        return None;
    }
    // prev[i]: best score with the previous query char at t[i].
    let mut prev = vec![NEG; n];
    let mut cur = vec![NEG; n];
    for (j, &qc) in q.iter().enumerate() {
        let mut gap = NEG;
        for i in 0..n {
            if i >= 2 && j > 0 {
                gap = (gap - 1).max(prev[i - 2]);
            }
            cur[i] = NEG;
            if t[i] != qc {
                continue;
            }
            let bonus = 16
                + if word_start(i.checked_sub(1).map(|k| t[k])) {
                    12
                } else {
                    0
                };
            let from = if j == 0 {
                -(i as i32).min(6)
            } else {
                let adj = if i >= 1 && prev[i - 1] > NEG {
                    prev[i - 1] + 8
                } else {
                    NEG
                };
                adj.max(gap)
            };
            if from > NEG {
                cur[i] = from + bonus;
            }
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev.into_iter().filter(|s| *s > NEG).max()
}

fn tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(str::to_lowercase)
}

/// The `name`s of `[name](blob:…)` / `![name](blob:…)` links in `md`.
fn attachment_names(md: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = md;
    while let Some(at) = rest.find("](blob:") {
        if let Some(open) = rest[..at].rfind('[') {
            out.push(&rest[open + 1..at]);
        }
        rest = &rest[at + 1..];
    }
    out
}

fn row_parts(row: &Row) -> Option<(&'static str, &str, &str)> {
    Some(match row {
        Row::Area(r) => ("area", &r.name, ""),
        Row::Project(r) => ("project", &r.name, ""),
        Row::Section(r) => ("section", &r.name, ""),
        Row::Task(r) => ("task", &r.title, &r.description),
        Row::Goal(r) => ("goal", &r.name, ""),
        Row::Milestone(r) => ("milestone", &r.title, ""),
        Row::Page(r) => ("page", &r.title, &r.body),
        Row::Routine(r) => ("routine", &r.name, ""),
        Row::View(r) => ("view", &r.name, ""),
        _ => return None,
    })
}

impl Index {
    pub fn new() -> Index {
        Index::default()
    }

    /// Adds or replaces the row's entry. Kinds that aren't searchable are ignored.
    pub fn upsert(&mut self, row: &Row) {
        let id = row.id();
        self.remove(id);
        let Some((kind, title, text)) = row_parts(row) else {
            return;
        };
        let mut toks: BTreeSet<String> = tokens(text).chain(tokens(title)).collect();
        for name in attachment_names(text) {
            toks.insert(name.to_lowercase());
            toks.extend(tokens(name));
        }
        for t in &toks {
            self.words.entry(t.clone()).or_default().insert(id);
        }
        self.entries.insert(
            id,
            Entry {
                kind,
                title: title.to_string(),
                lower: title.to_lowercase().chars().collect(),
                parent: row.parent_id(),
                off: row.trashed_at().is_some() || row.archived_at().is_some(),
                tokens: toks.into_iter().collect(),
            },
        );
    }

    pub fn remove(&mut self, id: Uuid) {
        let Some(e) = self.entries.remove(&id) else {
            return;
        };
        for t in e.tokens {
            if let Some(set) = self.words.get_mut(&t) {
                set.remove(&id);
                if set.is_empty() {
                    self.words.remove(&t);
                }
            }
        }
    }

    /// Records that the user opened `id` at `now_ms`.
    pub fn record_open(&mut self, id: Uuid, now_ms: u64) {
        let o = self.opens.entry(id).or_insert((0, 0));
        o.0 += 1;
        o.1 = o.1.max(now_ms);
    }

    /// Restores a saved open record (count, last) for `id`.
    pub fn set_opens(&mut self, id: Uuid, count: u32, last: u64) {
        self.opens.insert(id, (count, last));
    }

    fn frecency(&self, id: Uuid, now_ms: u64) -> f64 {
        let Some(&(count, last)) = self.opens.get(&id) else {
            return 0.0;
        };
        let days = now_ms.saturating_sub(last) as f64 / 86_400_000.0;
        (1.0 + count as f64).ln() * 30.0 * 0.5f64.powf(days / 14.0)
    }

    /// Trashed or archived itself, or under such a row.
    fn hidden(&self, id: Uuid) -> bool {
        let mut at = Some(id);
        for _ in 0..64 {
            let Some(e) = at.and_then(|i| self.entries.get(&i)) else {
                return false;
            };
            if e.off {
                return true;
            }
            at = e.parent;
        }
        false
    }

    /// Best matches for `text`, highest score first. Empty `text` lists recents.
    pub fn query(&self, text: &str, now_ms: u64, limit: usize) -> Vec<Hit> {
        let text = text.trim();
        let mut hits: Vec<Hit> = Vec::new();
        let hit = |id: Uuid, e: &Entry, score: f64| Hit {
            id,
            kind: e.kind,
            title: e.title.clone(),
            score,
        };
        if text.is_empty() {
            for (&id, e) in &self.entries {
                if let Some(&(_, last)) = self.opens.get(&id)
                    && !self.hidden(id)
                {
                    hits.push(hit(id, e, last as f64));
                }
            }
        } else {
            let lc = text.to_lowercase();
            let q: Vec<char> = lc.chars().filter(|c| !c.is_whitespace()).collect();
            let mut scored: HashMap<Uuid, f64> = HashMap::new();
            for (&id, e) in &self.entries {
                if e.off {
                    continue;
                }
                let Some(s) = fuzzy(&e.lower, &q) else {
                    continue;
                };
                let mut s = s as f64;
                let title: String = e.lower.iter().collect();
                if title == lc {
                    s += 1000.0;
                } else if title.starts_with(&lc) {
                    s += 300.0;
                } else if title
                    .match_indices(&lc)
                    .any(|(i, _)| word_start(title[..i].chars().last()))
                {
                    s += 150.0;
                } else if title.contains(&lc) {
                    s += 50.0;
                }
                scored.insert(id, s);
            }
            // Full text: every word must prefix-match some indexed word.
            let mut ids: Option<BTreeSet<Uuid>> = None;
            let mut exact: f64 = 0.0;
            let words: Vec<&str> = lc.split_whitespace().collect();
            if q.len() >= 2 {
                for w in &words {
                    let mut found = BTreeSet::new();
                    for (k, set) in self.words.range(w.to_string()..) {
                        if !k.starts_with(w) {
                            break;
                        }
                        if k == w {
                            exact += 10.0;
                        }
                        found.extend(set.iter().copied());
                    }
                    ids = Some(match ids {
                        None => found,
                        Some(prev) => prev.intersection(&found).copied().collect(),
                    });
                }
            }
            for id in ids.unwrap_or_default() {
                let e = &self.entries[&id];
                if !e.off {
                    let s = 20.0 + exact.min(30.0);
                    scored.entry(id).and_modify(|v| *v += 5.0).or_insert(s);
                }
            }
            for (id, s) in scored {
                if self.hidden(id) {
                    continue;
                }
                hits.push(hit(id, &self.entries[&id], s + self.frecency(id, now_ms)));
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });
        hits.truncate(limit);
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Page;
    use std::time::Instant;

    fn task(n: u128, title: &str, desc: &str) -> Row {
        serde_json::from_value(serde_json::json!({
            "kind": "task", "id": Uuid::from_u128(n), "parent_id": Uuid::nil(),
            "title": title, "description": desc, "order": "a", "completed_at": null,
        }))
        .unwrap()
    }

    fn page(n: u128, title: &str, body: &str) -> Row {
        Row::Page(Page {
            id: Uuid::from_u128(n),
            parent_id: Uuid::nil(),
            title: title.into(),
            body: body.into(),
            rev: 0,
            order: String::new(),
            trashed_at: None,
            links: vec![],
            blobs: vec![],
        })
    }

    #[test]
    fn fuzzy_titles_rank_prefix_first() {
        let mut ix = Index::new();
        ix.upsert(&task(1, "Write project plan", ""));
        ix.upsert(&task(2, "Project kickoff", ""));
        ix.upsert(&task(3, "Buy milk", ""));
        let hits = ix.query("proj", 0, 10);
        assert_eq!(hits[0].id, Uuid::from_u128(2));
        assert_eq!(hits.len(), 2);
        assert_eq!(ix.query("wpp", 0, 10)[0].id, Uuid::from_u128(1));
        assert!(ix.query("zzz", 0, 10).is_empty());
    }

    #[test]
    fn full_text_prefix_and_attachments() {
        let mut ix = Index::new();
        ix.upsert(&page(1, "Notes", "Quarterly retrospective agenda"));
        ix.upsert(&task(
            2,
            "Taxes",
            "see ![scan](blob:abc) and [Receipt-2024.pdf](blob:def)",
        ));
        assert_eq!(ix.query("retro", 0, 10)[0].id, Uuid::from_u128(1));
        assert_eq!(ix.query("receipt-20", 0, 10)[0].id, Uuid::from_u128(2));
        assert_eq!(ix.query("scan", 0, 10)[0].id, Uuid::from_u128(2));
        ix.upsert(&page(1, "Notes", "something else"));
        assert!(ix.query("retro", 0, 10).is_empty());
        ix.remove(Uuid::from_u128(2));
        assert!(ix.query("receipt", 0, 10).is_empty());
    }

    #[test]
    fn trashed_and_archived_excluded() {
        let mut ix = Index::new();
        let mut t = task(1, "Gone", "");
        if let Row::Task(t) = &mut t {
            t.trashed_at = Some(5);
        }
        ix.upsert(&t);
        assert!(ix.query("gone", 0, 10).is_empty());
        let area: Row = serde_json::from_value(serde_json::json!({
            "kind": "area", "id": Uuid::from_u128(9), "name": "Old", "archived_at": 5
        }))
        .unwrap();
        ix.upsert(&area);
        let mut child = task(2, "Under old", "");
        if let Row::Task(t) = &mut child {
            t.parent_id = Uuid::from_u128(9);
        }
        ix.upsert(&child);
        assert!(ix.query("under", 0, 10).is_empty());
    }

    #[test]
    fn frecency_and_recents() {
        let mut ix = Index::new();
        ix.upsert(&task(1, "alpha one", ""));
        ix.upsert(&task(2, "alpha two", ""));
        assert!(ix.query("", 0, 10).is_empty());
        ix.record_open(Uuid::from_u128(2), 1000);
        assert_eq!(ix.query("alpha", 1000, 10)[0].id, Uuid::from_u128(2));
        let recents = ix.query("", 1000, 10);
        assert_eq!(recents.len(), 1);
        assert_eq!(recents[0].id, Uuid::from_u128(2));
    }

    /// Release: each keystroke < 16 ms (`cargo test --release -p orbit-core search_budget`).
    /// Debug builds are ~20x slower, so there the bound is a loose 200 ms.
    #[test]
    fn search_budget() {
        let words = [
            "project", "plan", "write", "review", "report", "budget", "design", "meeting", "notes",
            "launch", "pricing", "customer", "roadmap", "backlog", "sprint", "invoice",
        ];
        let mut ix = Index::new();
        for n in 0..10_000u128 {
            let w = |k: u128| words[((n * 7 + k * 13) % words.len() as u128) as usize];
            let title = format!("{} {} {} {}", w(0), w(1), w(2), n);
            let desc = format!("{} {} {} {} {}", w(3), w(4), w(5), w(6), w(7));
            ix.upsert(&task(n + 1, &title, &desc));
        }
        let limit = if cfg!(debug_assertions) { 200.0 } else { 16.0 };
        let mut worst: f64 = 0.0;
        for q in [
            "p",
            "pr",
            "pro",
            "proj",
            "proje",
            "projec",
            "project",
            "project p",
            "project pl",
            "project pla",
            "project plan",
        ] {
            let start = Instant::now();
            let hits = ix.query(q, 0, 20);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            eprintln!("search_budget {q:?}: {ms:.2} ms, {} hits", hits.len());
            worst = worst.max(ms);
            assert!(ms < limit, "{q:?} took {ms:.2} ms (limit {limit} ms)");
        }
        for q in [
            "r",
            "re",
            "rev",
            "revi",
            "revie",
            "review",
            "review b",
            "review bu",
        ] {
            let start = Instant::now();
            ix.query(q, 0, 20);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            eprintln!("search_budget {q:?}: {ms:.2} ms");
            assert!(ms < limit, "{q:?} took {ms:.2} ms (limit {limit} ms)");
        }
        eprintln!("search_budget worst: {worst:.2} ms");
    }
}
