//! Links: `[Title](orbit:<uuid>)` in a body, and the Backlinks they make.
use crate::{Row, Uuid, trash};

/// The `orbit:` link targets in `markdown`, deduplicated, in order of first appearance.
/// Escaped brackets, code spans, code blocks and non-uuid targets don't count.
pub fn targets(markdown: &str) -> Vec<Uuid> {
    let mut out = Vec::new();
    let mut fence: Option<char> = None;
    for line in markdown.lines() {
        let t = line.trim_start();
        let mark = if t.starts_with("```") {
            Some('`')
        } else if t.starts_with("~~~") {
            Some('~')
        } else {
            None
        };
        match (fence, mark) {
            (None, Some(m)) => {
                fence = Some(m);
                continue;
            }
            (Some(f), Some(m)) if f == m => {
                fence = None;
                continue;
            }
            (Some(_), _) => continue,
            _ => {}
        }
        scan(line, &mut out);
    }
    out
}

fn scan(line: &str, out: &mut Vec<Uuid>) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '`' => {
                let run = chars[i..].iter().take_while(|c| **c == '`').count();
                // A code span closes on a backtick run of the same length.
                let mut j = i + run;
                let mut close = None;
                while j < chars.len() {
                    if chars[j] == '`' {
                        let r = chars[j..].iter().take_while(|c| **c == '`').count();
                        if r == run {
                            close = Some(j + r);
                            break;
                        }
                        j += r;
                    } else {
                        j += 1;
                    }
                }
                i = close.unwrap_or(i + run);
            }
            '[' => {
                // Find the closing `]`, honouring escapes and nesting.
                let mut depth = 1;
                let mut j = i + 1;
                while j < chars.len() && depth > 0 {
                    match chars[j] {
                        '\\' => j += 1,
                        '[' => depth += 1,
                        ']' => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                if depth == 0 {
                    let rest: String = chars[j..].iter().collect();
                    if let Some(after) = rest.strip_prefix("(orbit:")
                        && let Some(end) = after.find(')')
                        && let Ok(id) = after[..end].trim().parse::<Uuid>()
                    {
                        if !out.contains(&id) {
                            out.push(id);
                        }
                        i = j + "(orbit:".len() + end + 1;
                        continue;
                    }
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
}

/// The sha256 hashes (lowercase hex) of the `blob:` links in `markdown`, deduplicated, in order of
/// first appearance. `![name](blob:<hash>)` and `[name](blob:<hash>)` both count; code doesn't.
pub fn blob_refs(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fence = false;
    for line in markdown.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
            continue;
        }
        if fence {
            continue;
        }
        let mut rest = line;
        while let Some(at) = rest.find("](blob:") {
            let after = &rest[at + "](blob:".len()..];
            let hash: String = after.chars().take_while(|c| *c != ')').collect();
            let hash = hash.trim().to_ascii_lowercase();
            if hash.len() == 64
                && hash.bytes().all(|b| b.is_ascii_hexdigit())
                && !out.contains(&hash)
            {
                out.push(hash);
            }
            rest = after;
        }
    }
    out
}

/// The ids of the Pages and Tasks whose bodies link to `id`, leaving out any in the Trash
/// (or under something in the Trash).
pub fn backlinks(rows: &[Row], id: Uuid) -> Vec<Uuid> {
    let lookup = |i: Uuid| rows.iter().find(|r| r.id() == i).cloned();
    rows.iter()
        .filter(|r| match r {
            Row::Page(p) => p.links.contains(&id),
            Row::Task(t) => t.links.contains(&id),
            _ => false,
        })
        .filter(|r| {
            trash::chain(r, &lookup)
                .iter()
                .all(|c| c.trashed_at().is_none())
        })
        .map(Row::id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "00000000-0000-7000-8000-00000000000a";
    const B: &str = "00000000-0000-7000-8000-00000000000b";

    fn id(s: &str) -> Uuid {
        s.parse().unwrap()
    }

    #[test]
    fn finds_links_in_order_without_duplicates() {
        let md = format!("[B](orbit:{B}) and [A](orbit:{A}) and again [B2](orbit:{B})");
        assert_eq!(targets(&md), vec![id(B), id(A)]);
    }

    #[test]
    fn ignores_non_uuid_targets_and_other_schemes() {
        assert!(targets("[Foo](orbit:p1) [x](https://a.b)").is_empty());
    }

    #[test]
    fn ignores_escaped_brackets() {
        assert!(targets(&format!("\\[x](orbit:{A})")).is_empty());
        assert_eq!(targets(&format!("[a\\]b](orbit:{A})")), vec![id(A)]);
    }

    #[test]
    fn ignores_code() {
        let md = format!("`[x](orbit:{A})`\n```\n[y](orbit:{A})\n```\n~~~\n[z](orbit:{A})\n~~~\n");
        assert!(targets(&md).is_empty());
        let md = format!("``a ` [x](orbit:{A})`` then [B](orbit:{B})");
        assert_eq!(targets(&md), vec![id(B)]);
    }

    #[test]
    fn blob_refs_find_images_and_files_without_duplicates() {
        let h1 = "a".repeat(64);
        let h2 = "B".repeat(64);
        let md = format!(
            "![pic](blob:{h1}) and [doc.pdf](blob:{h2}) again ![pic](blob:{h1})\n```\n[x](blob:{h1})\n```\n[bad](blob:abc)"
        );
        assert_eq!(blob_refs(&md), vec![h1, "b".repeat(64)]);
    }

    #[test]
    fn backlinks_skip_trashed_sources() {
        let page = |n: u128, links: Vec<Uuid>, trashed: Option<u64>| {
            Row::Page(crate::Page {
                id: Uuid::from_u128(n),
                parent_id: Uuid::nil(),
                title: "P".into(),
                body: String::new(),
                rev: 0,
                order: String::new(),
                trashed_at: trashed,
                links,
                blobs: vec![],
            })
        };
        let rows = vec![
            page(1, vec![id(A)], None),
            page(2, vec![id(A)], Some(5)),
            page(3, vec![id(B)], None),
        ];
        assert_eq!(backlinks(&rows, id(A)), vec![Uuid::from_u128(1)]);
    }
}
