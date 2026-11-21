//! Refs resolve by the last device-wide listing, id prefix, or unique title match.
use crate::out::{R, fail, internal, usage};
use orbit_client::Client;
use orbit_core::{Row, Uuid};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const TTL_SECS: u64 = 24 * 60 * 60;
static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
struct Listing {
    listed_at: u64,
    ids: Vec<String>,
}

fn listing_path(dir: &Path) -> PathBuf {
    dir.join("listing.json")
}

pub fn save_listing(dir: &Path, ids: &[Uuid]) -> R<()> {
    let listing = Listing {
        listed_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        ids: ids.iter().map(ToString::to_string).collect(),
    };
    let json = serde_json::to_vec(&listing).map_err(internal)?;
    let destination = listing_path(dir);
    let (temp_path, mut temp) = loop {
        let id = TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!("listing-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => break (path, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(internal(error)),
        }
    };
    if let Err(error) = temp.write_all(&json) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(internal(error));
    }
    drop(temp);
    if let Err(error) = std::fs::rename(&temp_path, destination) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(internal(error));
    }
    Ok(())
}

fn load_listing(dir: &Path) -> R<Option<(Vec<Uuid>, u64)>> {
    let Ok(text) = std::fs::read_to_string(listing_path(dir)) else {
        return Ok(None);
    };
    let listing: Listing = serde_json::from_str(&text).map_err(internal)?;
    let ids = listing
        .ids
        .iter()
        .map(|s| Uuid::parse_str(s).map_err(internal))
        .collect::<R<_>>()?;
    Ok(Some((ids, listing.listed_at)))
}

pub fn listing_len(dir: &Path) -> R<usize> {
    Ok(load_listing(dir)?.map_or(0, |(ids, _)| ids.len()))
}

fn expired(listed_at: u64) -> Option<String> {
    let age = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(listed_at);
    (age > TTL_SECS).then(|| {
        let days = (age / (24 * 60 * 60)).max(1);
        format!(
            "listing is {days} day{} old, run `orbit ls`",
            if days == 1 { "" } else { "s" }
        )
    })
}

fn row_number(dir: &Path, r: &str) -> R<Uuid> {
    let Some((ids, listed_at)) = load_listing(dir)? else {
        return Err(fail(
            "not_found",
            format!("no row {r} in the last listing"),
            Some("run `orbit ls`"),
        ));
    };
    if let Some(hint) = expired(listed_at) {
        return Err(fail("stale_listing", hint.clone(), Some("run `orbit ls`")));
    }
    let n: usize = r.parse().unwrap_or(0);
    n.checked_sub(1)
        .and_then(|i| ids.get(i).copied())
        .ok_or_else(|| {
            fail(
                "not_found",
                format!("no row {r} in the last listing"),
                Some("run `orbit ls`"),
            )
        })
}

fn compact(id: Uuid) -> String {
    id.to_string().replace('-', "")
}

/// Resolves by row number, unique id prefix, or unique title substring.
pub fn resolve(dir: &Path, client: &Client, r: &str) -> R<Uuid> {
    let r = r.trim();
    if r.is_empty() {
        return Err(usage(
            "show needs a ref; try `orbit show 3` or `orbit show 3f2b9c`",
        ));
    }
    let numeric_row = r.bytes().all(|b| b.is_ascii_digit());
    let row_error = if numeric_row {
        match row_number(dir, r) {
            Ok(id) => return Ok(id),
            Err(error) => Some(error),
        }
    } else {
        None
    };

    let hex: String = r
        .chars()
        .filter(|c| *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if hex.bytes().all(|b| b.is_ascii_hexdigit()) && hex.len() >= 4 {
        let ids: Vec<Uuid> = client
            .rows()
            .map_err(internal)?
            .iter()
            .map(Row::id)
            .filter(|id| compact(*id).starts_with(&hex))
            .collect();
        match ids.as_slice() {
            [one] => return Ok(*one),
            [] => {}
            many => {
                let suggestions: Vec<String> = many
                    .iter()
                    .map(|id| {
                        let value = compact(*id);
                        let min = (hex.len() + 1..=value.len())
                            .find(|n| {
                                client.rows().ok().is_some_and(|rows| {
                                    rows.iter()
                                        .filter(|row| compact(row.id()).starts_with(&value[..*n]))
                                        .count()
                                        == 1
                                })
                            })
                            .unwrap_or(value.len());
                        value[..min].to_string()
                    })
                    .collect();
                let hint = format!(
                    "use a longer id prefix: {}; run `orbit ls`",
                    suggestions
                        .iter()
                        .map(|s| format!("`{s}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Err(fail(
                    "ambiguous",
                    format!("`{r}` matches {} rows", many.len()),
                    Some(&hint),
                ));
            }
        }
    }

    let needle = r.to_lowercase();
    let rows = client.rows().map_err(internal)?;
    let matches: Vec<(Uuid, String)> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Task(task) if task.title.to_lowercase().contains(&needle) => {
                Some((task.id, task.title.clone()))
            }
            Row::Routine(routine) if routine.name.to_lowercase().contains(&needle) => {
                Some((routine.id, routine.name.clone()))
            }
            _ => None,
        })
        .collect();
    match matches.as_slice() {
        [(id, _)] => Ok(*id),
        [] => match row_error {
            Some(error) => Err(error),
            None => Err(fail(
                "not_found",
                format!("no task or routine matches `{r}`"),
                Some("run `orbit ls`"),
            )),
        },
        many => {
            let listing = load_listing(dir)?.map(|(ids, _)| ids).unwrap_or_default();
            let candidates = many
                .iter()
                .enumerate()
                .map(|(i, (id, title))| {
                    let row = listing.iter().position(|listed| listed == id).unwrap_or(i);
                    format!("{}: {}", row + 1, title)
                })
                .collect::<Vec<_>>()
                .join("; ");
            Err(fail(
                "ambiguous",
                format!("`{r}` matches multiple rows: {candidates}"),
                Some("use a row number or a longer id prefix; run `orbit ls`"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_listing_updates_leave_a_complete_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut writers = vec![];
        for n in 1..=8 {
            let path = dir.path().to_path_buf();
            writers.push(std::thread::spawn(move || {
                save_listing(&path, &[Uuid::from_u128(n)]).unwrap();
            }));
        }
        for writer in writers {
            writer.join().unwrap();
        }
        let (ids, listed_at) = load_listing(dir.path()).unwrap().unwrap();
        assert_eq!(ids.len(), 1);
        assert!(listed_at > 0);
    }
}
