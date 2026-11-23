//! Sync client: a SQLite replica (rows, outbox, meta) and `POST /sync`. Shared by Tauri and the CLI.
//! Every write takes `now` so callers (and tests) own the clock.
use orbit_core::{
    Command, ErrorCode, Hlc, Op, PROTOCOL_V, Row, Status, SyncRequest, SyncResponse, Task,
    UpgradeRequired, Uuid, VersionInfo, apply, completion_for, new_id, parse,
    section_delete_effects,
};
use rusqlite::{Connection, OptionalExtension};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const GRAPH_CAPABILITY: &str = "x-orbit-graph";
const GRAPH_CURSOR: &str = "x-orbit-graph-cursor";

pub struct Client {
    db: Connection,
    url: String,
    http: reqwest::Client,
}

/// A command the server refused. It is gone from the outbox and its row is back to the server's.
#[derive(Debug)]
pub struct Rejection {
    pub command: Command,
    pub code: ErrorCode,
}

#[cfg(not(target_os = "android"))]
fn http() -> Result<reqwest::Client> {
    Ok(reqwest::Client::new())
}

/// Android: reqwest's default platform verifier aborts unless initialized over JNI with the
/// Android Context and its Kotlin AAR bundled (#89), so trust the bundled Mozilla roots instead.
#[cfg(target_os = "android")]
fn http() -> Result<reqwest::Client> {
    let roots = webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter();
    let certs = roots
        .map(|c| reqwest::Certificate::from_der(c))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(reqwest::Client::builder().tls_certs_only(certs).build()?)
}

impl Client {
    pub fn open(db: Connection, url: &str) -> Result<Client> {
        // Outbox rowid order is enqueue order, which is this Device's HLC order.
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS rows (id TEXT PRIMARY KEY, data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS outbox (uuid TEXT PRIMARY KEY, command TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        Ok(Client {
            db,
            url: url.trim_end_matches('/').into(),
            http: http()?,
        })
    }

    /// Logs in as a new Device. Its id becomes this replica's HLC device; the outbox is kept,
    /// so changes made while signed out go out on the next sync. Fails with the server's error
    /// code, e.g. `totp` when a code is needed.
    pub async fn login(
        &mut self,
        password: &str,
        totp: Option<&str>,
        name: &str,
        platform: &str,
    ) -> Result<Uuid> {
        let body = serde_json::json!({ "password": password, "totp": totp, "name": name, "platform": platform });
        let res = self
            .http
            .post(format!("{}/auth/login", self.url))
            .json(&body)
            .send()
            .await?;
        let status = res.status();
        let res: serde_json::Value = res.json().await.unwrap_or_default();
        if !status.is_success() {
            // The server's code (`credentials`, `totp`, `rate_limited`, ...) tells the caller what to ask for.
            return Err(res["error"].as_str().unwrap_or(status.as_str()).into());
        }
        let (Some(device), Some(token)) = (res["device"].as_str(), res["token"].as_str()) else {
            return Err(format!("unexpected login response: {res}").into());
        };
        let device = Uuid::parse_str(device)?;
        let hlc = match meta(&self.db, "hlc")? {
            Some(s) => Hlc {
                device,
                ..serde_json::from_str(&s)?
            },
            None => Hlc {
                wall: 0,
                counter: 0,
                device,
            },
        };
        let tx = self.db.transaction()?;
        set_meta(&tx, "hlc", &serde_json::to_string(&hlc)?)?;
        set_meta(&tx, "auth", token)?;
        tx.commit()?;
        Ok(device)
    }

    /// Queues an Inbox task from quick-add text: title, due, priority, labels and `@Area`/`@project`
    /// applied (`now_local` is `YYYY-MM-DDTHH:MM`). Goal and page tokens are parsed but not applied
    /// yet. Returns the id and the parsed title; blank or token-only text is an error.
    pub fn capture(
        &mut self,
        text: &str,
        id: Option<Uuid>,
        now_local: &str,
        now_ms: u64,
    ) -> Result<(Uuid, String)> {
        let rows = self.rows()?;
        let mut known = parse::Known::default();
        for r in &rows {
            match r {
                Row::Area(a) => known.areas.push(a.name.clone()),
                Row::Project(p) => known.areas.push(p.name.clone()),
                Row::Goal(g) => known.goals.push(g.name.clone()),
                Row::Page(p) => known.pages.push(p.title.clone()),
                Row::Label(l) => known.labels.push(l.name.clone()),
                _ => {}
            }
        }
        let parsed = parse::parse(text.trim(), now_local, &known);
        if parsed.title.is_empty() {
            return Err("empty capture".into());
        }
        let id = match id {
            Some(id) => {
                self.enqueue(
                    Op::TaskCreate {
                        id,
                        title: parsed.title.clone(),
                        parent_id: None,
                        order: String::new(),
                    },
                    now_ms,
                )?;
                id
            }
            None => self.create_task(&parsed.title, now_ms)?,
        };
        // An unknown `@name` leaves the task in the Inbox.
        let place = parsed.place.as_deref().and_then(|p| {
            rows.iter().find_map(|r| match r {
                Row::Area(a) if a.name.eq_ignore_ascii_case(p) => Some(a.id),
                Row::Project(x) if x.name.eq_ignore_ascii_case(p) => Some(x.id),
                _ => None,
            })
        });
        if let Some(parent_id) = place {
            self.enqueue(
                Op::ItemMove {
                    id,
                    parent_id: Some(parent_id),
                    order: String::new(),
                },
                now_ms,
            )?;
        }
        let label_ids: Vec<Uuid> = rows
            .iter()
            .filter_map(|r| match r {
                Row::Label(l) if parsed.labels.contains(&l.name) => Some(l.id),
                _ => None,
            })
            .collect();
        let mut patch = serde_json::Map::new();
        if let Some(d) = &parsed.due {
            patch.insert(
                "due".into(),
                serde_json::json!({ "date": d.date, "time": d.time }),
            );
        }
        if let Some(p) = parsed.priority {
            patch.insert("priority".into(), serde_json::json!(p));
        }
        if !label_ids.is_empty() {
            patch.insert("label_ids".into(), serde_json::json!(label_ids));
        }
        if !parsed.description.is_empty() {
            patch.insert("description".into(), serde_json::json!(parsed.description));
        }
        if !patch.is_empty() {
            self.update_task(id, serde_json::Value::Object(patch), now_ms)?;
        }
        Ok((id, parsed.title))
    }

    pub fn create_task(&mut self, title: &str, now_ms: u64) -> Result<Uuid> {
        let id = new_id(now_ms, &random()?);
        self.enqueue(
            Op::TaskCreate {
                id,
                title: title.into(),
                parent_id: None,
                order: String::new(),
            },
            now_ms,
        )?;
        Ok(id)
    }

    pub fn create_area(&mut self, name: &str, order: &str, now_ms: u64) -> Result<Uuid> {
        let id = new_id(now_ms, &random()?);
        self.enqueue(
            Op::AreaCreate {
                color: String::new(),
                id,
                name: name.into(),
                order: order.into(),
            },
            now_ms,
        )?;
        Ok(id)
    }

    pub fn move_item(
        &mut self,
        id: Uuid,
        parent_id: Option<Uuid>,
        order: &str,
        now_ms: u64,
    ) -> Result<()> {
        self.enqueue(
            Op::ItemMove {
                id,
                parent_id,
                order: order.into(),
            },
            now_ms,
        )
        .map(drop)
    }

    pub fn rename_task(&mut self, id: Uuid, title: &str, now_ms: u64) -> Result<()> {
        self.update_task(id, serde_json::json!({ "title": title }), now_ms)
    }

    /// Queues a `task.update`. `patch` holds the fields to change; `null` clears one.
    pub fn update_task(&mut self, id: Uuid, patch: serde_json::Value, now_ms: u64) -> Result<()> {
        let mut args = patch;
        args["id"] = serde_json::json!(id);
        let op =
            serde_json::from_value(serde_json::json!({ "type": "task.update", "args": args }))?;
        self.enqueue(op, now_ms).map(drop)
    }

    pub fn complete_task(&mut self, id: Uuid, now_ms: u64) -> Result<()> {
        self.enqueue(Op::TaskComplete { id, today: None }, now_ms)
            .map(drop)
    }

    /// Stamps, predicts and queues one command, atomically. Fails without queueing if core rejects it.
    pub fn enqueue(&mut self, op: Op, now_ms: u64) -> Result<Command> {
        let tx = self.db.transaction()?;
        let mut op = op;
        if let Op::TaskUncomplete {
            id,
            completion_hlc: Some(completion),
            restore_due: Some(_),
            ..
        } = &op
            && newer_due_update(&tx, *id, completion)?
            && let Op::TaskUncomplete { restore_due, .. } = &mut op
        {
            *restore_due = None;
        }
        let cmd = Command {
            uuid: new_id(now_ms, &random()?),
            op,
            hlc: clock(&tx, now_ms)?.next(now_ms),
        };
        predict(&tx, &cmd)?;
        if let Op::TaskUncomplete {
            completion_id: Some(id),
            ..
        } = &cmd.op
        {
            tx.execute("DELETE FROM rows WHERE id = ?", [id.to_string()])?;
        }
        set_meta(&tx, "hlc", &serde_json::to_string(&cmd.hlc)?)?;
        tx.execute(
            "INSERT INTO outbox (uuid, command) VALUES (?1, ?2)",
            [cmd.uuid.to_string(), serde_json::to_string(&cmd)?],
        )?;
        tx.commit()?;
        Ok(cmd)
    }

    pub fn task(&self, id: Uuid) -> Result<Option<Task>> {
        Ok(match row(&self.db, id)? {
            Some(Row::Task(t)) => Some(t),
            _ => None,
        })
    }

    /// Any local row by id.
    pub fn get_row(&self, id: Uuid) -> Result<Option<Row>> {
        row(&self.db, id)
    }

    /// Every local row, for pure computations over the whole replica.
    pub fn rows(&self) -> Result<Vec<Row>> {
        let mut st = self.db.prepare("SELECT data FROM rows")?;
        let data = st.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = vec![];
        for d in data {
            out.push(serde_json::from_str(&d?)?);
        }
        Ok(out)
    }

    /// Uses a Device made elsewhere (e.g. `POST /devices`) instead of logging in: its id becomes
    /// this replica's HLC device and its token authorizes syncs.
    pub fn set_auth(&mut self, device: Uuid, token: &str) -> Result<()> {
        let hlc = match meta(&self.db, "hlc")? {
            Some(s) => Hlc {
                device,
                ..serde_json::from_str(&s)?
            },
            None => Hlc {
                wall: 0,
                counter: 0,
                device,
            },
        };
        let tx = self.db.transaction()?;
        set_meta(&tx, "hlc", &serde_json::to_string(&hlc)?)?;
        set_meta(&tx, "auth", token)?;
        tx.commit()?;
        Ok(())
    }

    /// True once a Device has logged in or been given a token.
    pub fn is_signed_in(&self) -> Result<bool> {
        Ok(meta(&self.db, "auth")?.is_some())
    }

    /// Forgets the auth token so syncs stop authorizing. Rows and the outbox stay for the next login.
    pub fn logout(&mut self) -> Result<()> {
        self.db.execute("DELETE FROM meta WHERE key = 'auth'", [])?;
        Ok(())
    }

    pub fn outbox_len(&self) -> Result<usize> {
        let n: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// Calls a server endpoint as this Device (POST with `body`, else GET) and returns its JSON body
    /// (`Null` if none). Other than a 401 (see `is_signed_out`), a failure is the server's `error`
    /// code or the HTTP status.
    pub async fn call(
        &self,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let url = format!("{}{path}", self.url);
        let mut req = match &body {
            Some(body) => self.http.post(url).json(body),
            None => self.http.get(url),
        };
        if let Some(token) = meta(&self.db, "auth")? {
            req = req.bearer_auth(token);
        }
        let res = req.send().await?;
        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(res.error_for_status().unwrap_err().into());
        }
        let status = res.status();
        let body: serde_json::Value = res.json().await.unwrap_or_default();
        if !status.is_success() {
            return Err(body["error"].as_str().unwrap_or(status.as_str()).into());
        }
        Ok(body)
    }

    /// GETs a server endpoint as this Device and returns the raw body. A 401 is `is_signed_out`;
    /// any other failure is the HTTP status.
    pub async fn download(&self, path: &str) -> Result<Vec<u8>> {
        let mut req = self.http.get(format!("{}{path}", self.url));
        if let Some(token) = meta(&self.db, "auth")? {
            req = req.bearer_auth(token);
        }
        let res = req.send().await?.error_for_status()?;
        Ok(res.bytes().await?.to_vec())
    }

    /// Reads the public server and compatibility contract without authenticating.
    pub async fn server_version(&self) -> Result<VersionInfo> {
        Ok(self
            .http
            .get(format!("{}/version", self.url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Sends the outbox in batches of 100 and merges each delta. On a network or HTTP error
    /// (401, 426, ...) the outbox stays queued and the error is returned.
    pub async fn sync(&mut self, now_ms: u64) -> Result<Vec<Rejection>> {
        let mut rejected = Vec::new();
        let mut graph_supported = false;
        loop {
            let commands: Vec<_> = outbox(&self.db, usize::MAX)?
                .into_iter()
                .filter(|cmd| graph_supported || !is_graph_op(&cmd.op))
                .take(100)
                .collect();
            let token = meta(&self.db, "token")?.map(|t| t.parse()).transpose()?;
            let previous_graph_cursor = meta(&self.db, "graph_cursor")?;
            let req = SyncRequest {
                v: PROTOCOL_V,
                token,
                commands,
            };
            let mut post = self
                .http
                .post(format!("{}/sync", self.url))
                .header(GRAPH_CAPABILITY, "v1")
                .json(&req);
            if let Some(cursor) = previous_graph_cursor.as_deref() {
                post = post.header(GRAPH_CURSOR, cursor);
            }
            if let Some(token) = meta(&self.db, "auth")? {
                post = post.bearer_auth(token);
            }
            let res = post.send().await?;
            if res.status() == reqwest::StatusCode::UPGRADE_REQUIRED
                && let Err(http_error) = res.error_for_status_ref()
            {
                if let Ok(error) = res.json::<UpgradeRequired>().await {
                    return Err(Box::new(error));
                }
                return Err(Box::new(http_error));
            }
            let response_graph_cursor = res
                .headers()
                .get(GRAPH_CURSOR)
                .and_then(|v| v.to_str().ok())
                .filter(|v| v.starts_with("v1:"))
                .map(str::to_owned);
            graph_supported = response_graph_cursor.is_some();
            let res: SyncResponse = res.error_for_status()?.json().await?;

            let tx = self.db.transaction()?;
            let mut answered = 0;
            for cmd in req.commands.iter() {
                match res.statuses.get(&cmd.uuid) {
                    None => continue, // not answered: keep it queued
                    Some(Status::Ok) => answered += 1,
                    Some(Status::Error { code }) => {
                        // Undo the guess; the delta brings back the server's row if it has one.
                        tx.execute(
                            "DELETE FROM rows WHERE id = ?",
                            [cmd.op.target().to_string()],
                        )?;
                        rejected.push(Rejection {
                            command: cmd.clone(),
                            code: *code,
                        });
                    }
                }
                tx.execute("DELETE FROM outbox WHERE uuid = ?", [cmd.uuid.to_string()])?;
            }
            if res.full {
                if previous_graph_cursor.is_some() && response_graph_cursor.is_none() {
                    tx.execute(
                        "DELETE FROM rows WHERE json_extract(data, '$.kind') NOT IN ('graph_node', 'graph_edge')",
                        [],
                    )?;
                } else {
                    tx.execute("DELETE FROM rows", [])?;
                }
            }
            for r in &res.delta {
                match r {
                    Row::Purged(p) => {
                        tx.execute("DELETE FROM rows WHERE id = ?", [p.id.to_string()])?;
                    }
                    _ => put(&tx, r)?,
                }
            }
            set_meta(&tx, "token", &res.token.to_string())?;
            if let Some(cursor) = &response_graph_cursor {
                set_meta(&tx, "graph_cursor", cursor)?;
            } else {
                tx.execute("DELETE FROM meta WHERE key = 'graph_cursor'", [])?;
            }
            if let Some(seen) = &res.clock {
                let observed = clock(&tx, now_ms)?.observe(seen, now_ms);
                set_meta(&tx, "hlc", &serde_json::to_string(&observed)?)?;
            }
            // Re-predict what is still queued on top of the server's rows.
            // Plain `apply`, not per-field clocks: an older queued edit can show over a newer
            // server value until it is sent, after which the server's row comes back.
            for cmd in outbox(&tx, usize::MAX)? {
                let _ = predict(&tx, &cmd);
            }
            tx.commit()?;
            if answered < req.commands.len() {
                return Ok(rejected);
            }
            let remaining = outbox(&self.db, usize::MAX)?;
            if req.commands.len() == 100 {
                continue;
            }
            if graph_supported && remaining.iter().any(|cmd| is_graph_op(&cmd.op)) {
                continue;
            }
            return Ok(rejected);
        }
    }
}

/// A queued same-value due edit is still newer than the completion being undone.
fn newer_due_update(db: &Connection, id: Uuid, completion: &Hlc) -> Result<bool> {
    let mut stmt = db.prepare("SELECT command FROM outbox")?;
    let commands = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for command in commands {
        let cmd: Command = serde_json::from_str(&command?)?;
        if &cmd.hlc > completion
            && matches!(&cmd.op, Op::TaskUpdate { id: target, due: Some(_), .. } if *target == id)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// True if `e` is a 401: the Device was revoked or never logged in. Log in again to send the outbox.
pub fn is_signed_out(e: &(dyn std::error::Error + 'static)) -> bool {
    e.downcast_ref::<reqwest::Error>()
        .and_then(reqwest::Error::status)
        == Some(reqwest::StatusCode::UNAUTHORIZED)
}

/// This Device's last clock reading. Before the first login the Device id is a random placeholder.
fn clock(db: &Connection, now_ms: u64) -> Result<Hlc> {
    match meta(db, "hlc")? {
        Some(s) => Ok(serde_json::from_str(&s)?),
        None => Ok(Hlc {
            wall: 0,
            counter: 0,
            device: new_id(now_ms, &random()?),
        }),
    }
}

fn predict(db: &Connection, cmd: &Command) -> Result<()> {
    let existing = row(db, cmd.op.target())?;
    let root_completion = match &cmd.op {
        Op::TaskUncomplete {
            completion_id: Some(id),
            ..
        } => match row(db, *id)? {
            Some(Row::Completion(c)) => Some(c),
            _ => None,
        },
        _ => None,
    };
    if let Some(old) = &existing
        && let Some(completion) = completion_for(old, cmd)
    {
        put(db, &Row::Completion(completion))?;
    }
    let new =
        apply(existing, cmd).map_err(|code| format!("rejected: {}", serde_json::json!(code)))?;
    put(db, &new)?;
    match (&cmd.op, &new) {
        (Op::TaskComplete { id, .. }, Row::Task(task)) if task.completed_at.is_some() => {
            complete_descendants(db, *id, cmd)?;
        }
        (
            Op::TaskUncomplete {
                completion_id: Some(id),
                ..
            },
            _,
        ) => {
            db.execute("DELETE FROM rows WHERE id = ?", [id.to_string()])?;
            if let Some(root) = root_completion {
                uncomplete_descendants(db, &root, cmd)?;
            }
        }
        _ => {}
    }
    if matches!(&cmd.op, Op::ItemDelete { .. })
        && let Row::Section(section) = &new
        && section.trashed_at.is_some()
    {
        let rows = all_rows(db)?;
        for effect in section_delete_effects(&rows, section.id, cmd) {
            let Some(existing) = row(db, effect.op.target())? else {
                continue;
            };
            let moved = apply(Some(existing), &effect)
                .map_err(|code| format!("rejected: {}", serde_json::json!(code)))?;
            put(db, &moved)?;
        }
    }
    Ok(())
}

fn all_rows(db: &Connection) -> Result<Vec<Row>> {
    let mut stmt = db.prepare("SELECT data FROM rows")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
}

fn complete_descendants(db: &Connection, root: Uuid, cmd: &Command) -> Result<()> {
    let rows = all_rows(db)?;
    let tasks: Vec<Task> = rows
        .into_iter()
        .filter_map(|r| match r {
            Row::Task(t) => Some(t),
            _ => None,
        })
        .collect();
    let mut pending = vec![root];
    let mut seen = std::collections::HashSet::new();
    while let Some(parent) = pending.pop() {
        for task in tasks
            .iter()
            .filter(|t| t.parent_id == parent && seen.insert(t.id))
        {
            pending.push(task.id);
            if task.completed_at.is_some() {
                continue;
            }
            let done = Command {
                op: Op::TaskComplete {
                    id: task.id,
                    today: match &cmd.op {
                        Op::TaskComplete { today, .. } => today.clone(),
                        _ => None,
                    },
                },
                ..cmd.clone()
            };
            if let Some(completion) = completion_for(&Row::Task(task.clone()), &done) {
                put(db, &Row::Completion(completion))?;
            }
            let row = apply(Some(Row::Task(task.clone())), &done)
                .map_err(|code| format!("rejected: {}", serde_json::json!(code)))?;
            put(db, &row)?;
        }
    }
    Ok(())
}

fn uncomplete_descendants(
    db: &Connection,
    root: &orbit_core::Completion,
    cmd: &Command,
) -> Result<()> {
    let Some(source_id) = root.source_id else {
        return Ok(());
    };
    let completions: Vec<orbit_core::Completion> = all_rows(db)?
        .into_iter()
        .filter_map(|r| match r {
            Row::Completion(c) if c.source_id == Some(source_id) && c.id != root.id => Some(c),
            _ => None,
        })
        .collect();
    for completion in completions {
        let Some(existing) = row(db, completion.item_id)? else {
            continue;
        };
        if !matches!(&existing, Row::Task(_)) {
            continue;
        }
        let preserve_due = match completion.completion_hlc.as_ref() {
            Some(hlc) => newer_due_update(db, completion.item_id, hlc)?,
            None => false,
        };
        let op = Op::TaskUncomplete {
            id: completion.item_id,
            completion_id: Some(completion.id),
            completed_at: if completion.expected_due.is_some() {
                None
            } else {
                Some(completion.at)
            },
            completion_hlc: completion.completion_hlc.clone(),
            expected_due: completion.expected_due.clone(),
            restore_due: if preserve_due {
                None
            } else {
                completion.restore_due.clone()
            },
        };
        let undo = Command { op, ..cmd.clone() };
        if let Ok(row) = apply(Some(existing), &undo) {
            put(db, &row)?;
        }
        db.execute("DELETE FROM rows WHERE id = ?", [completion.id.to_string()])?;
    }
    Ok(())
}

fn is_graph_op(op: &Op) -> bool {
    matches!(
        op,
        Op::RoadmapNodeCreate { .. }
            | Op::RoadmapNodeUpdate { .. }
            | Op::RoadmapNodeMove { .. }
            | Op::RoadmapNodeDecide { .. }
            | Op::RoadmapEdgeCreate { .. }
            | Op::RoadmapEdgeUpdate { .. }
            | Op::RoadmapEdgeDelete { .. }
    )
}

fn outbox(db: &Connection, limit: usize) -> Result<Vec<Command>> {
    let mut stmt = db.prepare("SELECT command FROM outbox ORDER BY rowid LIMIT ?")?;
    let rows = stmt.query_map([limit.min(i64::MAX as usize) as i64], |r| {
        r.get::<_, String>(0)
    })?;
    rows.map(|c| Ok(serde_json::from_str(&c?)?)).collect()
}

fn row(db: &Connection, id: Uuid) -> Result<Option<Row>> {
    let data: Option<String> = db
        .query_row(
            "SELECT data FROM rows WHERE id = ?",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    Ok(data.map(|d| serde_json::from_str(&d)).transpose()?)
}

fn put(db: &Connection, row: &Row) -> Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO rows (id, data) VALUES (?1, ?2)",
        [row.id().to_string(), serde_json::to_string(row)?],
    )?;
    Ok(())
}

fn meta(db: &Connection, key: &str) -> Result<Option<String>> {
    Ok(db
        .query_row("SELECT value FROM meta WHERE key = ?", [key], |r| r.get(0))
        .optional()?)
}

fn set_meta(db: &Connection, key: &str, value: &str) -> Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        [key, value],
    )?;
    Ok(())
}

fn random() -> Result<[u8; 10]> {
    let mut b = [0; 10];
    getrandom::fill(&mut b)?;
    Ok(b)
}
