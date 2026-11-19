use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use orbit_core::{
    Clocks, Command, Hlc, PROTOCOL_V, Row, Status, SyncRequest, SyncResponse, VersionError,
    apply_lww, check_version,
};
use rusqlite::{Connection, OptionalExtension};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

/// How long a command uuid's status is kept for replays.
const KEEP_PROCESSED_MS: i64 = 30 * 24 * 60 * 60 * 1000;

// shortcut: one connection behind a mutex, queries run on the async thread; move to spawn_blocking/pool when load matters.
type Db = Arc<Mutex<Connection>>;

/// Opens (or creates) `<data_dir>/orbit.db` and returns the HTTP app.
pub fn app(data_dir: &Path) -> rusqlite::Result<Router> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    let conn = Connection::open(data_dir.join("orbit.db"))?;
    // `clocks` is the row's per-field last-writer HLCs (JSON); it never leaves the server.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rows (id TEXT PRIMARY KEY, data TEXT NOT NULL, clocks TEXT NOT NULL, seq INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS rows_seq ON rows (seq);
         CREATE TABLE IF NOT EXISTS processed (uuid TEXT PRIMARY KEY, status TEXT NOT NULL, at INTEGER NOT NULL);",
    )?;
    // Data dirs from before per-field clocks.
    let has_clocks: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('rows') WHERE name = 'clocks'",
        [],
        |r| r.get(0),
    )?;
    if !has_clocks {
        conn.execute(
            "ALTER TABLE rows ADD COLUMN clocks TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }
    let inbox = Row::inbox();
    if load(&conn, &inbox.id().to_string())?.is_none() {
        write(&conn, &inbox, &Clocks::new())?;
    }
    Ok(Router::new()
        .route("/sync", post(sync))
        .with_state(Arc::new(Mutex::new(conn))))
}

async fn sync(
    State(db): State<Db>,
    Json(req): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, StatusCode> {
    match check_version(PROTOCOL_V, req.v) {
        Err(VersionError::TooOld) => return Err(StatusCode::UPGRADE_REQUIRED),
        Err(VersionError::TooNew) => return Err(StatusCode::BAD_REQUEST),
        Ok(()) => {}
    }
    if req.commands.len() > 100 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    // A panic mid-request leaves the connection usable; don't let poison take the server down.
    let mut conn = db.lock().unwrap_or_else(|e| e.into_inner());
    run(&mut conn, req).map(Json).map_err(|e| {
        eprintln!("sync failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn run(conn: &mut Connection, req: SyncRequest) -> rusqlite::Result<SyncResponse> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    conn.execute(
        "DELETE FROM processed WHERE at < ?",
        [now - KEEP_PROCESSED_MS],
    )?;
    let mut statuses = BTreeMap::new();
    let mut rejected = Vec::new();
    for cmd in &req.commands {
        let status = run_command(conn, cmd, now)?;
        if status != Status::Ok {
            rejected.push(cmd.op.target().to_string());
        }
        statuses.insert(cmd.uuid, status);
    }
    let max: i64 = conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM rows", [], |r| r.get(0))?;
    // A token from before a data reset can't be trusted: send everything.
    let since = req.token.filter(|&t| t <= max as u64).unwrap_or(0);
    let mut stmt =
        conn.prepare("SELECT id, data, clocks, seq FROM rows WHERE seq > ? ORDER BY seq")?;
    let mut token = since;
    let mut rows = Vec::new();
    for r in stmt.query_map([since as i64], |r| {
        Ok((
            r.get::<_, String>(0)?,
            (r.get(1)?, r.get(2)?),
            r.get::<_, i64>(3)? as u64,
        ))
    })? {
        let (id, row, seq) = r?;
        rows.push((id, row));
        token = seq;
    }
    // Rejected commands' rows go back even if unchanged, so clients can undo their guess.
    for id in rejected {
        if !rows.iter().any(|(have, _)| *have == id)
            && let Some(row) = load(conn, &id)?
        {
            rows.push((id, row));
        }
    }
    let mut delta = Vec::new();
    let mut clock: Option<Hlc> = None;
    for (_, (data, clocks)) in rows {
        delta.push(serde_json::from_str(&data).map_err(json_err)?);
        let clocks: Clocks = serde_json::from_str(&clocks).map_err(json_err)?;
        clock = clock.into_iter().chain(clocks.into_values()).max();
    }
    Ok(SyncResponse {
        statuses,
        delta,
        token,
        clock,
    })
}

/// One transaction per command; a rejected command doesn't stop the batch.
fn run_command(conn: &mut Connection, cmd: &Command, now: i64) -> rusqlite::Result<Status> {
    let tx = conn.transaction()?;
    let uuid = cmd.uuid.to_string();
    let seen: Option<String> = tx
        .query_row(
            "SELECT status FROM processed WHERE uuid = ?",
            [&uuid],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(seen) = seen {
        return serde_json::from_str(&seen).map_err(json_err);
    }
    let (existing, mut clocks) = match load(&tx, &cmd.op.target().to_string())? {
        Some((data, clocks)) => (
            Some(serde_json::from_str(&data).map_err(json_err)?),
            serde_json::from_str(&clocks).map_err(json_err)?,
        ),
        None => (None, Clocks::new()),
    };
    let status = match apply_lww(existing, &mut clocks, cmd) {
        Ok(row) => {
            write(&tx, &row, &clocks)?;
            Status::Ok
        }
        Err(code) => Status::Error { code },
    };
    let stored = serde_json::to_string(&status).map_err(json_err)?;
    tx.execute(
        "INSERT INTO processed (uuid, status, at) VALUES (?1, ?2, ?3)",
        (&uuid, stored, now),
    )?;
    tx.commit()?;
    Ok(status)
}

/// A row's `(data, clocks)` JSON.
fn load(conn: &Connection, id: &str) -> rusqlite::Result<Option<(String, String)>> {
    conn.query_row("SELECT data, clocks FROM rows WHERE id = ?", [id], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })
    .optional()
}

/// Upserts a row with the next global change seq.
fn write(conn: &Connection, row: &Row, clocks: &Clocks) -> rusqlite::Result<()> {
    let data = serde_json::to_string(row).map_err(json_err)?;
    let clocks = serde_json::to_string(clocks).map_err(json_err)?;
    conn.execute(
        "INSERT INTO rows (id, data, clocks, seq) VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(seq), 0) + 1 FROM rows))
         ON CONFLICT (id) DO UPDATE SET data = excluded.data, clocks = excluded.clocks, seq = excluded.seq",
        [row.id().to_string(), data, clocks],
    )?;
    Ok(())
}

fn json_err(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(e.into())
}
