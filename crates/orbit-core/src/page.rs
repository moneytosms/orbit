//! Pages: markdown documents owned by an Area, Project or Goal.
use crate::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Page {
    pub id: Uuid,
    /// An Area, Project or Goal. Pages don't nest.
    pub parent_id: Uuid,
    pub title: String,
    /// Markdown.
    #[serde(default)]
    pub body: String,
    /// Counts body edits: an edit made against an older one is a conflict.
    #[serde(default)]
    pub rev: u64,
    #[serde(default)]
    pub order: String,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// The `orbit:` targets in `body`, kept up to date by every body write.
    #[serde(default)]
    pub links: Vec<Uuid>,
    /// The `blob:` sha256 hashes in `body`, kept up to date by every body write.
    #[serde(default)]
    pub blobs: Vec<String>,
}

/// Whether an edit made against `base_rev` clashes with the page as it is now.
pub fn is_conflict(page: &Page, base_rev: u64) -> bool {
    page.rev != base_rev
}

/// The id of the sibling a losing `cmd` creates: the same command always makes the same one.
pub fn conflict_id(page: Uuid, cmd: Uuid) -> Uuid {
    crate::completion_id(page, cmd)
}

/// The UTC `YYYY-MM-DD` of a unix-ms clock reading.
pub fn utc_date(wall_ms: u64) -> String {
    crate::parse::fmt_date((wall_ms / 86_400_000) as i64)
}

/// The title of the sibling that keeps a losing edit. `date` is `YYYY-MM-DD`.
pub fn conflict_title(title: &str, device: &str, date: &str) -> String {
    format!("{title} (conflict, {device}, {date})")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(rev: u64) -> Page {
        Page {
            id: Uuid::nil(),
            parent_id: Uuid::nil(),
            title: "Notes".into(),
            body: String::new(),
            rev,
            order: String::new(),
            trashed_at: None,
            links: vec![],
            blobs: vec![],
        }
    }

    #[test]
    fn only_a_matching_base_rev_is_not_a_conflict() {
        assert!(!is_conflict(&page(2), 2));
        assert!(is_conflict(&page(2), 1));
        assert!(is_conflict(&page(0), 3));
    }

    #[test]
    fn the_conflict_title_names_device_and_date() {
        assert_eq!(
            conflict_title("Notes", "Laptop", "2026-03-04"),
            "Notes (conflict, Laptop, 2026-03-04)"
        );
    }
}
