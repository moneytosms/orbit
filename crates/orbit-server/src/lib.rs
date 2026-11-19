use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use orbit_core::{
    Clocks, Command, ErrorCode, Hlc, Op, PROTOCOL_V, Row, Status, SyncRequest, SyncResponse,
    VersionError, apply, apply_lww, check_parent, check_version,
};
use rusqlite::{Connection, OptionalExtension};
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

/// How long a command uuid's status is kept for replays.
const KEEP_PROCESSED_MS: i64 = 30 * 24 * 60 * 60 * 1000;

mod auth;
pub use auth::reset_password;

pub(crate) struct Shared {
    // shortcut: one connection behind a mutex, queries run on the async thread; move to spawn_blocking/pool when load matters.
    db: Mutex<Connection>,
    /// The data dir, for free-space checks.
    data: PathBuf,
    /// The one-time first-run code; `None` once the user exists.
    setup: Mutex<Option<String>>,
    limits: Mutex<HashMap<std::net::IpAddr, auth::Limit>>,
}
type St = Arc<Shared>;

/// Locks a mutex even if a panic mid-request poisoned it: the data behind it stays usable.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn open(data_dir: &Path) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    Connection::open(data_dir.join("orbit.db"))
}

/// Opens (or creates) `<data_dir>/orbit.db` and returns the HTTP app, plus the setup code
/// to print when no user exists yet. Serve it with `into_make_service_with_connect_info::<SocketAddr>`.
pub fn app(data_dir: &Path) -> rusqlite::Result<(Router, Option<String>)> {
    let conn = open(data_dir)?;
    auth::migrate(&conn)?;
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
    let setup = auth::setup_code(&conn)?;
    let shared = Shared {
        db: Mutex::new(conn),
        data: data_dir.to_path_buf(),
        setup: Mutex::new(setup.clone()),
        limits: Mutex::default(),
    };
    let router = Router::new()
        .route("/sync", post(sync))
        .route("/export", get(export))
        .route("/disk", get(disk))
        .route("/auth", get(auth::state))
        .route("/auth/setup", post(auth::setup))
        .route("/auth/login", post(auth::login))
        .route("/auth/password", post(auth::password))
        .route("/auth/totp", post(auth::totp_start))
        .route("/auth/totp/confirm", post(auth::totp_confirm))
        .route("/devices", get(auth::devices).post(auth::create_token))
        .route("/devices/{id}", delete(auth::revoke))
        .with_state(Arc::new(shared));
    Ok((router, setup))
}

async fn sync(
    State(st): State<St>,
    _: auth::Device,
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
    let mut conn = lock(&st.db);
    run(&mut conn, req).map(Json).map_err(|e| {
        eprintln!("sync failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// A zip of every row's data as `items.json`.
async fn export(State(st): State<St>, _: auth::Device) -> Result<Response, StatusCode> {
    // shortcut: no Pages/blobs yet, add with #54/#56
    let items = {
        let conn = lock(&st.db);
        items_json(&conn).map_err(|e| {
            eprintln!("export failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };
    let zipped = zip_items(&items).map_err(|e| {
        eprintln!("export failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"orbit-export.zip\"",
            ),
        ],
        zipped,
    )
        .into_response())
}

/// Every row's `data` as a JSON array, ordered by id.
fn items_json(conn: &Connection) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT data FROM rows ORDER BY id")?;
    let mut items = Vec::new();
    for data in stmt.query_map([], |r| r.get::<_, String>(0))? {
        items.push(serde_json::from_str::<serde_json::Value>(&data?)?);
    }
    Ok(serde_json::to_vec(&items)?)
}

fn zip_items(items: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("items.json", options)?;
    zip.write_all(items)?;
    Ok(zip.finish()?.into_inner())
}

/// Free bytes on the disk holding the data dir.
async fn disk(
    State(st): State<St>,
    _: auth::Device,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let free_bytes = fs4::available_space(&st.data).map_err(|e| {
        eprintln!("disk failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "free_bytes": free_bytes })))
}

fn run(conn: &mut Connection, req: SyncRequest) -> rusqlite::Result<SyncResponse> {
    let now = now_ms();
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
    let status = match check_tree(&tx, existing.as_ref(), cmd)? {
        Some(code) => Status::Error { code },
        None => match apply_lww(existing.clone(), &mut clocks, cmd) {
            Ok(row) => {
                write(&tx, &row, &clocks)?;
                if let Op::TaskComplete { id, .. } = &cmd.op {
                    if let Some(Row::Task(old)) = existing {
                        log_completion(&tx, old, cmd)?;
                    }
                    // A recurring Task only moves on; its subtasks stay as they are.
                    if matches!(&row, Row::Task(t) if t.completed_at.is_some()) {
                        complete_descendants(&tx, *id, cmd)?;
                    }
                }
                Status::Ok
            }
            Err(code) => Status::Error { code },
        },
    };
    let stored = serde_json::to_string(&status).map_err(json_err)?;
    tx.execute(
        "INSERT INTO processed (uuid, status, at) VALUES (?1, ?2, ?3)",
        (&uuid, stored, now),
    )?;
    tx.commit()?;
    Ok(status)
}

fn load_row(conn: &Connection, id: &str) -> rusqlite::Result<Option<Row>> {
    load(conn, id)?
        .map(|(data, _)| serde_json::from_str(&data).map_err(json_err))
        .transpose()
}

/// Tree rules for creates and moves: the new parent must exist, fit `check_parent`, and not be
/// the row itself or one of its descendants. `None` means the command may go on to `apply_lww`.
fn check_tree(
    conn: &Connection,
    existing: Option<&Row>,
    cmd: &Command,
) -> rusqlite::Result<Option<ErrorCode>> {
    if !matches!(
        cmd.op,
        Op::AreaCreate { .. }
            | Op::ProjectCreate { .. }
            | Op::SectionCreate { .. }
            | Op::TaskCreate { .. }
            | Op::ItemMove { .. }
    ) {
        return Ok(None);
    }
    // Other errors (exists, not found, blank title) come from `apply_lww` itself.
    let Ok(candidate) = apply(existing.cloned(), cmd) else {
        return Ok(None);
    };
    let parent = match candidate.parent_id() {
        Some(pid) => load_row(conn, &pid.to_string())?,
        None => None,
    };
    if let Err(code) = check_parent(&candidate, parent.as_ref()) {
        return Ok(Some(code));
    }
    // Walk up from the new parent; meeting the row itself would make a cycle.
    let mut next = candidate.parent_id();
    while let Some(pid) = next {
        if pid == candidate.id() {
            return Ok(Some(ErrorCode::BadParent));
        }
        next = load_row(conn, &pid.to_string())?.and_then(|r| r.parent_id());
    }
    Ok(None)
}

/// Completes every open Task under `id`, at any depth, as the same command.
fn complete_descendants(
    conn: &Connection,
    id: orbit_core::Uuid,
    cmd: &Command,
) -> rusqlite::Result<()> {
    let ids: Vec<String> = {
        let mut stmt = conn.prepare(
            "WITH RECURSIVE d(id) AS (
                 SELECT id FROM rows WHERE json_extract(data, '$.parent_id') = ?1
                 UNION
                 SELECT r.id FROM rows r JOIN d ON json_extract(r.data, '$.parent_id') = d.id
             )
             SELECT id FROM rows WHERE id IN (SELECT id FROM d)
               AND json_extract(data, '$.kind') = 'task'
               AND json_extract(data, '$.completed_at') IS NULL",
        )?;
        stmt.query_map([id.to_string()], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    for child in ids {
        let Some((data, clocks)) = load(conn, &child)? else {
            continue;
        };
        let row: Row = serde_json::from_str(&data).map_err(json_err)?;
        let mut clocks: Clocks = serde_json::from_str(&clocks).map_err(json_err)?;
        let done = Command {
            op: Op::TaskComplete {
                id: row.id(),
                today: match &cmd.op {
                    Op::TaskComplete { today, .. } => today.clone(),
                    _ => None,
                },
            },
            ..cmd.clone()
        };
        let old = row.clone();
        if let Ok(row) = apply_lww(Some(row), &mut clocks, &done) {
            write(conn, &row, &clocks)?;
            if let Row::Task(old) = old {
                log_completion(conn, old, &done)?;
            }
        }
    }
    Ok(())
}

/// Stores the completion record that completing `old` under `cmd` produces, if any.
fn log_completion(conn: &Connection, old: orbit_core::Task, cmd: &Command) -> rusqlite::Result<()> {
    if let (_, Some(c)) = orbit_core::complete_effects(old, cmd) {
        write(conn, &Row::Completion(c), &Clocks::new())?;
    }
    Ok(())
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

pub(crate) fn json_err(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(e.into())
}
