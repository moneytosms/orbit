use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use orbit_core::{
    Clocks, Command, ErrorCode, Hlc, MIN_CLIENT_PROTOCOL, Op, PROTOCOL_V, Row, Status, SyncRequest,
    SyncResponse, UpgradeRequired, VersionError, VersionInfo, apply, apply_lww, check_parent,
    check_version,
    trash::{self, KEEP_PURGED_MS, KEEP_TRASH_MS},
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
const GRAPH_CAPABILITY: &str = "x-orbit-graph";
const GRAPH_CURSOR: &str = "x-orbit-graph-cursor";
const GRAPH_CURSOR_V1: &str = "v1:";
const JS_SAFE_INTEGER_MAX: u64 = 9_007_199_254_740_991;
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
// Release builds set these so `/version` describes immutable build inputs.
const SERVER_COMMIT: &str = match option_env!("ORBIT_COMMIT") {
    Some(commit) => commit,
    None => "unknown",
};
const UI_BUILD_HASH: &str = match option_env!("ORBIT_UI_HASH") {
    Some(hash) => hash,
    None => "unknown",
};
const MIN_SHELL_VERSION: &str = match option_env!("ORBIT_MIN_SHELL_VERSION") {
    Some(version) => version,
    None => "0.0.0",
};
const UPGRADE_URL: &str = "https://github.com/moneytosms/orbit/releases/latest";

mod auth;
pub mod backup;
mod blobs;
mod todoist;
pub use auth::reset_password;

pub(crate) struct Shared {
    // shortcut: one connection behind a mutex, queries run on the async thread; move to spawn_blocking/pool when load matters.
    db: Mutex<Connection>,
    /// The data dir, for free-space checks.
    data: PathBuf,
    /// Per-file blob size limit in bytes.
    max_blob: usize,
    /// The one-time first-run code; `None` once the user exists.
    setup: Mutex<Option<String>>,
    limits: Mutex<HashMap<std::net::IpAddr, auth::Limit>>,
    /// The Todoist API v1 base URL.
    todoist: String,
    jobs: Mutex<HashMap<String, todoist::Job>>,
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
    let conn = Connection::open(data_dir.join("orbit.db"))?;
    // WAL: commits stop fsyncing the whole journal. Backups use VACUUM INTO, so `-wal`/`-shm` are safe.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(conn)
}

/// Opens (or creates) `<data_dir>/orbit.db` and returns the HTTP app, plus the setup code
/// to print when no user exists yet. Serve it with `into_make_service_with_connect_info::<SocketAddr>`.
pub fn app(data_dir: &Path) -> rusqlite::Result<(Router, Option<String>)> {
    app_with(data_dir, todoist::BASE)
}

/// `app` reading Todoist at `todoist_base` instead of the real API. For tests.
pub fn app_with(data_dir: &Path, todoist_base: &str) -> rusqlite::Result<(Router, Option<String>)> {
    let mut conn = open(data_dir)?;
    auth::migrate(&conn)?;
    todoist::migrate(&conn)?;
    // `clocks` is the row's per-field last-writer HLCs (JSON); it never leaves the server.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rows (id TEXT PRIMARY KEY, data TEXT NOT NULL, clocks TEXT NOT NULL, seq INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS rows_seq ON rows (seq);
         CREATE TABLE IF NOT EXISTS processed (uuid TEXT PRIMARY KEY, status TEXT NOT NULL, at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);",
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
    migrate_legacy_roadmaps(&mut conn)?;
    let setup = auth::setup_code(&conn)?;
    let max_blob = blobs::max_bytes();
    let shared = Shared {
        db: Mutex::new(conn),
        data: data_dir.to_path_buf(),
        max_blob,
        setup: Mutex::new(setup.clone()),
        limits: Mutex::default(),
        todoist: todoist_base.into(),
        jobs: Mutex::default(),
    };
    let router = Router::new()
        .route("/version", get(version))
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
        .route("/import/todoist", post(todoist::start))
        .route("/import/todoist/{job}", get(todoist::status))
        .route("/import/todoist/{job}/commit", post(todoist::commit))
        .route(
            "/blobs/{hash}",
            put(blobs::put)
                .get(blobs::get)
                .layer(axum::extract::DefaultBodyLimit::max(max_blob)),
        )
        .with_state(Arc::new(shared));
    Ok((router, setup))
}

async fn version() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "public, max-age=300")],
        Json(VersionInfo {
            server: SERVER_VERSION.into(),
            protocol: PROTOCOL_V,
            min_client_protocol: MIN_CLIENT_PROTOCOL,
            min_shell_version: MIN_SHELL_VERSION.into(),
            built: SERVER_COMMIT.into(),
            ui: UI_BUILD_HASH.into(),
        }),
    )
}

/// Serves the built web app from `dir` for every path the API does not claim; unknown paths get
/// `index.html` so client-side routes load. The Android app loads the UI from the server.
pub fn with_web(router: Router, dir: &Path) -> Router {
    use tower_http::services::{ServeDir, ServeFile};
    router.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html"))))
}

async fn sync(
    State(st): State<St>,
    auth::Device(device_id): auth::Device,
    headers: HeaderMap,
    Json(req): Json<SyncRequest>,
) -> Response {
    match check_version(PROTOCOL_V, req.v) {
        Err(VersionError::TooOld) => {
            return (
                StatusCode::UPGRADE_REQUIRED,
                Json(UpgradeRequired {
                    error: "upgrade_required".into(),
                    client_protocol: req.v,
                    min_client_protocol: MIN_CLIENT_PROTOCOL,
                    upgrade: UPGRADE_URL.into(),
                }),
            )
                .into_response();
        }
        Err(VersionError::TooNew) => return StatusCode::BAD_REQUEST.into_response(),
        Ok(()) => {}
    }
    if req.commands.len() > 100 {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    let mut conn = lock(&st.db);
    let graph = headers.get(GRAPH_CAPABILITY).and_then(|v| v.to_str().ok()) == Some("v1");
    let graph_cursor = graph
        .then(|| headers.get(GRAPH_CURSOR).and_then(|v| v.to_str().ok()))
        .flatten()
        .and_then(|v| v.strip_prefix(GRAPH_CURSOR_V1))
        .and_then(|v| v.parse::<u64>().ok());
    // Debug-only latency trace (#14); off unless ORBIT_SYNC_TRACE=1.
    let trace = std::env::var("ORBIT_SYNC_TRACE").is_ok_and(|value| value == "1");
    let received_at = trace.then(now_ms);
    let targets = trace.then(|| {
        req.commands
            .iter()
            .map(|command| (command.uuid, command.op.target()))
            .collect::<Vec<_>>()
    });
    match run(&mut conn, &st.data, req, graph, graph_cursor) {
        Ok(response) => {
            if let Some(targets) = targets {
                let applied: Vec<_> = targets
                    .iter()
                    .filter(|(uuid, _)| matches!(response.statuses.get(uuid), Some(Status::Ok)))
                    .map(|(_, target)| target.to_string())
                    .collect();
                let delta: Vec<_> = response
                    .delta
                    .iter()
                    .map(|row| row.id().to_string())
                    .collect();
                eprintln!(
                    "sync_trace {}",
                    serde_json::json!({
                        "device": device_id,
                        "received_at": received_at,
                        "applied_at": now_ms(),
                        "commands": applied,
                        "delta": delta,
                    })
                );
            }
            let graph_token = response.token;
            let mut response = Json(response).into_response();
            if graph
                && graph_token <= JS_SAFE_INTEGER_MAX
                && let Ok(value) = HeaderValue::from_str(&format!("{GRAPH_CURSOR_V1}{graph_token}"))
            {
                response.headers_mut().insert(GRAPH_CURSOR, value);
            }
            response
        }
        Err(e) => {
            eprintln!("sync failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// A zip of every row's data as `items.json`, each live Page as `pages/<name>.md` and every
/// blob live rows reference as `blobs/<hash>`.
async fn export(State(st): State<St>, _: auth::Device) -> Result<Response, StatusCode> {
    let zipped = build_export(&st).map_err(|e| {
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

fn build_export(st: &St) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (items, pages, hashes) = {
        let conn = lock(&st.db);
        (
            items_json(&conn)?,
            live_pages(&conn)?,
            live_blob_hashes(&conn)?,
        )
    };
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("items.json", options)?;
    zip.write_all(&items)?;
    let mut used = std::collections::HashSet::new();
    for (id, title, body) in pages {
        let slug = slug(&title);
        let name = if slug.is_empty() || used.contains(&slug) {
            id
        } else {
            slug
        };
        used.insert(name.clone());
        zip.start_file(format!("pages/{name}.md"), options)?;
        zip.write_all(body.as_bytes())?;
    }
    for hash in hashes {
        if !blobs::exists(&st.data, &hash) {
            continue;
        }
        zip.start_file(format!("blobs/{hash}"), options)?;
        zip.write_all(&std::fs::read(blobs::path(&st.data, &hash))?)?;
    }
    Ok(zip.finish()?.into_inner())
}

/// `(id, title, body)` of every Page not in the Trash.
fn live_pages(conn: &Connection) -> rusqlite::Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT json_extract(data, '$.id'), json_extract(data, '$.title'),
                COALESCE(json_extract(data, '$.body'), '')
         FROM rows
         WHERE json_extract(data, '$.kind') = 'page'
           AND json_extract(data, '$.trashed_at') IS NULL
         ORDER BY id",
    )?;
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect()
}

/// Every blob hash a live row references.
fn live_blob_hashes(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT j.value FROM rows, json_each(rows.data, '$.blobs') j
         WHERE json_extract(rows.data, '$.kind') != 'purged' ORDER BY j.value",
    )?;
    stmt.query_map([], |r| r.get(0))?.collect()
}

/// A file-name-safe lowercase form of `title`, empty if nothing is left.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(60).collect()
}

/// Every row's `data` as a JSON array, ordered by id.
fn items_json(conn: &Connection) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT data FROM rows WHERE json_extract(data, '$.kind') != 'purged' ORDER BY id",
    )?;
    let mut items = Vec::new();
    for data in stmt.query_map([], |r| r.get::<_, String>(0))? {
        items.push(serde_json::from_str::<serde_json::Value>(&data?)?);
    }
    Ok(serde_json::to_vec(&items)?)
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

fn run(
    conn: &mut Connection,
    data: &Path,
    req: SyncRequest,
    graph: bool,
    graph_cursor: Option<u64>,
) -> rusqlite::Result<SyncResponse> {
    let now = now_ms();
    conn.execute(
        "DELETE FROM processed WHERE at < ?",
        [now - KEEP_PROCESSED_MS],
    )?;
    let mut statuses = BTreeMap::new();
    let mut rejected = Vec::new();
    for cmd in &req.commands {
        let status = run_command(conn, data, cmd, now)?;
        if status != Status::Ok {
            rejected.push(cmd.op.target().to_string());
        }
        statuses.insert(cmd.uuid, status);
    }
    purge(conn, now)?;
    // shortcut: walks the blobs dir on every sync, move to a timed task if blob counts grow large
    blobs::gc(conn, data, now)?;
    let purge_seq = purge_seq(conn)?;
    let max = max_seq(conn)?;
    // A token from before a data reset, or from before purged tombstones were dropped, can't be
    // trusted: send everything.
    let (since, full) = if graph {
        match graph_cursor
            .filter(|&t| t <= JS_SAFE_INTEGER_MAX && t <= max as u64 && t >= purge_seq as u64)
        {
            Some(t) => (t, false),
            None => (0, true),
        }
    } else {
        let since = req
            .token
            .filter(|&t| t <= max as u64 && t >= purge_seq as u64)
            .unwrap_or(0);
        let full = req.token.is_some() && since == 0 && req.token != Some(0);
        (since, full)
    };
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
    if !graph {
        rows.retain(|(_, (data, _))| {
            !matches!(
                serde_json::from_str::<Row>(data),
                Ok(Row::GraphNode(_) | Row::GraphEdge(_))
            )
        });
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
        full,
    })
}

/// The highest seq among live rows and dropped tombstones, so seqs never go back.
fn max_seq(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT MAX(COALESCE((SELECT MAX(seq) FROM rows), 0), COALESCE((SELECT value FROM meta WHERE key = 'purge_seq'), 0))",
        [],
        |r| r.get(0),
    )
}

/// The highest seq of a tombstone that was dropped; older tokens can't see those deletions.
fn purge_seq(conn: &Connection) -> rusqlite::Result<i64> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = 'purge_seq'", [], |r| {
            r.get(0)
        })
        .optional()?
        .unwrap_or(0))
}

/// Runs `purge` against the data dir at `now_ms` (ms since the epoch). For tests.
pub fn purge_at(data_dir: &Path, now_ms: i64) -> rusqlite::Result<()> {
    let mut conn = open(data_dir)?;
    purge(&mut conn, now_ms)?;
    blobs::gc(&conn, data_dir, now_ms)
}

/// Drops tombstones older than 90 days, then turns rows trashed over 30 days ago, and
/// everything under them, into `{id, purged}` tombstones.
fn purge(conn: &mut Connection, now: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let old_tombstones =
        "json_extract(data, '$.kind') = 'purged' AND json_extract(data, '$.purged_at') < ?1";
    let dropped: i64 = tx.query_row(
        &format!("SELECT COALESCE(MAX(seq), 0) FROM rows WHERE {old_tombstones}"),
        [now - KEEP_PURGED_MS as i64],
        |r| r.get(0),
    )?;
    if dropped > 0 {
        let keep = dropped.max(purge_seq(&tx)?);
        tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('purge_seq', ?1)",
            [keep],
        )?;
        tx.execute(
            &format!("DELETE FROM rows WHERE {old_tombstones}"),
            [now - KEEP_PURGED_MS as i64],
        )?;
    }
    // Milestones hang under their Goal by `goal_id`; for Tasks and Projects `goal_id` is only a link.
    let ids: Vec<String> = {
        let mut stmt = tx.prepare(
            "WITH RECURSIVE d(id) AS (
                 SELECT id FROM rows
                  WHERE json_extract(data, '$.trashed_at') < ?1
                 UNION
                 SELECT r.id FROM rows r JOIN d ON
                     json_extract(r.data, '$.parent_id') = d.id
                     OR json_extract(r.data, '$.roadmap_id') = d.id
                     OR (json_extract(r.data, '$.kind') = 'milestone'
                         AND json_extract(r.data, '$.goal_id') = d.id)
             )
             SELECT id FROM rows WHERE id IN (SELECT id FROM d)
               AND json_extract(data, '$.kind') != 'purged'",
        )?;
        stmt.query_map([now - KEEP_TRASH_MS as i64], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    for id in ids {
        let id = orbit_core::Uuid::parse_str(&id)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        let tombstone = Row::Purged(orbit_core::Purged {
            id,
            purged_at: now as u64,
        });
        write(&tx, &tombstone, &Clocks::new())?;
    }
    tx.commit()
}

/// One transaction per command; a rejected command doesn't stop the batch.
fn run_command(
    conn: &mut Connection,
    data: &Path,
    cmd: &Command,
    now: i64,
) -> rusqlite::Result<Status> {
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
    // Not recorded as processed: the client uploads the blob and sends the same command again.
    if blobs::refs(cmd).iter().any(|h| !blobs::exists(data, h)) {
        return Ok(Status::Error {
            code: ErrorCode::MissingBlob,
        });
    }
    let (existing, mut clocks) = match load(&tx, &cmd.op.target().to_string())? {
        Some((data, clocks)) => (
            Some(serde_json::from_str(&data).map_err(json_err)?),
            serde_json::from_str(&clocks).map_err(json_err)?,
        ),
        None => (None, Clocks::new()),
    };
    // Chains from the target, and from the parent a create or move puts it under.
    let lookup = |id: orbit_core::Uuid| load_row(&tx, &id.to_string()).ok().flatten();
    let own = existing
        .as_ref()
        .map(|r| trash::chain(r, &lookup))
        .unwrap_or_default();
    let under = cmd
        .op
        .new_parent()
        .and_then(&lookup)
        .map(|p| trash::chain(&p, &lookup))
        .unwrap_or_default();
    let archived = !matches!(cmd.op, Op::ItemUnarchive { .. })
        && (trash::any_archived(&own) || trash::any_archived(&under));
    // A create under a trashed parent: a Task goes to the Inbox, anything else is born trashed.
    let (adjusted, born_trashed) = match trash::top_trashed(&under) {
        Some(at) if cmd.op.is_create() => trash::adjust_create(cmd, at),
        _ => (cmd.clone(), None),
    };
    let cmd = &adjusted;
    let stale_undo = match &cmd.op {
        Op::TaskUncomplete {
            id,
            completion_id: Some(completion_id),
            completion_hlc,
            ..
        } => !matches!(load_row(&tx, &completion_id.to_string())?, Some(Row::Completion(c))
        if c.item_id == *id
            && completion_hlc.as_ref().is_none_or(|h| {
                c.completion_hlc.as_ref().is_none_or(|stored| stored == h)
            })),
        _ => false,
    };
    let status = if stale_undo {
        Status::Ok
    } else {
        match if archived {
            Some(ErrorCode::Archived)
        } else {
            check_tree(&tx, existing.as_ref(), cmd)?
        } {
            Some(code) => Status::Error { code },
            None => match apply_lww(existing.clone(), &mut clocks, cmd) {
                Ok(mut row) => {
                    if born_trashed.is_some() {
                        row.set_trashed(born_trashed);
                    }
                    write(&tx, &row, &clocks)?;
                    if let Row::Section(section) = &row
                        && section.trashed_at.is_some()
                        && matches!(&cmd.op, Op::ItemDelete { .. })
                    {
                        reparent_section_tasks(&tx, section.id, cmd)?;
                    }
                    if let (Some(Row::Page(old)), Op::PageEditBody { body, base_rev, .. }) =
                        (&existing, &cmd.op)
                        && orbit_core::page::is_conflict(old, *base_rev)
                    {
                        save_conflict(&tx, old, body, cmd)?;
                    }
                    if let Some(old) = &existing
                        && matches!(
                            &cmd.op,
                            Op::GoalComplete { .. } | Op::MilestoneUpdate { .. }
                        )
                    {
                        log_completion(&tx, old, cmd)?;
                    }
                    if matches!(
                        &cmd.op,
                        Op::TaskCreate { .. }
                            | Op::TaskUpdate { .. }
                            | Op::TaskComplete { .. }
                            | Op::TaskUncomplete { .. }
                    ) {
                        touch_project(&tx, &row, cmd.hlc.wall)?;
                    }
                    if let Op::TaskComplete { id, .. } = &cmd.op {
                        if let Some(old) = &existing {
                            log_completion(&tx, old, cmd)?;
                        }
                        // A recurring Task only moves on; its subtasks stay as they are.
                        if matches!(&row, Row::Task(t) if t.completed_at.is_some()) {
                            complete_descendants(&tx, *id, cmd)?;
                        }
                    }
                    if let Op::TaskUncomplete {
                        id,
                        completion_id: Some(completion_id),
                        ..
                    } = &cmd.op
                    {
                        let completion = match load_row(&tx, &completion_id.to_string())? {
                            Some(Row::Completion(c)) if c.item_id == *id => Some(c),
                            _ => None,
                        };
                        purge_completion(&tx, *completion_id, cmd.hlc.wall)?;
                        if let Some(completion) = completion {
                            uncomplete_descendants(&tx, &completion, cmd)?;
                        }
                    }
                    award_badges(&tx, cmd, now)?;
                    Status::Ok
                }
                Err(code) => Status::Error { code },
            },
        }
    };
    let stored = serde_json::to_string(&status).map_err(json_err)?;
    tx.execute(
        "INSERT INTO processed (uuid, status, at) VALUES (?1, ?2, ?3)",
        (&uuid, stored, now),
    )?;
    tx.commit()?;
    Ok(status)
}

/// Removes one exact completion from every replica by replacing it with the standard tombstone.
fn purge_completion(conn: &Connection, id: orbit_core::Uuid, wall: u64) -> rusqlite::Result<()> {
    if matches!(load_row(conn, &id.to_string())?, Some(Row::Completion(_))) {
        write(
            conn,
            &Row::Purged(orbit_core::Purged {
                id,
                purged_at: wall,
            }),
            &Clocks::new(),
        )?;
    }
    Ok(())
}

/// Reverses only records created by the same completion command, preserving newer writes.
fn uncomplete_descendants(
    conn: &Connection,
    root: &orbit_core::Completion,
    cmd: &Command,
) -> rusqlite::Result<()> {
    let Some(source_id) = root.source_id else {
        return Ok(());
    };
    let completions: Vec<orbit_core::Completion> = all_rows(conn)?
        .into_iter()
        .filter_map(|row| match row {
            Row::Completion(c) if c.source_id == Some(source_id) && c.id != root.id => Some(c),
            _ => None,
        })
        .collect();
    for completion in completions {
        purge_completion(conn, completion.id, cmd.hlc.wall)?;
        let Some((data, clocks)) = load(conn, &completion.item_id.to_string())? else {
            continue;
        };
        let old: Row = serde_json::from_str(&data).map_err(json_err)?;
        let mut clocks: Clocks = serde_json::from_str(&clocks).map_err(json_err)?;
        if !matches!(&old, Row::Task(_)) {
            continue;
        }
        let expected_completed_at = if completion.expected_due.is_some() {
            None
        } else {
            Some(completion.at)
        };
        let undo = Command {
            op: Op::TaskUncomplete {
                id: completion.item_id,
                completion_id: Some(completion.id),
                completed_at: expected_completed_at,
                completion_hlc: completion.completion_hlc.clone(),
                expected_due: completion.expected_due.clone(),
                restore_due: completion.restore_due.clone(),
            },
            ..cmd.clone()
        };
        if let Ok(row) = apply_lww(Some(old), &mut clocks, &undo) {
            write(conn, &row, &clocks)?;
        }
    }
    Ok(())
}

/// Bump `touched_at` on the nearest Project above `row`, keeping its clocks.
fn touch_project(conn: &Connection, row: &Row, wall: u64) -> rusqlite::Result<()> {
    let mut next = row.parent_id();
    // The depth cap guards against a parent cycle.
    for _ in 0..32 {
        let Some(id) = next else { return Ok(()) };
        let Some((data, clocks)) = load(conn, &id.to_string())? else {
            return Ok(());
        };
        let parent: Row = serde_json::from_str(&data).map_err(json_err)?;
        if let Row::Project(mut p) = parent {
            p.touched_at = p.touched_at.max(Some(wall));
            let clocks: Clocks = serde_json::from_str(&clocks).map_err(json_err)?;
            return write(conn, &Row::Project(p), &clocks);
        }
        next = parent.parent_id();
    }
    Ok(())
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
            | Op::GoalCreate { .. }
            | Op::MilestoneCreate { .. }
            | Op::PageCreate { .. }
            | Op::ItemMove { .. }
            | Op::RoutineCreate { .. }
            | Op::RoutineUpdate { .. }
            | Op::CheckinSet { .. }
            | Op::RoadmapCreate { .. }
            | Op::PhaseCreate { .. }
            | Op::PlacementSet { .. }
            | Op::PhaseUpdate { .. }
            | Op::RoadmapNodeCreate { .. }
            | Op::RoadmapNodeUpdate { .. }
            | Op::RoadmapNodeMove { .. }
            | Op::RoadmapNodeDecide { .. }
            | Op::RoadmapEdgeCreate { .. }
            | Op::RoadmapEdgeUpdate { .. }
            | Op::RoadmapEdgeDelete { .. }
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
    // A Routine links only to a Target Goal.
    if let Row::Routine(r) = &candidate
        && let Some(gid) = r.goal_id
    {
        let target = matches!(
            load_row(conn, &gid.to_string())?,
            Some(Row::Goal(g)) if matches!(g.progress, orbit_core::goal::Progress::Target { .. })
        );
        if !target {
            return Ok(Some(ErrorCode::BadParent));
        }
    }
    // A Placement points at a Goal, Project, Milestone or Task, and at a Phase of its Roadmap.
    if let Row::Placement(pl) = &candidate {
        let item = load_row(conn, &pl.item_id.to_string())?;
        if !item.as_ref().is_some_and(orbit_core::roadmap::is_placeable) {
            return Ok(Some(ErrorCode::BadParent));
        }
        if let Some(phase_id) = pl.phase_id {
            let phase = load_row(conn, &phase_id.to_string())?;
            if !matches!(phase, Some(Row::Phase(p)) if p.roadmap_id == pl.roadmap_id) {
                return Ok(Some(ErrorCode::BadParent));
            }
        }
    }
    if let Row::GraphNode(node) = &candidate {
        let ref_row = node
            .ref_id
            .map(|id| load_row(conn, &id.to_string()))
            .transpose()?
            .flatten();
        let valid_ref = match (node.kind, ref_row.as_ref()) {
            (orbit_core::roadmap::NodeKind::Task, Some(Row::Task(_)))
            | (orbit_core::roadmap::NodeKind::Project, Some(Row::Project(_)))
            | (orbit_core::roadmap::NodeKind::Goal, Some(Row::Goal(_)))
            | (orbit_core::roadmap::NodeKind::Milestone, Some(Row::Milestone(_))) => true,
            (
                orbit_core::roadmap::NodeKind::Decision
                | orbit_core::roadmap::NodeKind::Note
                | orbit_core::roadmap::NodeKind::Text
                | orbit_core::roadmap::NodeKind::Heading,
                None,
            ) => node.ref_id.is_none(),
            _ => false,
        };
        if !valid_ref {
            return Ok(Some(ErrorCode::BadParent));
        }
        if let Some(band_id) = node.band_id
            && !matches!(load_row(conn, &band_id.to_string())?, Some(Row::Phase(p)) if p.roadmap_id == node.roadmap_id)
        {
            return Ok(Some(ErrorCode::BadParent));
        }
        if let Some(edge_id) = node.taken_edge_id
            && !matches!(load_row(conn, &edge_id.to_string())?, Some(Row::GraphEdge(e)) if e.roadmap_id == node.roadmap_id && e.source_id == node.id)
        {
            return Ok(Some(ErrorCode::BadParent));
        }
    }
    if let Row::GraphEdge(edge) = &candidate {
        for id in [edge.source_id, edge.target_id] {
            if !matches!(load_row(conn, &id.to_string())?, Some(Row::GraphNode(n)) if n.roadmap_id == edge.roadmap_id)
            {
                return Ok(Some(ErrorCode::BadParent));
            }
        }
    }
    // A Phase's gate is a Milestone.
    if let Row::Phase(ph) = &candidate
        && let Some(gate) = ph.gate_id
        && !matches!(load_row(conn, &gate.to_string())?, Some(Row::Milestone(_)))
    {
        return Ok(Some(ErrorCode::BadParent));
    }
    // Sub-goals go one level deep, under a top-level Goal in an Area.
    if let (Row::Goal(_), Some(p @ Row::Goal(_))) = (&candidate, parent.as_ref()) {
        let grandparent = match p.parent_id() {
            Some(gid) => load_row(conn, &gid.to_string())?,
            None => None,
        };
        if let Err(code) = orbit_core::goal::check_nesting(p, grandparent.as_ref()) {
            return Ok(Some(code));
        }
        // A Goal with Sub-goals of its own can't become a Sub-goal.
        let has_subgoals: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM rows WHERE json_extract(data, '$.kind') = 'goal'
             AND json_extract(data, '$.parent_id') = ?1)",
            [candidate.id().to_string()],
            |r| r.get(0),
        )?;
        if has_subgoals {
            return Ok(Some(ErrorCode::BadParent));
        }
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
            log_completion(conn, &old, &done)?;
        }
    }
    Ok(())
}

/// Keeps the body of a stale `page.edit_body` as a sibling of `page`, titled with the device
/// that made it and the day.
fn save_conflict(
    conn: &Connection,
    page: &orbit_core::Page,
    body: &str,
    cmd: &Command,
) -> rusqlite::Result<()> {
    let device = cmd.hlc.device.to_string();
    let name: Option<String> = conn
        .query_row("SELECT name FROM devices WHERE id = ?", [&device], |r| {
            r.get(0)
        })
        .optional()?;
    let device = name.unwrap_or_else(|| device[..8].into());
    let date = orbit_core::page::utc_date(cmd.hlc.wall);
    // Right after the original, before whatever followed it.
    let next: Option<String> = conn
        .query_row(
            "SELECT MIN(json_extract(data, '$.order')) FROM rows
             WHERE json_extract(data, '$.kind') = 'page' AND json_extract(data, '$.parent_id') = ?1
               AND json_extract(data, '$.order') > ?2",
            (page.parent_id.to_string(), &page.order),
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    let create = Command {
        op: Op::PageCreate {
            id: orbit_core::page::conflict_id(page.id, cmd.uuid),
            parent_id: page.parent_id,
            title: orbit_core::page::conflict_title(&page.title, &device, &date),
            body: body.into(),
            order: orbit_core::order::between(Some(&page.order), next.as_deref()),
        },
        ..cmd.clone()
    };
    let mut clocks = Clocks::new();
    if let Ok(row) = apply_lww(None, &mut clocks, &create) {
        write(conn, &row, &clocks)?;
    }
    Ok(())
}

/// Stores the completion record that `cmd` produces on `old`, if any.
fn log_completion(conn: &Connection, old: &Row, cmd: &Command) -> rusqlite::Result<()> {
    if let Some(c) = orbit_core::completion_for(old, cmd) {
        write(conn, &Row::Completion(c), &Clocks::new())?;
    }
    Ok(())
}

/// Every live row, read inside `conn`'s transaction.
fn all_rows(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    let mut stmt =
        conn.prepare("SELECT data FROM rows WHERE json_extract(data, '$.kind') != 'purged'")?;
    let mut rows = Vec::new();
    for data in stmt.query_map([], |r| r.get::<_, String>(0))? {
        rows.push(serde_json::from_str(&data?).map_err(json_err)?);
    }
    Ok(rows)
}

/// Deleting a Section moves every direct Task, including completed or already trashed Tasks,
/// back to its Area or Project. Child moves use the delete's HLC so newer moves still win.
fn reparent_section_tasks(
    conn: &Connection,
    section_id: orbit_core::Uuid,
    cmd: &Command,
) -> rusqlite::Result<()> {
    let rows = all_rows(conn)?;
    for effect in orbit_core::section_delete_effects(&rows, section_id, cmd) {
        let Some((data, clocks)) = load(conn, &effect.op.target().to_string())? else {
            continue;
        };
        let old: Row = serde_json::from_str(&data).map_err(json_err)?;
        let old_clocks: Clocks = serde_json::from_str(&clocks).map_err(json_err)?;
        let mut clocks = old_clocks.clone();
        let Ok(row) = apply_lww(Some(old.clone()), &mut clocks, &effect) else {
            continue;
        };
        if row != old || clocks != old_clocks {
            write(conn, &row, &clocks)?;
        }
    }
    Ok(())
}

/// Materialize deterministic graph rows for old Placement data. Writes are idempotent so a
/// partial startup migration safely finishes on the next open.
fn migrate_legacy_roadmaps(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS roadmap_graph_migrations (id TEXT PRIMARY KEY)")?;
    let mut rows = all_rows(&tx)?;
    let mut row_ids: std::collections::BTreeSet<String> = {
        let mut stmt = tx.prepare("SELECT id FROM rows")?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    let node_candidates = orbit_core::roadmap::migrate_legacy_nodes(&rows);
    let placements: Vec<_> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Placement(p) => Some(p.clone()),
            _ => None,
        })
        .collect();
    for placement in placements {
        let id = orbit_core::roadmap::graph_node_id(placement.roadmap_id, placement.item_id);
        let key = id.to_string();
        if !matches!(rows.iter().find(|r| r.id() == placement.item_id), Some(item) if orbit_core::roadmap::is_placeable(item))
        {
            continue;
        }
        let marked: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM roadmap_graph_migrations WHERE id = ?1)",
            [&key],
            |r| r.get(0),
        )?;
        if marked {
            continue;
        }
        let already_represented = row_ids.contains(&key)
            || rows.iter().any(|r| matches!(r, Row::GraphNode(n) if n.roadmap_id == placement.roadmap_id && n.ref_id == Some(placement.item_id)));
        let candidate = node_candidates.iter().find(|n| n.id == id);
        if !already_represented && let Some(node) = candidate {
            let row = Row::GraphNode(node.clone());
            write(&tx, &row, &Clocks::new())?;
            rows.push(row);
            row_ids.insert(key.clone());
        }
        tx.execute(
            "INSERT OR IGNORE INTO roadmap_graph_migrations (id) VALUES (?1)",
            [&key],
        )?;
    }
    for edge in orbit_core::roadmap::migrate_legacy_edges(&rows) {
        let key = edge.id.to_string();
        let marked: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM roadmap_graph_migrations WHERE id = ?1)",
            [&key],
            |r| r.get(0),
        )?;
        if marked {
            continue;
        }
        let endpoints_live = [edge.source_id, edge.target_id].into_iter().all(|id| {
            rows.iter()
                .any(|r| matches!(r, Row::GraphNode(n) if n.id == id && n.trashed_at.is_none()))
        });
        if !row_ids.contains(&key) && endpoints_live {
            let row = Row::GraphEdge(edge);
            write(&tx, &row, &Clocks::new())?;
            row_ids.insert(key.clone());
        }
        tx.execute(
            "INSERT OR IGNORE INTO roadmap_graph_migrations (id) VALUES (?1)",
            [&key],
        )?;
    }
    tx.commit()
}

/// Open Inbox Tasks: not done and not in the Trash.
fn open_inbox(rows: &[Row]) -> usize {
    rows.iter()
        .filter(|r| {
            matches!(r, Row::Task(t) if t.parent_id == orbit_core::INBOX
                && t.completed_at.is_none()
                && t.trashed_at.is_none())
        })
        .count()
}

/// Writes the Awards `cmd` newly earns. The server has no time zone, so days are UTC.
fn award_badges(conn: &Connection, cmd: &Command, now: i64) -> rusqlite::Result<()> {
    if matches!(cmd.op, Op::AwardGrant { .. }) {
        return Ok(());
    }
    let rows = all_rows(conn)?;
    let emptied = match cmd.op {
        Op::TaskComplete { .. } if open_inbox(&rows) == 0 => Some(cmd.uuid),
        _ => None,
    };
    let now = now as u64;
    let today = orbit_core::page::utc_date(now);
    for a in orbit_core::badge::earned(&rows, now, 0, "", &today, emptied) {
        let grant = Command {
            op: Op::AwardGrant {
                id: a.id,
                badge: a.badge,
                key: a.key,
                earned_at: a.earned_at,
                points: a.points,
            },
            ..cmd.clone()
        };
        let mut clocks = Clocks::new();
        if let Ok(row) = apply_lww(None, &mut clocks, &grant) {
            write(conn, &row, &clocks)?;
        }
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
        "INSERT INTO rows (id, data, clocks, seq) VALUES (?1, ?2, ?3, (SELECT MAX(COALESCE((SELECT MAX(seq) FROM rows), 0), COALESCE((SELECT value FROM meta WHERE key = 'purge_seq'), 0)) + 1))
         ON CONFLICT (id) DO UPDATE SET data = excluded.data, clocks = excluded.clocks, seq = excluded.seq",
        [row.id().to_string(), data, clocks],
    )?;
    Ok(())
}

pub(crate) fn json_err(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(e.into())
}
