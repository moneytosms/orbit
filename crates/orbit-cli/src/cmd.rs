//! The commands. Each returns the human text and the `--json` body; `main` prints them.
use crate::out::{Done, Fail, R, fail, internal, table, usage};
use crate::refs;
use orbit_client::{Client, Rejection, is_signed_out};
use orbit_core::next::{Cfg, whats_next};
use orbit_core::routine::{CheckState, checkin_id};
use orbit_core::search::Index;
use orbit_core::view::today_string;
use orbit_core::{ErrorCode, Op, PROTOCOL_V, Row, Task, UpgradeRequired, Uuid};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::error::Error;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Rows `ls`, `next` and `search` show before truncating, unless `--all`.
const LIMIT: usize = 20;
const SERVER_FILE: &str = "server";

/// The user data dir: `ORBIT_DIR`, else `$XDG_DATA_HOME/orbit`, else `~/.local/share/orbit`.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ORBIT_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("orbit")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The replica in the data dir, with the server URL saved by `login` (empty before it).
struct Ctx {
    client: Client,
    dir: PathBuf,
    url: String,
}

fn open() -> R<Ctx> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(internal)?;
    let url = std::fs::read_to_string(dir.join(SERVER_FILE))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let db = Connection::open(dir.join("orbit.db")).map_err(internal)?;
    let client = Client::open(db, &url).map_err(internal)?;
    Ok(Ctx { client, dir, url })
}

fn runtime() -> R<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(internal)
}

fn short(id: Uuid) -> String {
    id.to_string()[..8].to_string()
}

/// The wire name of a core rejection, as scripts see it.
fn code_name(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::Exists => "exists",
        ErrorCode::NotFound => "not_found",
        ErrorCode::TitleRequired => "title_required",
        ErrorCode::BadParent => "bad_parent",
        ErrorCode::BadField => "bad_field",
        ErrorCode::Archived => "archived",
        ErrorCode::MissingBlob => "missing_blob",
    }
}

fn password() -> R<String> {
    if let Some(p) = std::env::var("ORBIT_PASSWORD")
        .ok()
        .filter(|p| !p.is_empty())
    {
        return Ok(p);
    }
    // Only a terminal gets a prompt; a pipe or a closed stdin is read as-is, never asked for.
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        eprint!("password: ");
        std::io::stderr().flush().ok();
    }
    let mut line = String::new();
    stdin.lock().read_line(&mut line).map_err(internal)?;
    let p = line.trim_end_matches(['\r', '\n']).to_string();
    if p.is_empty() {
        return Err(fail(
            "password_required",
            "no password given",
            Some("set ORBIT_PASSWORD or pipe the password on stdin"),
        ));
    }
    Ok(p)
}

fn login_error(e: Box<dyn Error + Send + Sync>) -> crate::out::Fail {
    let msg = e.to_string();
    match msg.as_str() {
        "totp" => fail("totp", msg, Some("pass the current code with --totp")),
        "credentials" => fail("credentials", msg, Some("check the password")),
        "rate_limited" => fail("rate_limited", msg, Some("wait a while, then try again")),
        _ => fail("login_failed", msg, Some("check the server URL")),
    }
}

pub fn login(url: &str, totp: Option<&str>, name: &str) -> Done {
    let url = url.trim().trim_end_matches('/');
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(usage(format!("`{url}` is not an http:// or https:// URL")));
    }
    let password = password()?;
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(internal)?;
    let db = Connection::open(dir.join("orbit.db")).map_err(internal)?;
    let mut client = Client::open(db, url).map_err(internal)?;
    let device = runtime()?
        .block_on(client.login(&password, totp, name, "cli"))
        .map_err(login_error)?;
    std::fs::write(dir.join(SERVER_FILE), url).map_err(internal)?;
    Ok((
        format!("logged in to {url} as device {}", short(device)),
        json!({ "server": url, "device": device }),
    ))
}

pub fn logout(yes: bool, dry_run: bool) -> Done {
    let mut ctx = open()?;
    if !ctx.client.is_signed_in().map_err(internal)? {
        return Ok((
            "not signed in".into(),
            json!({ "signed_in": false, "changed": false, "dry_run": dry_run }),
        ));
    }
    let pending = ctx.client.outbox_len().map_err(internal)?;
    if pending > 0 && !yes && !dry_run {
        return Err(fail(
            "confirm",
            format!("{pending} unsynced change(s) wait until you log in again"),
            Some("pass --yes to log out anyway"),
        ));
    }
    if dry_run {
        return Ok((
            "would log out".into(),
            json!({ "signed_in": true, "changed": false, "dry_run": true, "unsynced": pending }),
        ));
    }
    ctx.client.logout().map_err(internal)?;
    Ok((
        "logged out".into(),
        json!({ "signed_in": false, "changed": true, "dry_run": false, "unsynced": pending }),
    ))
}

pub fn sync() -> Done {
    let mut ctx = open()?;
    if ctx.url.is_empty() {
        return Err(fail(
            "no_server",
            "not logged in to a server",
            Some("run `orbit login <url>`"),
        ));
    }
    if !ctx.client.is_signed_in().map_err(internal)? {
        return Err(fail(
            "signed_out",
            "this device is signed out",
            Some("run `orbit login <url>`"),
        ));
    }
    let before = ctx.client.outbox_len().map_err(internal)?;
    let rejected = runtime()?
        .block_on(ctx.client.sync(now_ms()))
        .map_err(|e| sync_error(&*e))?;
    let after = ctx.client.outbox_len().map_err(internal)?;
    let sent = before.saturating_sub(after);
    let rejections: Vec<Value> = rejected.iter().map(rejection_json).collect();
    let mut human = format!("synced {sent} change(s)");
    for r in &rejected {
        human.push_str(&format!(
            "\nrejected {}: {}",
            r.command.uuid,
            code_name(r.code)
        ));
    }
    Ok((human, json!({ "sent": sent, "rejected": rejections })))
}

fn sync_error(e: &(dyn Error + Send + Sync + 'static)) -> Fail {
    if is_signed_out(e) {
        fail(
            "signed_out",
            "the server no longer accepts this device",
            Some("run `orbit login <url>`"),
        )
    } else if let Some(upgrade) = e.downcast_ref::<UpgradeRequired>() {
        upgrade_error(upgrade.client_protocol, upgrade.min_client_protocol)
    } else {
        fail(
            "network",
            e.to_string(),
            Some("changes stay queued; try again later"),
        )
    }
}

fn upgrade_error(client_protocol: u32, min_client_protocol: u32) -> Fail {
    fail(
        "upgrade_required",
        format!(
            "This Orbit CLI is too old for that server (client protocol {client_protocol}, server needs {min_client_protocol}). Run `orbit self-update`."
        ),
        Some("run `orbit self-update` after checking your network connection"),
    )
}

fn rejection_json(r: &Rejection) -> Value {
    let op = serde_json::to_value(&r.command.op).unwrap_or(Value::Null);
    json!({ "uuid": r.command.uuid, "type": op["type"], "code": code_name(r.code) })
}

pub fn status() -> Done {
    let ctx = open()?;
    let signed_in = ctx.client.is_signed_in().map_err(internal)?;
    let pending = ctx.client.outbox_len().map_err(internal)?;
    let open_tasks = open_tasks(&ctx.client)?.len();
    let server = (!ctx.url.is_empty()).then(|| ctx.url.clone());
    let versions = if ctx.url.is_empty() {
        None
    } else {
        runtime()?.block_on(async {
            tokio::time::timeout(Duration::from_secs(3), ctx.client.server_version())
                .await
                .ok()
                .and_then(Result::ok)
        })
    };
    let upgrade = versions
        .as_ref()
        .filter(|v| v.min_client_protocol > PROTOCOL_V);
    let message = upgrade.map(|v| upgrade_error(PROTOCOL_V, v.min_client_protocol));
    let status = if let Some(error) = &message {
        error.message.clone()
    } else if versions
        .as_ref()
        .is_some_and(|v| version_is_newer(&v.server, env!("CARGO_PKG_VERSION")))
    {
        "update available".into()
    } else if versions.is_some() {
        "up to date".into()
    } else {
        "server version unavailable".into()
    };
    let server_version = versions
        .as_ref()
        .map(|v| v.server.as_str())
        .unwrap_or("unavailable");
    let server_protocol = versions
        .as_ref()
        .map(|v| v.protocol.to_string())
        .unwrap_or_else(|| "unavailable".into());
    let human = format!(
        "client version: {}\nserver: {}\nserver version: {}\nprotocol: client {}, server {}\nupdate: {status}\nsigned in: {}\nunsynced changes: {pending}\nopen tasks: {open_tasks}",
        env!("CARGO_PKG_VERSION"),
        server.as_deref().unwrap_or("none"),
        server_version,
        PROTOCOL_V,
        server_protocol,
        if signed_in { "yes" } else { "no" },
    );
    Ok((
        human,
        json!({ "server": server, "signed_in": signed_in, "unsynced": pending, "open_tasks": open_tasks, "client_version": env!("CARGO_PKG_VERSION"), "server_version": versions.as_ref().map(|v| &v.server), "client_protocol": PROTOCOL_V, "server_protocol": versions.as_ref().map(|v| v.protocol), "update": status, "upgrade_required": message.map(|error| json!({ "code": error.code, "message": error.message, "hint": error.hint, "client_protocol": PROTOCOL_V, "min_client_protocol": upgrade.map(|v| v.min_client_protocol), "upgrade": "https://github.com/moneytosms/orbit/releases/latest" })) }),
    ))
}

fn version_is_newer(left: &str, right: &str) -> bool {
    fn parts(s: &str) -> Option<Vec<u64>> {
        let values: Vec<_> = s
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        (values.len() == 3).then_some(values)
    }
    matches!((parts(left), parts(right)), (Some(a), Some(b)) if a > b)
}

pub fn self_update(dry_run: bool) -> Done {
    crate::updates::self_update(dry_run)
}

pub fn export(file: &Path) -> Done {
    let ctx = open()?;
    if ctx.url.is_empty() {
        return Err(fail(
            "no_server",
            "not logged in to a server",
            Some("run `orbit login <url>`"),
        ));
    }
    if !ctx.client.is_signed_in().map_err(internal)? {
        return Err(fail(
            "signed_out",
            "this device is signed out",
            Some("run `orbit login <url>`"),
        ));
    }
    let bytes = runtime()?
        .block_on(ctx.client.download("/export"))
        .map_err(|e| {
            if is_signed_out(&*e) {
                fail(
                    "signed_out",
                    "the server no longer accepts this device",
                    Some("run `orbit login <url>`"),
                )
            } else {
                fail("network", e.to_string(), Some("try again later"))
            }
        })?;
    std::fs::write(file, &bytes).map_err(internal)?;
    let path = file.display().to_string();
    Ok((
        format!("wrote {path} ({} bytes)", bytes.len()),
        json!({ "path": path, "bytes": bytes.len() }),
    ))
}

/// Open (not done, not in the Trash) tasks, in list order.
fn open_tasks(client: &Client) -> R<Vec<Task>> {
    let mut tasks: Vec<Task> = client
        .rows()
        .map_err(internal)?
        .into_iter()
        .filter_map(|r| match r {
            Row::Task(t) if t.completed_at.is_none() && t.trashed_at.is_none() => Some(t),
            _ => None,
        })
        .collect();
    tasks.sort_by(|a, b| (a.order.as_str(), a.id).cmp(&(b.order.as_str(), b.id)));
    Ok(tasks)
}

struct Entry {
    id: Uuid,
    title: String,
    due: String,
    priority: String,
    context: String,
    description: String,
    reason: String,
    detail: String,
    done: bool,
}

struct ListingView<'a> {
    all: bool,
    empty: &'a str,
    wide: bool,
    no_color: bool,
    next: bool,
    all_ids: &'a [Uuid],
}

/// Prints a listing, saves it for ref lookups, and truncates to `LIMIT` unless `all`.
fn present(dir: &Path, entries: Vec<Entry>, view: ListingView<'_>) -> Done {
    let total = entries.len();
    let shown = if view.all { total } else { total.min(LIMIT) };
    let entries = &entries[..shown];
    let ids: Vec<Uuid> = entries.iter().map(|e| e.id).collect();
    refs::save_listing(dir, &ids)?;
    let truncated = shown < total;
    let mut human = if total == 0 {
        view.empty.to_string()
    } else {
        let rows: Vec<Vec<String>> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let mut row = vec![
                    (i + 1).to_string(),
                    e.title.clone(),
                    e.due.clone(),
                    e.priority.clone(),
                ];
                if view.next {
                    row.push(e.reason.clone());
                }
                row.push(e.context.clone());
                if view.wide {
                    row.extend([id_prefix(e.id, view.all_ids, shown), e.description.clone()]);
                }
                if !view.no_color {
                    if !e.due.is_empty() {
                        let due_date = e.due.get(..10).unwrap_or(&e.due);
                        let today = today_string((now_ms() / 86_400_000) as i64);
                        let color = if due_date < today.as_str() {
                            "\u{1b}[31m"
                        } else if due_date == today.as_str() {
                            "\u{1b}[33m"
                        } else {
                            ""
                        };
                        if !color.is_empty() {
                            row[2] = format!("{color}{}\u{1b}[0m", row[2]);
                        }
                    }
                    if view.next {
                        let color = if e.reason == "Overdue" {
                            "\u{1b}[31m"
                        } else if e.reason == "Due today" {
                            "\u{1b}[33m"
                        } else {
                            ""
                        };
                        if !color.is_empty() {
                            row[4] = format!("{color}{}\u{1b}[0m", row[4]);
                        }
                    }
                    if e.done {
                        for cell in &mut row {
                            *cell = format!("\u{1b}[2m{cell}\u{1b}[0m");
                        }
                    }
                    if !e.context.is_empty() {
                        let i = if view.next { 5 } else { 4 };
                        row[i] = format!("\u{1b}[2m{}\u{1b}[0m", row[i]);
                    }
                }
                row
            })
            .collect();
        let mut headers = vec!["#", "TITLE", "DUE", "P"];
        if view.next {
            headers.push("WHY");
        }
        headers.push("CONTEXT");
        if view.wide {
            headers.extend(["ID", "DESCRIPTION / SOURCE"]);
        }
        let term_width = std::env::var("COLUMNS")
            .ok()
            .and_then(|s| s.parse().ok())
            .or_else(|| {
                terminal_size::terminal_size().map(|(terminal_size::Width(w), _)| usize::from(w))
            })
            .unwrap_or(80);
        table(
            &headers,
            &rows,
            if view.wide {
                usize::MAX / 4
            } else {
                term_width
            },
        )
    };
    let mut body = json!({
        "items": entries.iter().enumerate().map(|(i, e)| json!({
            "n": i + 1, "id": e.id, "title": e.title, "detail": e.detail
        })).collect::<Vec<_>>(),
        "total": total,
        "shown": shown,
        "truncated": truncated,
    });
    if truncated {
        let hint = format!("showing {shown}/{total}, pass --all to see the rest");
        human.push_str(&format!("\n{hint}"));
        body["hint"] = json!(hint);
    }
    Ok((human, body))
}

fn task_entries(tasks: &[Task], client: &Client) -> Vec<Entry> {
    tasks
        .iter()
        .map(|t| Entry {
            id: t.id,
            title: t.title.clone(),
            due: t
                .due
                .as_ref()
                .map(|d| match &d.time {
                    Some(time) => format!("{} {time}", d.date),
                    None => d.date.clone(),
                })
                .unwrap_or_default(),
            priority: format!("P{}", t.priority),
            context: context(t, client),
            description: t.description.clone(),
            reason: String::new(),
            detail: detail(t),
            done: t.completed_at.is_some(),
        })
        .collect()
}

fn detail(t: &Task) -> String {
    let mut parts = vec![format!("P{}", t.priority)];
    if let Some(due) = &t.due {
        parts.push(match &due.time {
            Some(time) => format!("due {} {time}", due.date),
            None => format!("due {}", due.date),
        });
    }
    if t.completed_at.is_some() {
        parts.push("done".into());
    }
    parts.join(" ")
}

fn id_prefix(id: Uuid, ids: &[Uuid], row_count: usize) -> String {
    let full = id.to_string().replace('-', "");
    for n in 4..=full.len() {
        let prefix = &full[..n];
        let resolves_as_row = prefix
            .parse::<usize>()
            .is_ok_and(|n| n > 0 && n <= row_count);
        if !resolves_as_row
            && ids
                .iter()
                .filter(|other| other.to_string().replace('-', "").starts_with(prefix))
                .count()
                == 1
        {
            return prefix.to_string();
        }
    }
    full
}

fn context(task: &Task, client: &Client) -> String {
    let Ok(rows) = client.rows() else {
        return String::new();
    };
    let mut parent = task.parent_id;
    let mut names = vec![];
    for _ in 0..8 {
        let Some(row) = rows.iter().find(|r| r.id() == parent) else {
            break;
        };
        match row {
            Row::Area(a) => names.push(a.name.clone()),
            Row::Project(p) => names.push(p.name.clone()),
            _ => {}
        }
        let Some(next) = row.parent_id() else { break };
        parent = next;
    }
    names.reverse();
    names.join(" / ")
}

pub fn ls(all: bool, wide: bool, no_color: bool) -> Done {
    let ctx = open()?;
    let tasks = open_tasks(&ctx.client)?;
    let ids = ctx
        .client
        .rows()
        .map_err(internal)?
        .iter()
        .map(Row::id)
        .collect::<Vec<_>>();
    present(
        &ctx.dir,
        task_entries(&tasks, &ctx.client),
        ListingView {
            all,
            empty: "no tasks",
            wide,
            no_color,
            next: false,
            all_ids: &ids,
        },
    )
}

pub fn next(all: bool, wide: bool, no_color: bool) -> Done {
    let ctx = open()?;
    let now = now_ms();
    let rows = ctx.client.rows().map_err(internal)?;
    let today = today_string((now / 86_400_000) as i64);
    let hhmm = format!("{:02}:{:02}", (now / 3_600_000) % 24, (now / 60_000) % 60);
    let entries = whats_next(&rows, &today, &hhmm, &Cfg::default())
        .into_iter()
        .map(|item| Entry {
            id: item.id,
            title: item.title,
            due: String::new(),
            priority: String::new(),
            context: item.breadcrumb.join(" / "),
            description: rows
                .iter()
                .find(|row| row.id() == item.id)
                .and_then(|row| match row {
                    Row::Task(task) => Some(task.description.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            detail: item.reason.clone(),
            reason: item.reason,
            done: false,
        })
        .collect();
    let ids = rows.iter().map(Row::id).collect::<Vec<_>>();
    present(
        &ctx.dir,
        entries,
        ListingView {
            all,
            empty: "nothing to do next",
            wide,
            no_color,
            next: true,
            all_ids: &ids,
        },
    )
}

pub fn search(words: &[String], all: bool, wide: bool, no_color: bool) -> Done {
    let query = words.join(" ");
    if query.trim().is_empty() {
        return Err(usage("search needs some words"));
    }
    let ctx = open()?;
    let mut index = Index::new();
    for row in ctx.client.rows().map_err(internal)? {
        index.upsert(&row);
    }
    let mut entries = vec![];
    for hit in index.query(&query, now_ms(), 10_000) {
        if hit.kind != "task" {
            continue;
        }
        if let Some(t) = ctx.client.task(hit.id).map_err(internal)? {
            entries.push(Entry {
                id: t.id,
                title: t.title.clone(),
                due: t
                    .due
                    .as_ref()
                    .map(|d| match &d.time {
                        Some(time) => format!("{} {time}", d.date),
                        None => d.date.clone(),
                    })
                    .unwrap_or_default(),
                priority: format!("P{}", t.priority),
                context: context(&t, &ctx.client),
                description: t.description.clone(),
                reason: String::new(),
                detail: detail(&t),
                done: t.completed_at.is_some(),
            });
        }
    }
    let ids = ctx
        .client
        .rows()
        .map_err(internal)?
        .iter()
        .map(Row::id)
        .collect::<Vec<_>>();
    present(
        &ctx.dir,
        entries,
        ListingView {
            all,
            empty: &format!("no tasks match `{query}`"),
            wide,
            no_color,
            next: false,
            all_ids: &ids,
        },
    )
}

/// The Task a ref names, or a not_found failure.
fn task_of(ctx: &Ctx, r: &str) -> R<Task> {
    let id = refs::resolve(&ctx.dir, &ctx.client, r)?;
    ctx.client.task(id).map_err(internal)?.ok_or_else(|| {
        fail(
            "not_a_task",
            format!("`{r}` is not a task"),
            Some("run `orbit ls` to see tasks"),
        )
    })
}

pub fn add(words: &[String], id: Option<&str>, dry_run: bool) -> Done {
    let title = words.join(" ").trim().to_string();
    if title.is_empty() {
        return Err(fail(
            "title_required",
            "a task needs a title",
            Some("orbit add \"buy milk\""),
        ));
    }
    let id = match id {
        Some(s) => Some(Uuid::parse_str(s).map_err(|_| usage(format!("`{s}` is not a UUID")))?),
        None => None,
    };
    let mut ctx = open()?;
    if let Some(id) = id {
        // Idempotent: an id that already exists is the same task, so nothing is queued again.
        if ctx.client.get_row(id).map_err(internal)?.is_some() {
            return Ok((
                format!("already added {}", short(id)),
                json!({ "id": id, "title": title, "created": false, "dry_run": dry_run }),
            ));
        }
    }
    if dry_run {
        return Ok((
            format!("would add: {title}"),
            json!({ "id": id, "title": title, "created": false, "dry_run": true }),
        ));
    }
    let local = chrono::Local::now().format("%Y-%m-%dT%H:%M").to_string();
    let (id, title) = ctx
        .client
        .capture(&title, id, &local, now_ms())
        .map_err(|e| match e.to_string().as_str() {
            "empty capture" => fail(
                "title_required",
                "a task needs a title, not only quick-add tokens",
                Some("orbit add \"buy milk tomorrow p1\""),
            ),
            _ => internal(e),
        })?;
    Ok((
        {
            let ids = ctx
                .client
                .rows()
                .map_err(internal)?
                .iter()
                .map(Row::id)
                .collect::<Vec<_>>();
            let prefix = id_prefix(id, &ids, refs::listing_len(&ctx.dir)?);
            format!("Added {prefix} \"{title}\" · orbit done {prefix}")
        },
        json!({ "id": id, "title": title, "created": true, "dry_run": false }),
    ))
}

pub fn done(r: &str, dry_run: bool) -> Done {
    let mut ctx = open()?;
    let task = task_of(&ctx, r)?;
    if task.completed_at.is_some() {
        return Ok((
            format!("already done: {}", task.title),
            json!({ "id": task.id, "title": task.title, "changed": false, "dry_run": dry_run }),
        ));
    }
    if dry_run {
        return Ok((
            format!("would complete: {}", task.title),
            json!({ "id": task.id, "title": task.title, "changed": false, "dry_run": true }),
        ));
    }
    ctx.client
        .complete_task(task.id, now_ms())
        .map_err(internal)?;
    Ok((
        format!("done: {} · orbit ls", task.title),
        json!({ "id": task.id, "title": task.title, "changed": true, "dry_run": false }),
    ))
}

pub fn show(r: &str) -> Done {
    let ctx = open()?;
    let id = refs::resolve(&ctx.dir, &ctx.client, r)?;
    if let Some(Row::Section(section)) = ctx.client.get_row(id).map_err(internal)? {
        let human = format!("{}\nid: {}\nsection", section.name, section.id);
        let body = json!({ "section": serde_json::to_value(&section).map_err(internal)? });
        return Ok((human, body));
    }
    let task = task_of(&ctx, r)?;
    let status = if task.completed_at.is_some() {
        "done"
    } else {
        "open"
    };
    let mut human = format!(
        "{}\nid: {}\nstatus: {status}\npriority: P{}",
        task.title, task.id, task.priority
    );
    if let Some(due) = &task.due {
        human.push_str(&format!("\ndue: {}", due.date));
    }
    if !task.description.is_empty() {
        human.push_str(&format!("\n\n{}", task.description));
    }
    let body = json!({ "task": serde_json::to_value(&task).map_err(internal)? });
    Ok((human, body))
}

pub fn edit(r: &str, title: Option<&str>, description: Option<&str>, dry_run: bool) -> Done {
    if title.is_none() && description.is_none() {
        return Err(usage("nothing to edit: pass --title or --description"));
    }
    if title.is_some_and(|t| t.trim().is_empty()) {
        return Err(fail(
            "title_required",
            "a task needs a title",
            Some("pass a non-empty value with --title"),
        ));
    }
    let mut ctx = open()?;
    let task = task_of(&ctx, r)?;
    let mut patch = serde_json::Map::new();
    if let Some(t) = title {
        patch.insert("title".into(), json!(t));
    }
    if let Some(d) = description {
        patch.insert("description".into(), json!(d));
    }
    if dry_run {
        return Ok((
            format!("would edit: {}", task.title),
            json!({ "id": task.id, "changed": false, "dry_run": true }),
        ));
    }
    ctx.client
        .update_task(task.id, Value::Object(patch), now_ms())
        .map_err(internal)?;
    let ids = ctx
        .client
        .rows()
        .map_err(internal)?
        .iter()
        .map(Row::id)
        .collect::<Vec<_>>();
    let prefix = id_prefix(task.id, &ids, refs::listing_len(&ctx.dir)?);
    let human = format!("edited {prefix} · orbit show {prefix}");
    Ok((
        human,
        json!({ "id": task.id, "changed": true, "dry_run": false }),
    ))
}

pub fn checkin(r: &str, date: Option<&str>, dry_run: bool) -> Done {
    let mut ctx = open()?;
    let id = refs::resolve(&ctx.dir, &ctx.client, r)?;
    let routine = match ctx.client.get_row(id).map_err(internal)? {
        Some(Row::Routine(routine)) => routine,
        _ => {
            return Err(fail(
                "not_a_routine",
                format!("`{r}` is not a routine"),
                Some("run `orbit ls` and choose a routine"),
            ));
        }
    };
    let date = match date {
        Some(d) => d.to_string(),
        None => today_string((now_ms() / 86_400_000) as i64),
    };
    let shaped = date.len() == 10
        && date.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        });
    if !shaped {
        return Err(usage(format!("`{date}` is not a YYYY-MM-DD date")));
    }
    if dry_run {
        return Ok((
            format!("would check in {} for {date}", routine.name),
            json!({ "routine": routine.id, "name": routine.name, "date": date, "changed": false, "dry_run": true }),
        ));
    }
    let op = Op::CheckinSet {
        id: checkin_id(routine.id, &date),
        routine_id: routine.id,
        date: date.clone(),
        state: CheckState::Done,
    };
    ctx.client.enqueue(op, now_ms()).map_err(internal)?;
    Ok((
        format!("checked in {} for {date} · orbit ls", routine.name),
        json!({ "routine": routine.id, "name": routine.name, "date": date, "changed": true, "dry_run": false }),
    ))
}

/// `ID=area:NAME`, `ID=project:AREA/NAME` or `ID=inbox` as a project mapping.
fn parse_map(spec: &str) -> R<Value> {
    let bad = || {
        usage(format!(
            "`{spec}` is not ID=area:NAME, ID=project:AREA/NAME or ID=inbox"
        ))
    };
    let (id, target) = spec.split_once('=').ok_or_else(bad)?;
    let (kind, rest) = target.split_once(':').unwrap_or((target, ""));
    let (area, name) = match kind {
        "area" if !rest.is_empty() => (rest, ""),
        "project" => rest
            .split_once('/')
            .filter(|(a, n)| !a.is_empty() && !n.is_empty())
            .ok_or_else(bad)?,
        "inbox" if rest.is_empty() => ("", ""),
        _ => return Err(bad()),
    };
    Ok(json!({ "id": id, "kind": kind, "area": area, "name": name }))
}

fn todoist_token() -> R<String> {
    if let Some(t) = std::env::var("ORBIT_TODOIST_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
    {
        return Ok(t);
    }
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        eprint!("todoist token: ");
        std::io::stderr().flush().ok();
    }
    let mut line = String::new();
    stdin.lock().read_line(&mut line).map_err(internal)?;
    let t = line.trim().to_string();
    if t.is_empty() {
        return Err(fail(
            "token_required",
            "no Todoist token given",
            Some("set ORBIT_TODOIST_TOKEN or pipe the token on stdin"),
        ));
    }
    Ok(t)
}

fn target_text(p: &Value) -> String {
    let (area, name) = (
        p["area"].as_str().unwrap_or(""),
        p["name"].as_str().unwrap_or(""),
    );
    match p["kind"].as_str() {
        Some("area") => format!("Area {area}"),
        Some("project") => format!("Project {area} / {name}"),
        _ => "Inbox".into(),
    }
}

pub fn import_todoist(maps: &[String], yes: bool) -> Done {
    let maps = maps.iter().map(|m| parse_map(m)).collect::<R<Vec<_>>>()?;
    let ctx = open()?;
    if !ctx.client.is_signed_in().map_err(internal)? {
        return Err(fail(
            "signed_out",
            "this device is signed out",
            Some("run `orbit login <url>`"),
        ));
    }
    let token = todoist_token()?;
    let net = |e: Box<dyn Error + Send + Sync>| {
        if is_signed_out(&*e) {
            fail(
                "signed_out",
                "the server no longer accepts this device",
                Some("run `orbit login <url>`"),
            )
        } else {
            fail("import_failed", e.to_string(), Some("nothing was imported"))
        }
    };
    runtime()?.block_on(async {
        let job = ctx.client.call("/import/todoist", Some(json!({ "token": token }))).await.map_err(net)?;
        let path = format!("/import/todoist/{}", job["job"].as_str().unwrap_or_default());
        let mut preview = loop {
            let p = ctx.client.call(&path, None).await.map_err(net)?;
            match p["status"].as_str() {
                Some("running") => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                Some("ready") => break p,
                _ => {
                    let why = p["error"].as_str().unwrap_or("unknown error").to_string();
                    return Err(fail("todoist", why, Some("check the token; nothing was imported")));
                }
            }
        };
        for p in preview["projects"].as_array_mut().into_iter().flatten() {
            if let Some(m) = maps.iter().find(|m| m["id"] == p["id"]) {
                for f in ["kind", "area", "name"] {
                    p[f] = m[f].clone();
                }
            }
        }
        if !yes {
            let rows: Vec<Vec<String>> = preview["projects"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| {
                    vec![
                        p["id"].as_str().unwrap_or("").into(),
                        p["path"].as_str().unwrap_or("").into(),
                        target_text(p),
                        format!("{} ({} new)", p["tasks"], p["tasks"].as_u64().unwrap_or(0) - p["imported"].as_u64().unwrap_or(0)),
                    ]
                })
                .collect();
            let human = format!(
                "{}\npass --yes to import; change a target with --map ID=area:NAME",
                table(&["ID", "TODOIST", "BECOMES", "TASKS"], &rows, std::env::var("COLUMNS").ok().and_then(|s| s.parse().ok()).unwrap_or(80))
            );
            return Ok((human, json!({ "committed": false, "preview": preview })));
        }
        let report = ctx
            .client
            .call(&format!("{path}/commit"), Some(json!({ "projects": preview["projects"] })))
            .await
            .map_err(net)?;
        let mut human = format!(
            "imported {} tasks, {} sections, {} projects, {} areas, {} labels, {} pages, {} reminders ({} tasks were already imported)",
            report["tasks"], report["sections"], report["projects"], report["areas"], report["labels"],
            report["pages"], report["reminders"], report["existing_tasks"]
        );
        for issue in report["issues"].as_array().into_iter().flatten() {
            human.push_str(&format!("\nnot imported as-is: {}", issue.as_str().unwrap_or("")));
        }
        let dropped: Vec<&str> = report["dropped"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        human.push_str(&format!("\nnever imported: {}\nrun `orbit sync` to fetch the new items", dropped.join(", ")));
        Ok((human, json!({ "committed": true, "report": report })))
    })
}

#[cfg(test)]
mod update_tests {
    use super::*;

    #[test]
    fn protocol_error_is_actionable_and_nonzero() {
        let error = UpgradeRequired {
            error: "upgrade_required".into(),
            client_protocol: 1,
            min_client_protocol: 2,
            upgrade: "https://example.test/releases/latest".into(),
        };
        let failure = sync_error(&error);
        assert_eq!(failure.code, "upgrade_required");
        assert_eq!(failure.exit, 1);
        assert!(
            failure
                .message
                .contains("client protocol 1, server needs 2")
        );
        assert!(failure.message.contains("orbit self-update"));
    }

    #[test]
    fn status_compares_versions_numerically() {
        assert!(version_is_newer("0.10.0", "0.9.9"));
        assert!(!version_is_newer("0.2.0", "0.2.0"));
    }
}
