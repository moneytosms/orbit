//! Attachments: content-addressed files in `<data>/blobs/ab/cd/<sha256>`, uploaded before the
//! command that references them, and deleted once nothing references them for a week.
use crate::{St, auth};
use axum::{
    body::Bytes,
    extract::{Path as UrlPath, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use orbit_core::{Command, Op};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

/// How long an unreferenced blob is kept before it is deleted.
pub(crate) const GRACE_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// The per-file size limit in bytes: `ORBIT_MAX_BLOB_MB`, 25 MB if unset.
pub(crate) fn max_bytes() -> usize {
    std::env::var("ORBIT_MAX_BLOB_MB")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(25)
        * 1024
        * 1024
}

fn valid(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Where the blob with this (valid) hash lives.
pub(crate) fn path(data: &Path, hash: &str) -> PathBuf {
    data.join("blobs")
        .join(&hash[..2])
        .join(&hash[2..4])
        .join(hash)
}

pub(crate) fn exists(data: &Path, hash: &str) -> bool {
    valid(hash) && path(data, hash).is_file()
}

/// The `blob:` hashes a command's body write references.
pub(crate) fn refs(cmd: &Command) -> Vec<String> {
    match &cmd.op {
        Op::PageCreate { body, .. } | Op::PageEditBody { body, .. } => {
            orbit_core::link::blob_refs(body)
        }
        Op::TaskUpdate {
            description: Some(d),
            ..
        } => orbit_core::link::blob_refs(d),
        _ => vec![],
    }
}

pub(crate) async fn put(
    State(st): State<St>,
    _: auth::Device,
    UrlPath(hash): UrlPath<String>,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    if !valid(&hash) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.len() > st.max_blob {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    if hex::encode(Sha256::digest(&body)) != hash {
        return Err(StatusCode::BAD_REQUEST);
    }
    store(&st.data, &hash, &body)
        .map(|created| {
            if created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            }
        })
        .map_err(|e| {
            eprintln!("blob put failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Writes a blob whose `hash` is already checked. True if it was new.
pub(crate) fn store(data: &Path, hash: &str, bytes: &[u8]) -> std::io::Result<bool> {
    let target = path(data, hash);
    if target.is_file() {
        return Ok(false);
    }
    fs::create_dir_all(target.parent().unwrap())?;
    let tmp = target.with_extension("part");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, &target)?;
    Ok(true)
}

pub(crate) async fn get(
    State(st): State<St>,
    _: auth::Device,
    UrlPath(hash): UrlPath<String>,
) -> Result<Response, StatusCode> {
    if !valid(&hash) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let bytes = fs::read(path(&st.data, &hash)).map_err(|_| StatusCode::NOT_FOUND)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable",
            ),
        ],
        bytes,
    )
        .into_response())
}

/// Every blob file under `<data>/blobs` as `(hash, path)`.
pub(crate) fn on_disk(data: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let mut dirs = vec![data.join("blobs")];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if let Some(name) = p.file_name().and_then(|n| n.to_str())
                && valid(name)
            {
                found.push((name.to_string(), p));
            }
        }
    }
    found
}

/// Deletes blobs that no live row (Trash counts as live; purged rows hold no refs) has
/// referenced for `GRACE_MS`. The grace starts the first pass that finds a blob unreferenced.
pub(crate) fn gc(conn: &Connection, data: &Path, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS blob_orphans (hash TEXT PRIMARY KEY, since INTEGER NOT NULL)",
        [],
    )?;
    let referenced: HashSet<String> = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT j.value FROM rows, json_each(rows.data, '$.blobs') j
             WHERE json_extract(rows.data, '$.kind') != 'purged'",
        )?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    let files = on_disk(data);
    let present: HashSet<&str> = files.iter().map(|(h, _)| h.as_str()).collect();
    for (hash, file) in &files {
        if referenced.contains(hash) {
            conn.execute("DELETE FROM blob_orphans WHERE hash = ?", [hash])?;
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO blob_orphans (hash, since) VALUES (?1, ?2)",
            (hash, now),
        )?;
        let since: i64 = conn.query_row(
            "SELECT since FROM blob_orphans WHERE hash = ?",
            [hash],
            |r| r.get(0),
        )?;
        if now - since >= GRACE_MS && fs::remove_file(file).is_ok() {
            conn.execute("DELETE FROM blob_orphans WHERE hash = ?", [hash])?;
        }
    }
    // Forget orphans whose file is gone.
    let known: Vec<String> = {
        let mut stmt = conn.prepare("SELECT hash FROM blob_orphans")?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    for hash in known.iter().filter(|h| !present.contains(h.as_str())) {
        conn.execute("DELETE FROM blob_orphans WHERE hash = ?", [hash])?;
    }
    Ok(())
}
