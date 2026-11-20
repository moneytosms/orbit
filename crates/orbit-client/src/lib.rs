//! Sync client: a SQLite replica (rows, outbox, meta) and `POST /sync`. Shared by Tauri and the CLI.
//! Every write takes `now` so callers (and tests) own the clock.
use orbit_core::{
    Command, ErrorCode, Hlc, Op, PROTOCOL_V, Row, Status, SyncRequest, SyncResponse, Task, Uuid,
    apply, new_id,
};
use rusqlite::{Connection, OptionalExtension};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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
            http: reqwest::Client::new(),
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
        let cmd = Command {
            uuid: new_id(now_ms, &random()?),
            op,
            hlc: clock(&tx, now_ms)?.next(now_ms),
        };
        predict(&tx, &cmd)?;
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

    /// Sends the outbox in batches of 100 and merges each delta. On a network or HTTP error
    /// (401, 426, ...) the outbox stays queued and the error is returned.
    pub async fn sync(&mut self, now_ms: u64) -> Result<Vec<Rejection>> {
        let mut rejected = Vec::new();
        loop {
            let commands = outbox(&self.db, 100)?;
            let token = meta(&self.db, "token")?.map(|t| t.parse()).transpose()?;
            let req = SyncRequest {
                v: PROTOCOL_V,
                token,
                commands,
            };
            let mut post = self.http.post(format!("{}/sync", self.url)).json(&req);
            if let Some(token) = meta(&self.db, "auth")? {
                post = post.bearer_auth(token);
            }
            let res: SyncResponse = post.send().await?.error_for_status()?.json().await?;

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
                tx.execute("DELETE FROM rows", [])?;
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
            // A partly answered batch would be resent as-is: stop rather than loop.
            if req.commands.len() < 100 || answered < req.commands.len() {
                return Ok(rejected);
            }
        }
    }
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
    let new =
        apply(existing, cmd).map_err(|code| format!("rejected: {}", serde_json::json!(code)))?;
    put(db, &new)
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
