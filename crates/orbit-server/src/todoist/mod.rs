//! Todoist import: `POST /import/todoist` starts a job that reads the account with the pasted
//! token (used once, never stored), `GET` shows its preview, `POST .../commit` emits the commands.
use crate::{St, auth, lock};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

mod api;
mod commit;

pub use api::BASE;
pub(crate) use api::Snapshot;

pub(crate) fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS import_map (key TEXT PRIMARY KEY, id TEXT NOT NULL);",
    )
}

pub(crate) enum Job {
    Running,
    Ready(Arc<Snapshot>),
    Failed(String),
}

#[derive(Deserialize)]
pub(crate) struct Start {
    token: String,
}

pub(crate) async fn start(
    State(st): State<St>,
    _: auth::Device,
    Json(b): Json<Start>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let token = b.token.trim().to_string();
    if token.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut raw = [0u8; 8];
    getrandom::fill(&mut raw).expect("OS randomness");
    let id = hex::encode(raw);
    {
        // One import at a time: a new job replaces the old.
        let mut jobs = lock(&st.jobs);
        jobs.clear();
        jobs.insert(id.clone(), Job::Running);
    }
    let job = id.clone();
    tokio::spawn(async move {
        let done = match api::fetch(&st.todoist, &token, &st.data, st.max_blob).await {
            Ok(s) => Job::Ready(Arc::new(s)),
            Err(e) => Job::Failed(e),
        };
        if let Some(slot) = lock(&st.jobs).get_mut(&job) {
            *slot = done;
        }
    });
    Ok((StatusCode::ACCEPTED, Json(json!({ "job": id }))))
}

pub(crate) async fn status(
    State(st): State<St>,
    _: auth::Device,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let snap = match lock(&st.jobs).get(&id) {
        None => return Err(StatusCode::NOT_FOUND),
        Some(Job::Running) => return Ok(Json(json!({ "status": "running" }))),
        Some(Job::Failed(e)) => return Ok(Json(json!({ "status": "failed", "error": e }))),
        Some(Job::Ready(s)) => s.clone(),
    };
    let map = commit::load_map(&lock(&st.db)).map_err(internal)?;
    let comments = snap
        .task_comments
        .values()
        .chain(snap.project_comments.values());
    Ok(Json(json!({
        "status": "ready",
        "projects": commit::default_plans(&snap, &map),
        "totals": {
            "tasks": snap.tasks.len(),
            "sections": snap.sections.len(),
            "labels": snap.labels.len(),
            "reminders": snap.reminders.len(),
            "comments": comments.clone().map(Vec::len).sum::<usize>(),
            "attachments": comments.flatten().filter(|c| c.file.is_some()).count(),
        },
    })))
}

#[derive(Deserialize)]
pub(crate) struct Commit {
    /// The mapping as edited in the preview; projects left out keep the default.
    #[serde(default)]
    projects: Vec<commit::Plan>,
}

pub(crate) async fn commit(
    State(st): State<St>,
    _: auth::Device,
    Path(id): Path<String>,
    Json(b): Json<Commit>,
) -> Result<Json<commit::Report>, StatusCode> {
    let snap = match lock(&st.jobs).get(&id) {
        None => return Err(StatusCode::NOT_FOUND),
        Some(Job::Ready(s)) => s.clone(),
        Some(_) => return Err(StatusCode::CONFLICT),
    };
    // shortcut: runs on the async thread under the db lock, like `/sync`; a huge import stalls syncs meanwhile.
    let report = commit::run(&mut lock(&st.db), &st.data, &snap, &b.projects).map_err(internal)?;
    lock(&st.jobs).remove(&id);
    Ok(Json(report))
}

fn internal(e: rusqlite::Error) -> StatusCode {
    eprintln!("import failed: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}
