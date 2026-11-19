//! Snapshots of `<data>/orbit.db` into `<data>/backups/`, pruning and restore.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// Milliseconds since the Unix epoch, for snapshot names.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Writes `<dir>/orbit-<now_ms>.db` with `VACUUM INTO` and returns its path.
pub fn snapshot(data: &Path, dir: &Path, now_ms: u64) -> rusqlite::Result<PathBuf> {
    fs::create_dir_all(dir).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let target = dir.join(format!("orbit-{now_ms}.db"));
    let quoted = target.to_string_lossy().replace('\'', "''");
    let conn = Connection::open(data.join("orbit.db"))?;
    // shortcut: no blobs yet, add with #56
    conn.execute_batch(&format!("VACUUM INTO '{quoted}'"))?;
    Ok(target)
}

/// Snapshot number from an `orbit-<n>.db` file name.
fn number(name: &str) -> Option<u64> {
    name.strip_prefix("orbit-")?
        .strip_suffix(".db")?
        .parse()
        .ok()
}

/// Snapshots in `dir` as `(number, path)`, newest first.
fn listed(dir: &Path) -> std::io::Result<Vec<(u64, PathBuf)>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let n = path.file_name().and_then(|f| f.to_str()).and_then(number);
        if let Some(n) = n {
            found.push((n, path));
        }
    }
    found.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    Ok(found)
}

/// Deletes all but the `keep` newest snapshots in `dir`.
pub fn prune(dir: &Path, keep: usize) -> std::io::Result<()> {
    for (_, path) in listed(dir)?.into_iter().skip(keep) {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Replaces `<data>/orbit.db` with `snapshot`. The server must not be running.
pub fn restore(data: &Path, snapshot: &Path) -> std::io::Result<()> {
    let staged = data.join("orbit.db.restore");
    fs::copy(snapshot, &staged)?;
    for stale in ["orbit.db-wal", "orbit.db-shm"] {
        match fs::remove_file(data.join(stale)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    fs::rename(staged, data.join("orbit.db"))
}

/// True when `dir` has no snapshot, or the newest is older than 24 h.
pub fn due(dir: &Path, now_ms: u64) -> bool {
    match listed(dir) {
        Ok(found) => match found.first() {
            None => true,
            Some((newest, _)) => now_ms.saturating_sub(*newest) > DAY_MS,
        },
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, unique directory under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("orbit-backup-unit-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prune_keeps_the_seven_newest_of_nine() {
        let dir = scratch("prune");
        for n in 1..=9u64 {
            fs::write(dir.join(format!("orbit-{n}.db")), b"x").unwrap();
        }
        prune(&dir, 7).unwrap();
        let mut left: Vec<u64> = listed(&dir).unwrap().into_iter().map(|(n, _)| n).collect();
        left.sort();
        assert_eq!(left, vec![3, 4, 5, 6, 7, 8, 9]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn due_when_empty_or_newest_is_over_a_day_old() {
        let dir = scratch("due");
        let now = 10 * DAY_MS;
        assert!(due(&dir, now), "no snapshot is due");
        fs::write(dir.join(format!("orbit-{}.db", now - DAY_MS + 1)), b"x").unwrap();
        assert!(!due(&dir, now), "newest under 24 h old is not due");
        fs::write(dir.join(format!("orbit-{}.db", now - DAY_MS - 1)), b"x").unwrap();
        assert!(!due(&dir, now), "an older file does not matter");
        fs::remove_file(dir.join(format!("orbit-{}.db", now - DAY_MS + 1))).unwrap();
        assert!(due(&dir, now), "newest over 24 h old is due");
        let _ = fs::remove_dir_all(&dir);
    }
}
