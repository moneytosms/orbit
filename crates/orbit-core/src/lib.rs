use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Builder;
pub use uuid::Uuid;

/// Hybrid logical clock. Field order is the comparison order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hlc {
    pub wall: u64,
    pub counter: u32,
    pub device: Uuid,
}

impl Hlc {
    /// The next local reading: never behind `now_ms`, always after `self`.
    pub fn next(&self, now_ms: u64) -> Hlc {
        let (wall, counter) = if now_ms > self.wall {
            (now_ms, 0)
        } else {
            (self.wall, self.counter + 1)
        };
        Hlc {
            wall,
            counter,
            device: self.device,
        }
    }

    /// The HLC receive step: after seeing `remote`, every later local reading sorts after it.
    pub fn observe(&self, remote: &Hlc, now_ms: u64) -> Hlc {
        let wall = now_ms.max(self.wall).max(remote.wall);
        let counter = match (wall == self.wall, wall == remote.wall) {
            (true, true) => self.counter.max(remote.counter) + 1,
            (true, false) => self.counter + 1,
            (false, true) => remote.counter + 1,
            (false, false) => 0,
        };
        Hlc {
            wall,
            counter,
            device: self.device,
        }
    }
}

/// The built-in Area every new Task lands in.
pub const INBOX: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0001);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Area {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Task {
    pub id: Uuid,
    pub area_id: Uuid,
    pub title: String,
    /// Unix ms.
    pub completed_at: Option<u64>,
}

/// A synced record, as stored by the server and sent in deltas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Row {
    Area(Area),
    Task(Task),
}

impl Row {
    pub fn id(&self) -> Uuid {
        match self {
            Row::Area(a) => a.id,
            Row::Task(t) => t.id,
        }
    }

    pub fn inbox() -> Row {
        Row::Area(Area {
            id: INBOX,
            name: "Inbox".into(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Command {
    pub uuid: Uuid,
    #[serde(flatten)]
    pub op: Op,
    pub hlc: Hlc,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "args")]
pub enum Op {
    #[serde(rename = "task.create")]
    TaskCreate { id: Uuid, title: String },
    /// Only the fields present change.
    #[serde(rename = "task.update")]
    TaskUpdate { id: Uuid, title: Option<String> },
    #[serde(rename = "task.complete")]
    TaskComplete { id: Uuid },
}

impl Op {
    /// The row this command writes.
    pub fn target(&self) -> Uuid {
        match self {
            Op::TaskCreate { id, .. } | Op::TaskUpdate { id, .. } | Op::TaskComplete { id } => *id,
        }
    }

    /// Row fields an edit writes, each guarded by its own clock. A create stamps all fields instead.
    fn fields(&self) -> Vec<&'static str> {
        match self {
            Op::TaskCreate { .. } => vec![],
            Op::TaskUpdate { title, .. } => title.iter().map(|_| "title").collect(),
            Op::TaskComplete { .. } => vec!["completed_at"],
        }
    }
}

/// Last-writer clock per field of one row. A missing clock is older than any command.
pub type Clocks = BTreeMap<String, Hlc>;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Exists,
    NotFound,
    TitleRequired,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Status {
    Ok,
    Error { code: ErrorCode },
}

/// Sync protocol version. The server also accepts the one before it.
pub const PROTOCOL_V: u32 = 1;

#[derive(Debug, PartialEq)]
pub enum VersionError {
    /// The client must update; it keeps its outbox (HTTP 426).
    TooOld,
    TooNew,
}

/// A server on `current` serves `current` and `current - 1`.
pub fn check_version(current: u32, v: u32) -> Result<(), VersionError> {
    match v {
        v if v > current => Err(VersionError::TooNew),
        v if v + 1 < current => Err(VersionError::TooOld),
        _ => Ok(()),
    }
}

/// `POST /sync` body. No token asks for a full snapshot.
#[derive(Serialize, Deserialize, Debug)]
pub struct SyncRequest {
    pub v: u32,
    pub token: Option<u64>,
    pub commands: Vec<Command>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SyncResponse {
    pub statuses: BTreeMap<Uuid, Status>,
    /// Full rows changed since the request token.
    pub delta: Vec<Row>,
    pub token: u64,
    /// Newest clock behind the delta rows; clients `observe` it so their next edits sort later.
    pub clock: Option<Hlc>,
}

/// Applies one command to the row it targets. Server and client prediction share this.
pub fn apply(existing: Option<Row>, cmd: &Command) -> Result<Row, ErrorCode> {
    match &cmd.op {
        Op::TaskCreate { id, title } => {
            let title = title.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if title.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else {
                Ok(Row::Task(Task {
                    id: *id,
                    area_id: INBOX,
                    title: title.into(),
                    completed_at: None,
                }))
            }
        }
        Op::TaskUpdate { title, .. } => match existing {
            Some(Row::Task(t)) => {
                let title = match title.as_deref().map(str::trim) {
                    Some("") => return Err(ErrorCode::TitleRequired),
                    Some(new) => new.into(),
                    None => t.title.clone(),
                };
                Ok(Row::Task(Task { title, ..t }))
            }
            _ => Err(ErrorCode::NotFound),
        },
        Op::TaskComplete { .. } => match existing {
            Some(Row::Task(t)) => Ok(Row::Task(Task {
                completed_at: t.completed_at.or(Some(cmd.hlc.wall)),
                ..t
            })),
            _ => Err(ErrorCode::NotFound),
        },
    }
}

/// `apply`, then per-field last-writer-wins: a field whose stored clock is newer than the
/// command keeps its old value. Updates `clocks` for the fields the command wins.
pub fn apply_lww(
    existing: Option<Row>,
    clocks: &mut Clocks,
    cmd: &Command,
) -> Result<Row, ErrorCode> {
    let old = existing.clone();
    let new = apply(existing, cmd)?;
    let Some(old) = old else {
        // A create stamps every field, so devices that see it observe its clock.
        let serde_json::Value::Object(fields) = json(&new) else {
            unreachable!("rows are objects")
        };
        for field in fields
            .keys()
            .filter(|f| !matches!(f.as_str(), "kind" | "id"))
        {
            clocks.insert(field.clone(), cmd.hlc.clone());
        }
        return Ok(new);
    };
    let (mut merged, old) = (json(&new), json(&old));
    for field in cmd.op.fields() {
        if clocks.get(field).is_some_and(|c| *c > cmd.hlc) {
            merged[field] = old[field].clone();
        } else {
            clocks.insert(field.into(), cmd.hlc.clone());
        }
    }
    Ok(serde_json::from_value(merged).expect("a row round-trips through JSON"))
}

fn json(row: &Row) -> serde_json::Value {
    serde_json::to_value(row).expect("rows serialize")
}

/// UUIDv7 from the caller's clock and randomness, so wasm needs no RNG of its own.
pub fn new_id(now_ms: u64, random: &[u8; 10]) -> Uuid {
    Builder::from_unix_timestamp_millis(now_ms, random).into_uuid()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_is_uuidv7_at_the_given_time() {
        let id = new_id(1_700_000_000_123, &[7; 10]);
        assert_eq!(id.get_version_num(), 7);
        let (secs, nanos) = id.get_timestamp().unwrap().to_unix();
        assert_eq!((secs, nanos), (1_700_000_000, 123_000_000));
    }

    #[test]
    fn hlc_next_always_moves_forward_even_when_the_wall_clock_goes_back() {
        let device = new_id(1, &[1; 10]);
        let t0 = Hlc {
            wall: 100,
            counter: 0,
            device,
        };
        let later = t0.next(200);
        assert_eq!((later.wall, later.counter), (200, 0));
        let same_ms = later.next(200);
        assert_eq!((same_ms.wall, same_ms.counter), (200, 1));
        let skewed_back = same_ms.next(50);
        assert_eq!((skewed_back.wall, skewed_back.counter), (200, 2));
        assert!(t0 < later && later < same_ms && same_ms < skewed_back);
    }

    #[test]
    fn hlc_observe_moves_a_slow_device_past_what_it_has_seen() {
        let slow = new_id(1, &[1; 10]);
        let fast = new_id(1, &[2; 10]);
        let mine = Hlc {
            wall: 100,
            counter: 4,
            device: slow,
        };
        let seen = Hlc {
            wall: 5000,
            counter: 2,
            device: fast,
        };
        // Slow wall clock: adopt the seen wall, step its counter, keep my device.
        assert_eq!(
            mine.observe(&seen, 200),
            Hlc {
                wall: 5000,
                counter: 3,
                device: slow
            }
        );
        // Same wall on both sides: past both counters.
        let tie = Hlc {
            wall: 5000,
            counter: 7,
            device: slow,
        };
        assert_eq!(
            tie.observe(&seen, 200),
            Hlc {
                wall: 5000,
                counter: 8,
                device: slow
            }
        );
        // My wall clock is ahead of everything: a fresh tick.
        assert_eq!(
            mine.observe(&seen, 9000),
            Hlc {
                wall: 9000,
                counter: 0,
                device: slow
            }
        );
    }

    const TASK: &str = "01900000-0000-7000-8000-00000000000a";
    const DEVICE: &str = "01900000-0000-7000-8000-0000000000d1";

    fn cmd(uuid: &str, r#type: &str, args: serde_json::Value, wall: u64) -> Command {
        serde_json::from_value(serde_json::json!({
            "uuid": uuid, "type": r#type, "args": args,
            "hlc": { "wall": wall, "counter": 0, "device": DEVICE },
        }))
        .unwrap()
    }

    #[test]
    fn task_create_puts_an_open_task_in_inbox() {
        let create = cmd(
            "01900000-0000-7000-8000-0000000000c1",
            "task.create",
            serde_json::json!({ "id": TASK, "title": "  Buy milk " }),
            1000,
        );
        let row = apply(None, &create).unwrap();
        assert_eq!(
            serde_json::to_value(&row).unwrap(),
            serde_json::json!({
                "kind": "task", "id": TASK, "area_id": "00000000-0000-7000-8000-000000000001",
                "title": "Buy milk", "completed_at": null,
            })
        );
    }

    fn created() -> Row {
        let create = cmd(
            "01900000-0000-7000-8000-0000000000c1",
            "task.create",
            serde_json::json!({ "id": TASK, "title": "Buy milk" }),
            1000,
        );
        apply(None, &create).unwrap()
    }

    fn complete(wall: u64) -> Command {
        cmd(
            "01900000-0000-7000-8000-0000000000c2",
            "task.complete",
            serde_json::json!({ "id": TASK }),
            wall,
        )
    }

    #[test]
    fn task_complete_stamps_the_time_the_user_acted_and_keeps_the_first() {
        let done = apply(Some(created()), &complete(5000)).unwrap();
        let Row::Task(t) = &done else { panic!() };
        assert_eq!(t.completed_at, Some(5000));
        let again = apply(Some(done), &complete(9000)).unwrap();
        let Row::Task(t) = again else { panic!() };
        assert_eq!(t.completed_at, Some(5000));
    }

    #[test]
    fn invalid_commands_are_rejected_with_a_code() {
        let blank = cmd(
            "01900000-0000-7000-8000-0000000000c3",
            "task.create",
            serde_json::json!({ "id": TASK, "title": "   " }),
            1000,
        );
        let dup = cmd(
            "01900000-0000-7000-8000-0000000000c4",
            "task.create",
            serde_json::json!({ "id": TASK, "title": "Again" }),
            1000,
        );
        assert_eq!(apply(None, &blank), Err(ErrorCode::TitleRequired));
        assert_eq!(apply(Some(created()), &dup), Err(ErrorCode::Exists));
        assert_eq!(apply(None, &complete(5000)), Err(ErrorCode::NotFound));
        assert_eq!(
            apply(Some(Row::inbox()), &complete(5000)),
            Err(ErrorCode::NotFound)
        );
    }

    fn rename(uuid: &str, title: &str, wall: u64) -> Command {
        cmd(
            uuid,
            "task.update",
            serde_json::json!({ "id": TASK, "title": title }),
            wall,
        )
    }

    fn task(row: &Row) -> &Task {
        let Row::Task(t) = row else {
            panic!("not a task")
        };
        t
    }

    #[test]
    fn edits_to_different_fields_both_survive_in_either_order() {
        let create = cmd(
            "01900000-0000-7000-8000-0000000000c1",
            "task.create",
            serde_json::json!({ "id": TASK, "title": "Buy milk" }),
            1000,
        );
        let a = rename("01900000-0000-7000-8000-0000000000c2", "Buy oat milk", 3000);
        let b = complete(2000);
        for (first, second) in [(&a, &b), (&b, &a)] {
            let mut clocks = Clocks::new();
            let row = apply_lww(None, &mut clocks, &create).unwrap();
            let row = apply_lww(Some(row), &mut clocks, first).unwrap();
            let row = apply_lww(Some(row), &mut clocks, second).unwrap();
            assert_eq!(
                (task(&row).title.as_str(), task(&row).completed_at),
                ("Buy oat milk", Some(2000))
            );
        }
    }

    #[test]
    fn an_older_edit_to_the_same_field_never_overwrites_a_newer_one() {
        let create = cmd(
            "01900000-0000-7000-8000-0000000000c1",
            "task.create",
            serde_json::json!({ "id": TASK, "title": "Buy milk" }),
            1000,
        );
        let newer = rename("01900000-0000-7000-8000-0000000000c2", "Newer", 3000);
        let late_offline = rename("01900000-0000-7000-8000-0000000000c3", "Older", 2000);
        let mut clocks = Clocks::new();
        let row = apply_lww(None, &mut clocks, &create).unwrap();
        let row = apply_lww(Some(row), &mut clocks, &newer).unwrap();
        let row = apply_lww(Some(row), &mut clocks, &late_offline).unwrap();
        assert_eq!(task(&row).title, "Newer");
        assert_eq!(clocks["title"].wall, 3000);
    }

    #[test]
    fn task_update_rejects_a_blank_title_and_a_missing_task() {
        let blank = rename("01900000-0000-7000-8000-0000000000c2", "  ", 3000);
        assert_eq!(
            apply(Some(created()), &blank),
            Err(ErrorCode::TitleRequired)
        );
        let ok = rename("01900000-0000-7000-8000-0000000000c3", "x", 3000);
        assert_eq!(apply(None, &ok), Err(ErrorCode::NotFound));
    }

    #[test]
    fn a_create_stamps_every_field_with_its_clock() {
        let create = cmd(
            "01900000-0000-7000-8000-0000000000c1",
            "task.create",
            serde_json::json!({ "id": TASK, "title": "Buy milk" }),
            1000,
        );
        let mut clocks = Clocks::new();
        apply_lww(None, &mut clocks, &create).unwrap();
        let fields: Vec<_> = clocks.iter().map(|(f, c)| (f.as_str(), c.wall)).collect();
        assert_eq!(
            fields,
            [("area_id", 1000), ("completed_at", 1000), ("title", 1000)]
        );
    }

    #[test]
    fn the_server_takes_the_current_and_previous_protocol_versions() {
        assert_eq!(check_version(5, 5), Ok(()));
        assert_eq!(check_version(5, 4), Ok(()));
        assert_eq!(check_version(5, 3), Err(VersionError::TooOld));
        assert_eq!(check_version(5, 0), Err(VersionError::TooOld));
        assert_eq!(check_version(5, 6), Err(VersionError::TooNew));
    }
}
