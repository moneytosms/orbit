use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use orbit_core::{Command, Row, Status, SyncRequest, SyncResponse, apply};
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
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rows (id TEXT PRIMARY KEY, data TEXT NOT NULL, seq INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS rows_seq ON rows (seq);
         CREATE TABLE IF NOT EXISTS processed (uuid TEXT PRIMARY KEY, status TEXT NOT NULL, at INTEGER NOT NULL);",
    )?;
    let inbox = Row::inbox();
    if !exists(&conn, &inbox)? {
        write(&conn, &inbox)?;
    }
    Ok(Router::new()
        .route("/sync", post(sync))
        .with_state(Arc::new(Mutex::new(conn))))
}

async fn sync(
    State(db): State<Db>,
    Json(req): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, StatusCode> {
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
    for cmd in &req.commands {
        statuses.insert(cmd.uuid, run_command(conn, cmd, now)?);
    }
    let max: i64 = conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM rows", [], |r| r.get(0))?;
    // A token from before a data reset can't be trusted: send everything.
    let since = req.token.filter(|&t| t <= max as u64).unwrap_or(0);
    let mut stmt = conn.prepare("SELECT data, seq FROM rows WHERE seq > ? ORDER BY seq")?;
    let mut token = since;
    let mut delta = Vec::new();
    for r in stmt.query_map([since as i64], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
    })? {
        let (data, seq) = r?;
        delta.push(serde_json::from_str(&data).map_err(json_err)?);
        token = seq;
    }
    Ok(SyncResponse {
        statuses,
        delta,
        token,
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
    let existing: Option<String> = tx
        .query_row(
            "SELECT data FROM rows WHERE id = ?",
            [cmd.op.target().to_string()],
            |r| r.get(0),
        )
        .optional()?;
    let existing = existing
        .map(|d| serde_json::from_str(&d))
        .transpose()
        .map_err(json_err)?;
    let status = match apply(existing, cmd) {
        Ok(row) => {
            write(&tx, &row)?;
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

fn exists(conn: &Connection, row: &Row) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM rows WHERE id = ?)",
        [row.id().to_string()],
        |r| r.get(0),
    )
}

/// Upserts a row with the next global change seq.
fn write(conn: &Connection, row: &Row) -> rusqlite::Result<()> {
    let data = serde_json::to_string(row).map_err(json_err)?;
    conn.execute(
        "INSERT INTO rows (id, data, seq) VALUES (?1, ?2, (SELECT COALESCE(MAX(seq), 0) + 1 FROM rows))
         ON CONFLICT (id) DO UPDATE SET data = excluded.data, seq = excluded.seq",
        [row.id().to_string(), data],
    )?;
    Ok(())
}

fn json_err(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(e.into())
}
