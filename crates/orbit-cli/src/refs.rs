//! Refs name a row: its number in this terminal session's last listing, or an id prefix of 6+ characters.
use crate::out::{R, fail, internal, usage};
use orbit_client::Client;
use orbit_core::Uuid;
use std::path::{Path, PathBuf};

/// The terminal session a listing belongs to: `ORBIT_SESSION`, else `TERM_SESSION_ID`, else one shared slot.
fn session() -> String {
    let raw = std::env::var("ORBIT_SESSION")
        .or_else(|_| std::env::var("TERM_SESSION_ID"))
        .unwrap_or_else(|_| "default".into());
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

fn listing_path(dir: &Path) -> PathBuf {
    dir.join(format!("listing-{}.json", session()))
}

/// Remembers the ids a listing showed, in row order, for this session.
pub fn save_listing(dir: &Path, ids: &[Uuid]) -> R<()> {
    let ids: Vec<String> = ids.iter().map(Uuid::to_string).collect();
    let json = serde_json::to_string(&ids).map_err(internal)?;
    std::fs::write(listing_path(dir), json).map_err(internal)
}

fn load_listing(dir: &Path) -> R<Vec<Uuid>> {
    let Ok(text) = std::fs::read_to_string(listing_path(dir)) else {
        return Ok(vec![]);
    };
    let ids: Vec<String> = serde_json::from_str(&text).map_err(internal)?;
    ids.iter()
        .map(|s| Uuid::parse_str(s).map_err(internal))
        .collect()
}

/// Resolves a ref to a row id. Ambiguous or unknown refs are errors, never guesses.
pub fn resolve(dir: &Path, client: &Client, r: &str) -> R<Uuid> {
    let r = r.trim();
    if !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()) {
        let listing = load_listing(dir)?;
        let n: usize = r.parse().unwrap_or(0);
        return n
            .checked_sub(1)
            .and_then(|i| listing.get(i).copied())
            .ok_or_else(|| {
                fail(
                    "not_found",
                    format!("no row {r} in the last listing"),
                    Some("list first with `orbit ls`, or use an id prefix of 6+ characters"),
                )
            });
    }
    let hex: String = r
        .chars()
        .filter(|c| *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if hex.len() < 6 {
        return Err(usage(format!(
            "`{r}` is neither a row number nor an id prefix of 6+ characters"
        )));
    }
    let hits: Vec<Uuid> = client
        .rows()
        .map_err(internal)?
        .iter()
        .map(|row| row.id())
        .filter(|id| id.to_string().replace('-', "").starts_with(&hex))
        .collect();
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => Err(fail(
            "not_found",
            format!("no row starts with `{r}`"),
            Some("run `orbit ls` to see row numbers"),
        )),
        many => Err(fail(
            "ambiguous",
            format!("`{r}` matches {} rows", many.len()),
            Some("use a longer id prefix or a row number"),
        )),
    }
}
