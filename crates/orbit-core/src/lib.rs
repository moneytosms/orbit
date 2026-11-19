pub mod order;
pub mod parse;
pub mod theme;

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
    /// Fractional index among siblings (see `order`).
    #[serde(default)]
    pub order: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Project {
    pub id: Uuid,
    /// The Area it lives in.
    pub parent_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub order: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Section {
    pub id: Uuid,
    /// The Area or Project it lives in.
    pub parent_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub order: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Task {
    pub id: Uuid,
    /// An Area, Project, Section or another Task (subtask).
    #[serde(alias = "area_id")]
    pub parent_id: Uuid,
    pub title: String,
    /// Unix ms.
    pub completed_at: Option<u64>,
    #[serde(default)]
    pub order: String,
}

/// A synced record, as stored by the server and sent in deltas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Row {
    Area(Area),
    Project(Project),
    Section(Section),
    Task(Task),
}

impl Row {
    pub fn id(&self) -> Uuid {
        match self {
            Row::Area(a) => a.id,
            Row::Project(p) => p.id,
            Row::Section(s) => s.id,
            Row::Task(t) => t.id,
        }
    }

    /// The row this one lives under. Areas have none.
    pub fn parent_id(&self) -> Option<Uuid> {
        match self {
            Row::Area(_) => None,
            Row::Project(p) => Some(p.parent_id),
            Row::Section(s) => Some(s.parent_id),
            Row::Task(t) => Some(t.parent_id),
        }
    }

    pub fn inbox() -> Row {
        Row::Area(Area {
            id: INBOX,
            name: "Inbox".into(),
            order: String::new(),
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
    #[serde(rename = "area.create")]
    AreaCreate {
        id: Uuid,
        name: String,
        #[serde(default)]
        order: String,
    },
    #[serde(rename = "project.create")]
    ProjectCreate {
        id: Uuid,
        parent_id: Uuid,
        name: String,
        #[serde(default)]
        order: String,
    },
    #[serde(rename = "section.create")]
    SectionCreate {
        id: Uuid,
        parent_id: Uuid,
        name: String,
        #[serde(default)]
        order: String,
    },
    /// No `parent_id` means the Inbox.
    #[serde(rename = "task.create")]
    TaskCreate {
        id: Uuid,
        title: String,
        #[serde(default)]
        parent_id: Option<Uuid>,
        #[serde(default)]
        order: String,
    },
    /// Reparents and reorders any kind. Areas have no parent: only their order changes.
    #[serde(rename = "item.move")]
    ItemMove {
        id: Uuid,
        parent_id: Option<Uuid>,
        order: String,
    },
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
            Op::AreaCreate { id, .. }
            | Op::ProjectCreate { id, .. }
            | Op::SectionCreate { id, .. }
            | Op::TaskCreate { id, .. }
            | Op::ItemMove { id, .. }
            | Op::TaskUpdate { id, .. }
            | Op::TaskComplete { id } => *id,
        }
    }

    /// Row fields an edit writes, each guarded by its own clock. A create stamps all fields instead.
    fn fields(&self) -> Vec<&'static str> {
        match self {
            Op::AreaCreate { .. }
            | Op::ProjectCreate { .. }
            | Op::SectionCreate { .. }
            | Op::TaskCreate { .. } => vec![],
            Op::ItemMove { .. } => vec!["parent_id", "order"],
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
    /// The parent breaks the tree rules, is missing, or would make a cycle.
    BadParent,
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
        Op::AreaCreate { id, name, order } => {
            let name = name.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if name.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else {
                Ok(Row::Area(Area {
                    id: *id,
                    name: name.into(),
                    order: order.clone(),
                }))
            }
        }
        Op::ProjectCreate {
            id,
            parent_id,
            name,
            order,
        } => {
            let name = name.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if name.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else {
                Ok(Row::Project(Project {
                    id: *id,
                    parent_id: *parent_id,
                    name: name.into(),
                    order: order.clone(),
                }))
            }
        }
        Op::SectionCreate {
            id,
            parent_id,
            name,
            order,
        } => {
            let name = name.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if name.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else {
                Ok(Row::Section(Section {
                    id: *id,
                    parent_id: *parent_id,
                    name: name.into(),
                    order: order.clone(),
                }))
            }
        }
        Op::TaskCreate {
            id,
            title,
            parent_id,
            order,
        } => {
            let title = title.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if title.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else {
                Ok(Row::Task(Task {
                    id: *id,
                    parent_id: parent_id.unwrap_or(INBOX),
                    title: title.into(),
                    completed_at: None,
                    order: order.clone(),
                }))
            }
        }
        Op::ItemMove {
            id,
            parent_id,
            order,
        } => {
            if *id == INBOX {
                return Err(ErrorCode::BadParent);
            }
            match existing {
                Some(Row::Area(a)) if parent_id.is_none() => Ok(Row::Area(Area {
                    order: order.clone(),
                    ..a
                })),
                Some(Row::Area(_)) => Err(ErrorCode::BadParent),
                Some(Row::Project(p)) => Ok(Row::Project(Project {
                    parent_id: parent_id.ok_or(ErrorCode::BadParent)?,
                    order: order.clone(),
                    ..p
                })),
                Some(Row::Section(s)) => Ok(Row::Section(Section {
                    parent_id: parent_id.ok_or(ErrorCode::BadParent)?,
                    order: order.clone(),
                    ..s
                })),
                Some(Row::Task(t)) => Ok(Row::Task(Task {
                    parent_id: parent_id.unwrap_or(INBOX),
                    order: order.clone(),
                    ..t
                })),
                None => Err(ErrorCode::NotFound),
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

/// The tree rules for `child` (its `parent_id` already set) under `parent`, the row that id
/// points at. Callers load the parent and call this before applying a create or move; a missing
/// parent is `None`. Cycles need the whole chain, so callers walk it themselves.
pub fn check_parent(child: &Row, parent: Option<&Row>) -> Result<(), ErrorCode> {
    let Some(want) = child.parent_id() else {
        return if parent.is_none() {
            Ok(())
        } else {
            Err(ErrorCode::BadParent)
        };
    };
    let Some(parent) = parent.filter(|p| p.id() == want) else {
        return Err(ErrorCode::BadParent);
    };
    let ok = match (child, parent) {
        (Row::Area(_), _) => false,
        // Inbox holds only Tasks.
        (Row::Project(_), Row::Area(a)) | (Row::Section(_), Row::Area(a)) => a.id != INBOX,
        (Row::Section(_), Row::Project(_)) => true,
        (Row::Task(_), Row::Area(_) | Row::Project(_) | Row::Section(_) | Row::Task(_)) => true,
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(ErrorCode::BadParent)
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
        // An Area has no `parent_id`: a move only changes its order.
        if old.get(field).is_none() {
            continue;
        }
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
                "kind": "task", "id": TASK, "parent_id": "00000000-0000-7000-8000-000000000001",
                "title": "Buy milk", "completed_at": null, "order": "",
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
            [
                ("completed_at", 1000),
                ("order", 1000),
                ("parent_id", 1000),
                ("title", 1000)
            ]
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

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 + n)
    }

    fn area(n: u128) -> Row {
        Row::Area(Area {
            id: uid(n),
            name: "A".into(),
            order: "a".into(),
        })
    }
    fn project(n: u128, parent: Uuid) -> Row {
        Row::Project(Project {
            id: uid(n),
            parent_id: parent,
            name: "P".into(),
            order: "a".into(),
        })
    }
    fn section(n: u128, parent: Uuid) -> Row {
        Row::Section(Section {
            id: uid(n),
            parent_id: parent,
            name: "S".into(),
            order: "a".into(),
        })
    }
    fn task_row(n: u128, parent: Uuid) -> Row {
        Row::Task(Task {
            id: uid(n),
            parent_id: parent,
            title: "T".into(),
            completed_at: None,
            order: "a".into(),
        })
    }

    #[test]
    fn check_parent_allows_exactly_the_tree_shapes() {
        let (a, p, s, t) = (
            area(1),
            project(2, uid(1)),
            section(3, uid(1)),
            task_row(4, uid(1)),
        );
        let inbox = Row::inbox();
        let ok = |child: Row, parent: &Row| {
            let child = match child {
                Row::Area(x) => Row::Area(x),
                Row::Project(x) => Row::Project(Project {
                    parent_id: parent.id(),
                    ..x
                }),
                Row::Section(x) => Row::Section(Section {
                    parent_id: parent.id(),
                    ..x
                }),
                Row::Task(x) => Row::Task(Task {
                    parent_id: parent.id(),
                    ..x
                }),
            };
            check_parent(&child, Some(parent))
        };
        let bad = Err(ErrorCode::BadParent);
        // Areas are flat.
        assert_eq!(check_parent(&a, None), Ok(()));
        assert_eq!(check_parent(&a, Some(&a)), bad);
        // Projects live in Areas only (not the Inbox).
        assert_eq!(ok(p.clone(), &a), Ok(()));
        assert_eq!(ok(p.clone(), &p), bad);
        assert_eq!(ok(p.clone(), &s), bad);
        assert_eq!(ok(p.clone(), &t), bad);
        assert_eq!(ok(p.clone(), &inbox), bad);
        // Sections live in an Area or Project.
        assert_eq!(ok(s.clone(), &a), Ok(()));
        assert_eq!(ok(s.clone(), &p), Ok(()));
        assert_eq!(ok(s.clone(), &s), bad);
        assert_eq!(ok(s.clone(), &t), bad);
        assert_eq!(ok(s.clone(), &inbox), bad);
        // Tasks live anywhere but under nothing, including subtasks and the Inbox.
        for parent in [&a, &p, &s, &t, &inbox] {
            assert_eq!(ok(t.clone(), parent), Ok(()));
        }
        // Missing or mismatched parents.
        assert_eq!(check_parent(&t, None), bad);
        assert_eq!(check_parent(&task_row(5, uid(99)), Some(&a)), bad);
    }

    fn mv(id: Uuid, parent: Option<Uuid>, order: &str, wall: u64) -> Command {
        cmd(
            "01900000-0000-7000-8000-0000000000c9",
            "item.move",
            serde_json::json!({ "id": id, "parent_id": parent, "order": order }),
            wall,
        )
    }

    #[test]
    fn item_move_reparents_and_reorders_but_never_moves_an_area_or_the_inbox() {
        let moved = apply(Some(task_row(4, uid(1))), &mv(uid(4), Some(uid(2)), "b", 5)).unwrap();
        assert_eq!(moved, task_row(4, uid(2)).with_order("b"));
        let p = apply(Some(project(2, uid(1))), &mv(uid(2), Some(uid(7)), "c", 5)).unwrap();
        assert_eq!(p.parent_id(), Some(uid(7)));
        assert_eq!(
            apply(Some(project(2, uid(1))), &mv(uid(2), None, "c", 5)),
            Err(ErrorCode::BadParent)
        );
        // An Area only changes order.
        let a = apply(Some(area(1)), &mv(uid(1), None, "z", 5)).unwrap();
        assert!(matches!(&a, Row::Area(x) if x.order == "z"));
        assert_eq!(
            apply(Some(area(1)), &mv(uid(1), Some(uid(2)), "z", 5)),
            Err(ErrorCode::BadParent)
        );
        assert_eq!(
            apply(Some(Row::inbox()), &mv(INBOX, None, "z", 5)),
            Err(ErrorCode::BadParent)
        );
        assert_eq!(
            apply(None, &mv(uid(4), None, "z", 5)),
            Err(ErrorCode::NotFound)
        );
    }

    impl Row {
        fn with_order(mut self, order: &str) -> Row {
            if let Row::Task(t) = &mut self {
                t.order = order.into();
            }
            self
        }
    }

    #[test]
    fn order_is_last_writer_wins_and_an_area_move_stamps_no_parent_clock() {
        let mut clocks = Clocks::new();
        let row = apply_lww(
            None,
            &mut clocks,
            &cmd(
                "01900000-0000-7000-8000-0000000000c1",
                "area.create",
                serde_json::json!({ "id": uid(1), "name": "Home", "order": "a" }),
                1000,
            ),
        )
        .unwrap();
        let row = apply_lww(Some(row), &mut clocks, &mv(uid(1), None, "n", 3000)).unwrap();
        let row = apply_lww(Some(row), &mut clocks, &mv(uid(1), None, "o", 2000)).unwrap();
        assert!(matches!(&row, Row::Area(a) if a.order == "n"));
        assert!(!clocks.contains_key("parent_id"));
    }

    #[test]
    fn old_rows_with_area_id_and_no_order_still_deserialize() {
        let row: Row = serde_json::from_value(serde_json::json!({
            "kind": "task", "id": TASK, "area_id": INBOX, "title": "Old", "completed_at": null,
        }))
        .unwrap();
        assert_eq!(task(&row).parent_id, INBOX);
        assert_eq!(task(&row).order, "");
        let create: Command = serde_json::from_value(serde_json::json!({
            "uuid": TASK, "type": "task.create", "args": { "id": TASK, "title": "Old" },
            "hlc": { "wall": 1, "counter": 0, "device": DEVICE },
        }))
        .unwrap();
        assert_eq!(task(&apply(None, &create).unwrap()).parent_id, INBOX);
    }
}
