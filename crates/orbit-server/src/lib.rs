use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use orbit_core::{
    Clocks, Command, ErrorCode, Hlc, Op, PROTOCOL_V, Row, Status, SyncRequest, SyncResponse,
    VersionError, apply, apply_lww, check_parent, check_version,
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

mod auth;
mod blobs;
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
    let setup = auth::setup_code(&conn)?;
    let max_blob = blobs::max_bytes();
    let shared = Shared {
        db: Mutex::new(conn),
        data: data_dir.to_path_buf(),
        max_blob,
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
        .route(
            "/blobs/{hash}",
            put(blobs::put)
                .get(blobs::get)
                .layer(axum::extract::DefaultBodyLimit::max(max_blob)),
        )
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
    run(&mut conn, &st.data, req).map(Json).map_err(|e| {
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
    let mut stmt = conn.prepare(
        "SELECT data FROM rows WHERE json_extract(data, '$.kind') != 'purged' ORDER BY id",
    )?;
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

fn run(conn: &mut Connection, data: &Path, req: SyncRequest) -> rusqlite::Result<SyncResponse> {
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
    let since = req
        .token
        .filter(|&t| t <= max as u64 && t >= purge_seq as u64)
        .unwrap_or(0);
    let full = req.token.is_some() && since == 0 && req.token != Some(0);
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
    let status = match if archived {
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
                    Op::TaskCreate { .. } | Op::TaskUpdate { .. } | Op::TaskComplete { .. }
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
