pub mod filter_text;
pub mod goal;
pub mod link;
pub mod next;
pub mod order;
pub mod page;
pub mod parse;
pub mod recur;
pub mod reminder;
pub mod routine;
pub mod search;
pub mod theme;
pub mod trash;
pub mod view;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Builder;

pub use goal::{Goal, Milestone, Progress};
pub use page::Page;
pub use recur::Recurrence;
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

/// The synced Settings record: one row, same id everywhere.
pub const SETTINGS: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0002);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Area {
    pub id: Uuid,
    pub name: String,
    /// Fractional index among siblings (see `order`).
    #[serde(default)]
    pub order: String,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// Unix ms when it was archived.
    #[serde(default)]
    pub archived_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Project {
    pub id: Uuid,
    /// The Area it lives in.
    pub parent_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub order: String,
    /// The Goal its Tasks count towards unless they link another.
    #[serde(default)]
    pub goal_id: Option<Uuid>,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// Unix ms when it was archived.
    #[serde(default)]
    pub archived_at: Option<u64>,
    /// Unix ms of the last edit to it or a task under it.
    #[serde(default)]
    pub touched_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Section {
    pub id: Uuid,
    /// The Area or Project it lives in.
    pub parent_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub order: String,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
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
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub due: Option<Due>,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub duration_min: Option<u32>,
    /// 1 (P1, most urgent) to 4 (P4, the default).
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub label_ids: Vec<Uuid>,
    /// Completing a recurring Task moves its `due` on instead of finishing it.
    #[serde(default)]
    pub recurrence: Option<Recurrence>,
    /// The one Goal this Task counts towards.
    #[serde(default)]
    pub goal_id: Option<Uuid>,
    /// Unix ms when it was created.
    #[serde(default)]
    pub created_at: Option<u64>,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// `YYYY-MM-DD`; pinned to the top of What's next only on that day.
    #[serde(default)]
    pub pinned_on: Option<String>,
    /// `YYYY-MM-DD`; hidden from What's next only on that day.
    #[serde(default)]
    pub not_today_on: Option<String>,
    /// The `orbit:` targets in `description`, kept up to date by every write to it.
    #[serde(default)]
    pub links: Vec<Uuid>,
    /// Offsets before due or deadline, or absolute times. One field for sync.
    #[serde(default)]
    pub reminders: Vec<reminder::Reminder>,
    /// The `blob:` sha256 hashes in `description`, kept up to date by every write to it.
    #[serde(default)]
    pub blobs: Vec<String>,
}

fn default_priority() -> u8 {
    4
}

/// A floating local date, with an optional time of day.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Due {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM`.
    #[serde(default)]
    pub time: Option<String>,
}

/// One time a Task was completed. A recurring Task logs one per occurrence.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Completion {
    pub id: Uuid,
    pub item_id: Uuid,
    /// Unix ms.
    pub at: u64,
}

/// Left behind when a trashed row is purged, so clients drop their copy.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Purged {
    pub id: Uuid,
    /// Unix ms of the purge.
    pub purged_at: u64,
}

/// A flat, global tag.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Label {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    pub id: Uuid,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub accent: String,
    #[serde(default)]
    pub start_view: String,
    /// IANA name; empty is the device's own zone.
    #[serde(default)]
    pub timezone: String,
    /// `HH:MM`; empty is midnight.
    #[serde(default)]
    pub day_start: String,
    /// What's next deadline window in days; `None` = 3.
    #[serde(default)]
    pub next_deadline_days: Option<u32>,
    /// What's next due window in days; `None` = 7.
    #[serde(default)]
    pub next_due_days: Option<u32>,
    /// Whether undated P1 tasks show in What's next; `None` = true.
    #[serde(default)]
    pub next_undated_p1: Option<bool>,
    /// A Task with a timed due gets an "at due time" reminder; `None` is on.
    #[serde(default)]
    pub default_reminder: Option<bool>,
}

/// A synced record, as stored by the server and sent in deltas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Row {
    Area(Area),
    Project(Project),
    Section(Section),
    Task(Task),
    Label(Label),
    Settings(Settings),
    Completion(Completion),
    Goal(Goal),
    Milestone(Milestone),
    Page(Page),
    Purged(Purged),
    View(view::View),
    Routine(routine::Routine),
    Checkin(routine::Checkin),
}

impl Row {
    pub fn id(&self) -> Uuid {
        match self {
            Row::Area(a) => a.id,
            Row::Project(p) => p.id,
            Row::Section(s) => s.id,
            Row::Task(t) => t.id,
            Row::Label(l) => l.id,
            Row::Settings(s) => s.id,
            Row::Completion(c) => c.id,
            Row::Goal(g) => g.id,
            Row::Milestone(m) => m.id,
            Row::Page(p) => p.id,
            Row::Purged(p) => p.id,
            Row::View(v) => v.id,
            Row::Routine(r) => r.id,
            Row::Checkin(c) => c.id,
        }
    }

    /// The row this one lives under. Areas, Labels and Settings have none.
    pub fn parent_id(&self) -> Option<Uuid> {
        match self {
            Row::Area(_)
            | Row::Label(_)
            | Row::Settings(_)
            | Row::Completion(_)
            | Row::Purged(_)
            | Row::View(_) => None,
            Row::Project(p) => Some(p.parent_id),
            Row::Section(s) => Some(s.parent_id),
            Row::Task(t) => Some(t.parent_id),
            Row::Goal(g) => Some(g.parent_id),
            Row::Milestone(m) => Some(m.goal_id),
            Row::Page(p) => Some(p.parent_id),
            Row::Routine(r) => Some(r.parent_id),
            Row::Checkin(c) => Some(c.routine_id),
        }
    }

    pub fn inbox() -> Row {
        Row::Area(Area {
            id: INBOX,
            name: "Inbox".into(),
            order: String::new(),
            trashed_at: None,
            archived_at: None,
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
    /// Absent = unchanged; `null` clears an optional field.
    #[serde(rename = "task.update")]
    TaskUpdate {
        id: Uuid,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        due: Option<Option<Due>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        deadline: Option<Option<String>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        duration_min: Option<Option<u32>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        priority: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label_ids: Option<Vec<Uuid>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        recurrence: Option<Option<Recurrence>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        goal_id: Option<Option<Uuid>>,
    },
    #[serde(rename = "project.update")]
    ProjectUpdate {
        id: Uuid,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        goal_id: Option<Option<Uuid>>,
    },
    #[serde(rename = "goal.create")]
    GoalCreate {
        id: Uuid,
        parent_id: Uuid,
        name: String,
        #[serde(default)]
        progress: Option<Progress>,
        #[serde(default)]
        start: Option<String>,
        #[serde(default)]
        target_date: Option<String>,
        #[serde(default)]
        done_condition: String,
        #[serde(default)]
        order: String,
    },
    #[serde(rename = "goal.update")]
    GoalUpdate {
        id: Uuid,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        done_condition: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        progress: Option<Progress>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        start: Option<Option<String>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        target_date: Option<Option<String>>,
    },
    #[serde(rename = "goal.complete")]
    GoalComplete { id: Uuid },
    /// Appends to a Target Goal's entries.
    #[serde(rename = "goal.add_entry")]
    GoalAddEntry {
        id: Uuid,
        amount: f64,
        date: String,
        #[serde(default)]
        note: String,
    },
    #[serde(rename = "milestone.create")]
    MilestoneCreate {
        id: Uuid,
        goal_id: Uuid,
        title: String,
        #[serde(default)]
        date: Option<String>,
        #[serde(default)]
        order: String,
    },
    /// `done: true` completes it and logs a completion.
    #[serde(rename = "milestone.update")]
    MilestoneUpdate {
        id: Uuid,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        date: Option<Option<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        done: Option<bool>,
    },
    #[serde(rename = "page.create")]
    PageCreate {
        id: Uuid,
        parent_id: Uuid,
        title: String,
        #[serde(default)]
        body: String,
        #[serde(default)]
        order: String,
    },
    #[serde(rename = "page.rename")]
    PageRename { id: Uuid, title: String },
    /// Applies only if `base_rev` is the page's current `rev`; the server keeps a losing body
    /// as a sibling Page.
    #[serde(rename = "page.edit_body")]
    PageEditBody {
        id: Uuid,
        body: String,
        base_rev: u64,
    },
    /// Sends any item to the Trash.
    #[serde(rename = "item.delete")]
    ItemDelete { id: Uuid },
    #[serde(rename = "item.restore")]
    ItemRestore { id: Uuid },
    /// Areas, Projects and Goals only.
    #[serde(rename = "item.archive")]
    ItemArchive { id: Uuid },
    #[serde(rename = "item.unarchive")]
    ItemUnarchive { id: Uuid },
    #[serde(rename = "task.complete")]
    TaskComplete {
        id: Uuid,
        /// The completer's local `YYYY-MM-DD`, which `every!` counts from.
        /// Without it the UTC date of the command's clock is used.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        today: Option<String>,
    },
    #[serde(rename = "label.create")]
    LabelCreate { id: Uuid, name: String },
    /// Creates the Settings row if missing; only the fields present change.
    #[serde(rename = "settings.update")]
    SettingsUpdate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        accent: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_view: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timezone: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        day_start: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next_deadline_days: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next_due_days: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next_undated_p1: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default_reminder: Option<bool>,
    },
    #[serde(rename = "view.create")]
    ViewCreate {
        id: Uuid,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        kind: view::ViewKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scope: Option<Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<view::Node>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sorts: Option<Vec<view::Sort>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layout: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        show_completed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pinned: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        order: Option<String>,
    },
    /// Any field present changes; `filter` replaces the whole tree. Works on a built-in's id.
    #[serde(rename = "view.update")]
    ViewUpdate {
        id: Uuid,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<view::ViewKind>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        scope: Option<Option<Uuid>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<view::Node>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sorts: Option<Vec<view::Sort>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        group_by: Option<Option<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layout: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        show_completed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pinned: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hidden: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        order: Option<String>,
    },
    /// Built-ins only: back to the default.
    #[serde(rename = "view.reset")]
    ViewReset { id: Uuid },
    /// Custom views only: to the Trash.
    #[serde(rename = "view.delete")]
    ViewDelete { id: Uuid },
    #[serde(rename = "routine.create")]
    RoutineCreate {
        id: Uuid,
        parent_id: Uuid,
        name: String,
        schedule: routine::Schedule,
        /// `YYYY-MM-DD`, the creator's Day start date.
        start: String,
        #[serde(default)]
        length_days: Option<u32>,
        #[serde(default)]
        time: Option<String>,
        #[serde(default)]
        goal_id: Option<Uuid>,
        #[serde(default)]
        order: String,
    },
    #[serde(rename = "routine.update")]
    RoutineUpdate {
        id: Uuid,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        schedule: Option<routine::Schedule>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        length_days: Option<Option<u32>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        time: Option<Option<String>>,
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        goal_id: Option<Option<Uuid>>,
        /// `HH:MM` daily reminder; `null` clears it.
        #[serde(
            default,
            deserialize_with = "some",
            skip_serializing_if = "Option::is_none"
        )]
        reminder: Option<Option<String>>,
    },
    /// Stops misses from `date` on.
    #[serde(rename = "routine.pause")]
    RoutinePause { id: Uuid, date: String },
    /// Ends the open pause at `date`.
    #[serde(rename = "routine.resume")]
    RoutineResume { id: Uuid, date: String },
    /// `id` is `checkin_id(routine_id, date)`, so two devices share one row.
    #[serde(rename = "checkin.set")]
    CheckinSet {
        id: Uuid,
        routine_id: Uuid,
        date: String,
        state: routine::CheckState,
    },
    #[serde(rename = "checkin.clear")]
    CheckinClear { id: Uuid },
    /// Pins a Task or Milestone to the top of What's next on `date` (null clears).
    #[serde(rename = "item.pin")]
    ItemPin { id: Uuid, date: Option<String> },
    /// Hides a Task or Milestone from What's next on `date` (null clears).
    #[serde(rename = "item.not_today")]
    ItemNotToday { id: Uuid, date: Option<String> },
    #[serde(rename = "reminder.add")]
    ReminderAdd {
        task_id: Uuid,
        reminder: reminder::Reminder,
    },
    #[serde(rename = "reminder.remove")]
    ReminderRemove { task_id: Uuid, id: Uuid },
    /// Adds a one-off absolute reminder at `at` (`YYYY-MM-DDTHH:MM`), its id the command's uuid.
    #[serde(rename = "reminder.snooze")]
    ReminderSnooze { task_id: Uuid, at: String },
    #[serde(rename = "reminder.spend")]
    ReminderSpend { task_id: Uuid, id: Uuid },
}

/// Deserializes a present value, `null` included, as `Some`, so that an absent field
/// (via `default`) stays `None`.
fn some<'de, T, D>(d: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(d).map(Some)
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
        && matches!(s[5..7].parse::<u8>(), Ok(1..=12))
        && matches!(s[8..10].parse::<u8>(), Ok(1..=31))
}

fn is_time(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b.iter().enumerate().all(|(i, c)| {
            if i == 2 {
                *c == b':'
            } else {
                c.is_ascii_digit()
            }
        })
        && s[0..2].parse::<u8>().is_ok_and(|h| h < 24)
        && s[3..5].parse::<u8>().is_ok_and(|m| m < 60)
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
            | Op::ItemDelete { id }
            | Op::ItemRestore { id }
            | Op::ItemArchive { id }
            | Op::ItemUnarchive { id }
            | Op::TaskUpdate { id, .. }
            | Op::TaskComplete { id, .. }
            | Op::ProjectUpdate { id, .. }
            | Op::GoalCreate { id, .. }
            | Op::GoalUpdate { id, .. }
            | Op::GoalComplete { id }
            | Op::GoalAddEntry { id, .. }
            | Op::MilestoneCreate { id, .. }
            | Op::MilestoneUpdate { id, .. }
            | Op::PageCreate { id, .. }
            | Op::PageRename { id, .. }
            | Op::PageEditBody { id, .. }
            | Op::ViewCreate { id, .. }
            | Op::ViewUpdate { id, .. }
            | Op::ViewReset { id }
            | Op::ViewDelete { id }
            | Op::RoutineCreate { id, .. }
            | Op::RoutineUpdate { id, .. }
            | Op::RoutinePause { id, .. }
            | Op::RoutineResume { id, .. }
            | Op::CheckinSet { id, .. }
            | Op::CheckinClear { id }
            | Op::LabelCreate { id, .. }
            | Op::ItemPin { id, .. }
            | Op::ItemNotToday { id, .. } => *id,
            Op::ReminderAdd { task_id, .. }
            | Op::ReminderRemove { task_id, .. }
            | Op::ReminderSnooze { task_id, .. }
            | Op::ReminderSpend { task_id, .. } => *task_id,
            Op::SettingsUpdate { .. } => SETTINGS,
        }
    }

    /// Whether this command creates its row.
    pub fn is_create(&self) -> bool {
        matches!(
            self,
            Op::AreaCreate { .. }
                | Op::ProjectCreate { .. }
                | Op::SectionCreate { .. }
                | Op::TaskCreate { .. }
                | Op::GoalCreate { .. }
                | Op::MilestoneCreate { .. }
                | Op::PageCreate { .. }
                | Op::RoutineCreate { .. }
        )
    }

    /// The parent a create or move puts its row under.
    pub fn new_parent(&self) -> Option<Uuid> {
        match self {
            Op::ProjectCreate { parent_id, .. }
            | Op::SectionCreate { parent_id, .. }
            | Op::GoalCreate { parent_id, .. }
            | Op::PageCreate { parent_id, .. } => Some(*parent_id),
            Op::TaskCreate { parent_id, .. } => Some(parent_id.unwrap_or(INBOX)),
            Op::MilestoneCreate { goal_id, .. } => Some(*goal_id),
            Op::RoutineCreate { parent_id, .. } => Some(*parent_id),
            Op::CheckinSet { routine_id, .. } => Some(*routine_id),
            Op::ItemMove { parent_id, .. } => *parent_id,
            _ => None,
        }
    }

    /// Row fields an edit writes, each guarded by its own clock. A create stamps all fields instead.
    fn fields(&self) -> Vec<&'static str> {
        match self {
            Op::AreaCreate { .. }
            | Op::ProjectCreate { .. }
            | Op::SectionCreate { .. }
            | Op::TaskCreate { .. }
            | Op::GoalCreate { .. }
            | Op::MilestoneCreate { .. }
            | Op::PageCreate { .. }
            | Op::ViewCreate { .. }
            | Op::RoutineCreate { .. }
            | Op::LabelCreate { .. } => vec![],
            Op::ViewDelete { .. } => vec!["trashed_at"],
            Op::ViewReset { .. } => vec![
                "name",
                "icon",
                "entity",
                "scope",
                "filter",
                "sorts",
                "group_by",
                "layout",
                "fields",
                "show_completed",
                "pinned",
                "hidden",
                "order",
                "trashed_at",
            ],
            Op::ViewUpdate {
                name,
                icon,
                kind,
                scope,
                filter,
                sorts,
                group_by,
                layout,
                fields,
                show_completed,
                pinned,
                hidden,
                order,
                ..
            } => [
                ("name", name.is_some()),
                ("icon", icon.is_some()),
                ("entity", kind.is_some()),
                ("scope", scope.is_some()),
                ("filter", filter.is_some()),
                ("sorts", sorts.is_some()),
                ("group_by", group_by.is_some()),
                ("layout", layout.is_some()),
                ("fields", fields.is_some()),
                ("show_completed", show_completed.is_some()),
                ("pinned", pinned.is_some()),
                ("hidden", hidden.is_some()),
                ("order", order.is_some()),
            ]
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(f, _)| f)
            .collect(),
            Op::ItemDelete { .. } | Op::ItemRestore { .. } => vec!["trashed_at"],
            Op::ItemArchive { .. } | Op::ItemUnarchive { .. } => vec!["archived_at"],
            Op::PageRename { .. } => vec!["title"],
            Op::PageEditBody { .. } => vec!["body", "rev", "links", "blobs"],
            Op::ProjectUpdate { goal_id, .. } => {
                if goal_id.is_some() {
                    vec!["goal_id"]
                } else {
                    vec![]
                }
            }
            Op::GoalUpdate {
                name,
                done_condition,
                progress,
                start,
                target_date,
                ..
            } => [
                ("name", name.is_some()),
                ("done_condition", done_condition.is_some()),
                ("progress", progress.is_some()),
                ("start", start.is_some()),
                ("target_date", target_date.is_some()),
            ]
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(f, _)| f)
            .collect(),
            Op::GoalComplete { .. } => vec!["completed_at"],
            Op::GoalAddEntry { .. } => vec!["progress"],
            Op::MilestoneUpdate {
                title, date, done, ..
            } => [
                ("title", title.is_some()),
                ("date", date.is_some()),
                ("done", done.is_some()),
            ]
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(f, _)| f)
            .collect(),
            Op::ItemMove { .. } => vec!["parent_id", "order"],
            Op::TaskUpdate {
                title,
                description,
                due,
                deadline,
                duration_min,
                priority,
                label_ids,
                recurrence,
                goal_id,
                ..
            } => [
                ("title", title.is_some()),
                ("description", description.is_some()),
                ("due", due.is_some()),
                ("deadline", deadline.is_some()),
                ("duration_min", duration_min.is_some()),
                ("priority", priority.is_some()),
                ("label_ids", label_ids.is_some()),
                ("recurrence", recurrence.is_some()),
                ("goal_id", goal_id.is_some()),
            ]
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(f, _)| f)
            .chain(description.is_some().then_some("links"))
            .chain(description.is_some().then_some("blobs"))
            .collect(),
            Op::SettingsUpdate {
                theme,
                accent,
                start_view,
                timezone,
                day_start,
                next_deadline_days,
                next_due_days,
                next_undated_p1,
                default_reminder,
            } => [
                ("theme", theme.is_some()),
                ("accent", accent.is_some()),
                ("start_view", start_view.is_some()),
                ("timezone", timezone.is_some()),
                ("day_start", day_start.is_some()),
                ("next_deadline_days", next_deadline_days.is_some()),
                ("next_due_days", next_due_days.is_some()),
                ("next_undated_p1", next_undated_p1.is_some()),
                ("default_reminder", default_reminder.is_some()),
            ]
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(f, _)| f)
            .collect(),
            Op::TaskComplete { .. } => vec!["completed_at"],
            Op::RoutineUpdate {
                name,
                schedule,
                length_days,
                time,
                goal_id,
                reminder,
                ..
            } => [
                ("name", name.is_some()),
                ("schedule", schedule.is_some()),
                ("length_days", length_days.is_some()),
                ("time", time.is_some()),
                ("goal_id", goal_id.is_some()),
                ("reminder", reminder.is_some()),
            ]
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(f, _)| f)
            .collect(),
            Op::RoutinePause { .. } | Op::RoutineResume { .. } => vec!["pauses"],
            Op::CheckinSet { .. } => vec!["state", "cleared"],
            Op::CheckinClear { .. } => vec!["cleared"],
            Op::ItemPin { .. } => vec!["pinned_on", "not_today_on"],
            Op::ItemNotToday { .. } => vec!["not_today_on", "pinned_on"],
            Op::ReminderAdd { .. }
            | Op::ReminderRemove { .. }
            | Op::ReminderSnooze { .. }
            | Op::ReminderSpend { .. } => vec!["reminders"],
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
    /// A field value is out of range or the wrong shape.
    BadField,
    /// The item or an ancestor is archived, which makes it read-only.
    Archived,
    /// The body references a `blob:` hash the server doesn't have yet; upload it and retry.
    MissingBlob,
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
    /// The delta is the whole data set: clients drop their rows before applying it.
    #[serde(default)]
    pub full: bool,
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
                    trashed_at: None,
                    archived_at: None,
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
                    goal_id: None,
                    trashed_at: None,
                    archived_at: None,
                    touched_at: Some(cmd.hlc.wall),
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
                    trashed_at: None,
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
                    description: String::new(),
                    due: None,
                    deadline: None,
                    duration_min: None,
                    priority: default_priority(),
                    label_ids: vec![],
                    recurrence: None,
                    goal_id: None,
                    created_at: Some(cmd.hlc.wall),
                    trashed_at: None,
                    pinned_on: None,
                    not_today_on: None,
                    links: vec![],
                    reminders: vec![],
                    blobs: vec![],
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
                    touched_at: Some(cmd.hlc.wall.max(p.touched_at.unwrap_or(0))),
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
                Some(Row::Goal(g)) => Ok(Row::Goal(Goal {
                    parent_id: parent_id.ok_or(ErrorCode::BadParent)?,
                    order: order.clone(),
                    touched_at: Some(cmd.hlc.wall.max(g.touched_at.unwrap_or(0))),
                    ..g
                })),
                Some(Row::Milestone(m)) => Ok(Row::Milestone(Milestone {
                    goal_id: parent_id.ok_or(ErrorCode::BadParent)?,
                    order: order.clone(),
                    ..m
                })),
                Some(Row::Page(p)) => Ok(Row::Page(Page {
                    parent_id: parent_id.ok_or(ErrorCode::BadParent)?,
                    order: order.clone(),
                    ..p
                })),
                Some(Row::Routine(r)) => Ok(Row::Routine(routine::Routine {
                    parent_id: parent_id.ok_or(ErrorCode::BadParent)?,
                    order: order.clone(),
                    ..r
                })),
                Some(
                    Row::Label(_)
                    | Row::Settings(_)
                    | Row::Completion(_)
                    | Row::Purged(_)
                    | Row::View(_)
                    | Row::Checkin(_),
                ) => Err(ErrorCode::BadParent),
                None => Err(ErrorCode::NotFound),
            }
        }
        Op::TaskUpdate {
            title,
            description,
            due,
            deadline,
            duration_min,
            priority,
            label_ids,
            recurrence,
            goal_id,
            ..
        } => match existing {
            Some(Row::Task(t)) => {
                let title = match title.as_deref().map(str::trim) {
                    Some("") => return Err(ErrorCode::TitleRequired),
                    Some(new) => new.into(),
                    None => t.title.clone(),
                };
                if let Some(Some(d)) = due
                    && (!is_date(&d.date) || d.time.as_deref().is_some_and(|t| !is_time(t)))
                {
                    return Err(ErrorCode::BadField);
                }
                if let Some(Some(d)) = deadline
                    && !is_date(d)
                {
                    return Err(ErrorCode::BadField);
                }
                if priority.is_some_and(|p| !(1..=4).contains(&p)) {
                    return Err(ErrorCode::BadField);
                }
                Ok(Row::Task(Task {
                    title,
                    links: match description {
                        Some(d) => link::targets(d),
                        None => t.links,
                    },
                    blobs: match description {
                        Some(d) => link::blob_refs(d),
                        None => t.blobs,
                    },
                    description: description.clone().unwrap_or(t.description),
                    due: due.clone().unwrap_or(t.due),
                    deadline: deadline.clone().unwrap_or(t.deadline),
                    duration_min: duration_min.unwrap_or(t.duration_min),
                    priority: priority.unwrap_or(t.priority),
                    label_ids: label_ids.clone().unwrap_or(t.label_ids),
                    recurrence: recurrence.clone().unwrap_or(t.recurrence),
                    goal_id: goal_id.unwrap_or(t.goal_id),
                    ..t
                }))
            }
            _ => Err(ErrorCode::NotFound),
        },
        Op::ItemDelete { .. }
        | Op::ItemRestore { .. }
        | Op::ItemArchive { .. }
        | Op::ItemUnarchive { .. } => trash::apply_flag(existing, cmd),
        Op::TaskComplete { .. } => match existing {
            Some(Row::Task(t)) => Ok(Row::Task(complete_effects(t, cmd).0)),
            _ => Err(ErrorCode::NotFound),
        },
        Op::ProjectUpdate { goal_id, .. } => match existing {
            Some(Row::Project(p)) => Ok(Row::Project(Project {
                goal_id: goal_id.unwrap_or(p.goal_id),
                touched_at: Some(cmd.hlc.wall.max(p.touched_at.unwrap_or(0))),
                ..p
            })),
            _ => Err(ErrorCode::NotFound),
        },
        Op::ViewCreate { .. }
        | Op::ViewUpdate { .. }
        | Op::ViewReset { .. }
        | Op::ViewDelete { .. } => view::apply(existing, cmd),
        Op::RoutineCreate { .. }
        | Op::RoutineUpdate { .. }
        | Op::RoutinePause { .. }
        | Op::RoutineResume { .. }
        | Op::CheckinSet { .. }
        | Op::CheckinClear { .. } => routine::apply(existing, cmd),
        Op::ReminderAdd { .. }
        | Op::ReminderRemove { .. }
        | Op::ReminderSnooze { .. }
        | Op::ReminderSpend { .. } => reminder::apply(existing, cmd),
        Op::GoalCreate {
            id,
            parent_id,
            name,
            progress,
            start,
            target_date,
            done_condition,
            order,
        } => {
            let name = name.trim();
            if existing.is_some() {
                return Err(ErrorCode::Exists);
            }
            if name.is_empty() {
                return Err(ErrorCode::TitleRequired);
            }
            let progress = progress.clone().unwrap_or(Progress::Count);
            check_goal_fields(&progress, start.as_deref(), target_date.as_deref())?;
            Ok(Row::Goal(Goal {
                id: *id,
                parent_id: *parent_id,
                name: name.into(),
                done_condition: done_condition.clone(),
                progress,
                start: start.clone(),
                target_date: target_date.clone(),
                completed_at: None,
                order: order.clone(),
                touched_at: Some(cmd.hlc.wall),
                trashed_at: None,
                archived_at: None,
            }))
        }
        Op::GoalUpdate {
            name,
            done_condition,
            progress,
            start,
            target_date,
            ..
        } => match existing {
            Some(Row::Goal(g)) => {
                let name = match name.as_deref().map(str::trim) {
                    Some("") => return Err(ErrorCode::TitleRequired),
                    Some(new) => new.into(),
                    None => g.name.clone(),
                };
                let progress = progress.clone().unwrap_or(g.progress.clone());
                let start = start.clone().unwrap_or(g.start.clone());
                let target_date = target_date.clone().unwrap_or(g.target_date.clone());
                check_goal_fields(&progress, start.as_deref(), target_date.as_deref())?;
                Ok(Row::Goal(Goal {
                    name,
                    done_condition: done_condition.clone().unwrap_or(g.done_condition.clone()),
                    progress,
                    start,
                    target_date,
                    touched_at: touch(&g, cmd),
                    ..g
                }))
            }
            _ => Err(ErrorCode::NotFound),
        },
        Op::GoalComplete { .. } => match existing {
            Some(Row::Goal(g)) => Ok(Row::Goal(Goal {
                completed_at: g.completed_at.or(Some(cmd.hlc.wall)),
                touched_at: touch(&g, cmd),
                ..g
            })),
            _ => Err(ErrorCode::NotFound),
        },
        Op::GoalAddEntry {
            amount, date, note, ..
        } => match existing {
            Some(Row::Goal(g)) => {
                let Progress::Target {
                    target,
                    mut entries,
                } = g.progress.clone()
                else {
                    return Err(ErrorCode::BadField);
                };
                if !amount.is_finite() || !is_date(date) {
                    return Err(ErrorCode::BadField);
                }
                entries.push(goal::Entry {
                    amount: *amount,
                    date: date.clone(),
                    note: note.clone(),
                });
                Ok(Row::Goal(Goal {
                    progress: Progress::Target { target, entries },
                    touched_at: touch(&g, cmd),
                    ..g
                }))
            }
            _ => Err(ErrorCode::NotFound),
        },
        Op::MilestoneCreate {
            id,
            goal_id,
            title,
            date,
            order,
        } => {
            let title = title.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if title.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else if date.as_deref().is_some_and(|d| !is_date(d)) {
                Err(ErrorCode::BadField)
            } else {
                Ok(Row::Milestone(Milestone {
                    id: *id,
                    goal_id: *goal_id,
                    title: title.into(),
                    date: date.clone(),
                    done: false,
                    order: order.clone(),
                    trashed_at: None,
                    pinned_on: None,
                    not_today_on: None,
                }))
            }
        }
        Op::MilestoneUpdate {
            title, date, done, ..
        } => match existing {
            Some(Row::Milestone(m)) => {
                let title = match title.as_deref().map(str::trim) {
                    Some("") => return Err(ErrorCode::TitleRequired),
                    Some(new) => new.into(),
                    None => m.title.clone(),
                };
                if let Some(Some(d)) = date
                    && !is_date(d)
                {
                    return Err(ErrorCode::BadField);
                }
                Ok(Row::Milestone(Milestone {
                    title,
                    date: date.clone().unwrap_or(m.date.clone()),
                    done: done.unwrap_or(m.done),
                    ..m
                }))
            }
            _ => Err(ErrorCode::NotFound),
        },
        Op::PageCreate {
            id,
            parent_id,
            title,
            body,
            order,
        } => {
            let title = title.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if title.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else {
                Ok(Row::Page(Page {
                    id: *id,
                    parent_id: *parent_id,
                    title: title.into(),
                    body: body.clone(),
                    rev: 0,
                    order: order.clone(),
                    trashed_at: None,
                    links: link::targets(body),
                    blobs: link::blob_refs(body),
                }))
            }
        }
        Op::PageRename { title, .. } => match existing {
            Some(Row::Page(p)) => match title.trim() {
                "" => Err(ErrorCode::TitleRequired),
                title => Ok(Row::Page(Page {
                    title: title.into(),
                    ..p
                })),
            },
            _ => Err(ErrorCode::NotFound),
        },
        Op::PageEditBody { body, base_rev, .. } => match existing {
            // A stale edit leaves the page alone; the server saves its body as a sibling.
            Some(Row::Page(p)) if page::is_conflict(&p, *base_rev) => Ok(Row::Page(p)),
            Some(Row::Page(p)) => Ok(Row::Page(Page {
                body: body.clone(),
                rev: p.rev + 1,
                links: link::targets(body),
                blobs: link::blob_refs(body),
                ..p
            })),
            _ => Err(ErrorCode::NotFound),
        },
        Op::LabelCreate { id, name } => {
            let name = name.trim();
            if existing.is_some() {
                Err(ErrorCode::Exists)
            } else if name.is_empty() {
                Err(ErrorCode::TitleRequired)
            } else {
                Ok(Row::Label(Label {
                    id: *id,
                    name: name.into(),
                }))
            }
        }
        Op::ItemPin { date, .. } | Op::ItemNotToday { date, .. } => {
            if date.as_deref().is_some_and(|d| !is_date(d)) {
                return Err(ErrorCode::BadField);
            }
            let pin = matches!(&cmd.op, Op::ItemPin { .. });
            // Setting one clears the other, so the later write wins.
            let apply_to = |old_pin: Option<String>, old_not: Option<String>| match (pin, date) {
                (true, Some(_)) => (date.clone(), None),
                (true, None) => (None, old_not),
                (false, Some(_)) => (None, date.clone()),
                (false, None) => (old_pin, None),
            };
            match existing {
                Some(Row::Task(t)) => {
                    let (p, n) = apply_to(t.pinned_on.clone(), t.not_today_on.clone());
                    Ok(Row::Task(Task {
                        pinned_on: p,
                        not_today_on: n,
                        ..t
                    }))
                }
                Some(Row::Milestone(m)) => {
                    let (p, n) = apply_to(m.pinned_on.clone(), m.not_today_on.clone());
                    Ok(Row::Milestone(Milestone {
                        pinned_on: p,
                        not_today_on: n,
                        ..m
                    }))
                }
                _ => Err(ErrorCode::NotFound),
            }
        }
        Op::SettingsUpdate {
            theme,
            accent,
            start_view,
            timezone,
            day_start,
            next_deadline_days,
            next_due_days,
            next_undated_p1,
            default_reminder,
        } => {
            let old = match existing {
                Some(Row::Settings(s)) => s,
                Some(_) => return Err(ErrorCode::BadField),
                None => Settings {
                    id: SETTINGS,
                    theme: String::new(),
                    accent: String::new(),
                    start_view: String::new(),
                    timezone: String::new(),
                    day_start: String::new(),
                    next_deadline_days: None,
                    next_due_days: None,
                    next_undated_p1: None,
                    default_reminder: None,
                },
            };
            Ok(Row::Settings(Settings {
                id: SETTINGS,
                theme: theme.clone().unwrap_or(old.theme),
                accent: accent.clone().unwrap_or(old.accent),
                start_view: start_view.clone().unwrap_or(old.start_view),
                timezone: timezone.clone().unwrap_or(old.timezone),
                day_start: day_start.clone().unwrap_or(old.day_start),
                next_deadline_days: next_deadline_days.or(old.next_deadline_days),
                next_due_days: next_due_days.or(old.next_due_days),
                next_undated_p1: next_undated_p1.or(old.next_undated_p1),
                default_reminder: default_reminder.or(old.default_reminder),
            }))
        }
    }
}

fn touch(g: &Goal, cmd: &Command) -> Option<u64> {
    Some(cmd.hlc.wall.max(g.touched_at.unwrap_or(0)))
}

/// A Target needs a positive finite target; dates must be `YYYY-MM-DD`.
fn check_goal_fields(
    progress: &Progress,
    start: Option<&str>,
    target_date: Option<&str>,
) -> Result<(), ErrorCode> {
    if matches!(progress, Progress::Target { target, .. } if !target.is_finite() || *target <= 0.0)
        || start.is_some_and(|d| !is_date(d))
        || target_date.is_some_and(|d| !is_date(d))
    {
        return Err(ErrorCode::BadField);
    }
    Ok(())
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
        // Sub-goal depth needs the grandparent: see `goal::check_nesting`.
        (Row::Goal(_), Row::Area(a)) => a.id != INBOX,
        (Row::Goal(_), Row::Goal(_)) => true,
        (Row::Milestone(_), Row::Goal(_)) => true,
        (Row::Page(_), Row::Area(a)) => a.id != INBOX,
        (Row::Page(_), Row::Project(_) | Row::Goal(_)) => true,
        (Row::Task(_), Row::Area(_) | Row::Project(_) | Row::Section(_) | Row::Task(_)) => true,
        (Row::Routine(_), Row::Area(a)) => a.id != INBOX,
        (Row::Routine(_), Row::Project(_)) => true,
        (Row::Checkin(_), Row::Routine(_)) => true,
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
    let existing = existing.or_else(|| view::builtin_for(&cmd.op));
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
    let mut fields = cmd.op.fields();
    // Completing a recurring Task moves its due on rather than setting `completed_at`.
    if matches!(cmd.op, Op::TaskComplete { .. }) && old["due"] != merged["due"] {
        fields.push("due");
    }
    for field in fields {
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

/// The completion side effect, shared by server and client prediction: the Task as it
/// is afterwards, and the record to log. A recurring Task gets its next `due` and stays open;
/// any other gets `completed_at`. Completing a finished Task again changes and logs nothing.
pub fn complete_effects(t: Task, cmd: &Command) -> (Task, Option<Completion>) {
    let at = cmd.hlc.wall;
    let log = |t: &Task| Completion {
        id: completion_id(t.id, cmd.uuid),
        item_id: t.id,
        at,
    };
    if let Some(rec) = &t.recurrence {
        let today = match &cmd.op {
            Op::TaskComplete { today: Some(d), .. } if is_date(d) => d.clone(),
            // No local date was sent: the UTC one.
            _ => parse::fmt_date((at / 86_400_000) as i64),
        };
        let from = t.due.clone().unwrap_or(Due {
            date: today.clone(),
            time: None,
        });
        let due = Some(recur::next(rec, &from, &today));
        let c = log(&t);
        return (Task { due, ..t }, Some(c));
    }
    if t.completed_at.is_some() {
        return (t, None);
    }
    let c = log(&t);
    (
        Task {
            completed_at: Some(at),
            ..t
        },
        Some(c),
    )
}

/// The completion record `cmd` logs when applied to `old`, for a Task, Goal or Milestone.
pub fn completion_for(old: &Row, cmd: &Command) -> Option<Completion> {
    let at = cmd.hlc.wall;
    let log = |item_id| Completion {
        id: completion_id(item_id, cmd.uuid),
        item_id,
        at,
    };
    match (&cmd.op, old) {
        (Op::TaskComplete { .. }, Row::Task(t)) => complete_effects(t.clone(), cmd).1,
        (Op::GoalComplete { .. }, Row::Goal(g)) if g.completed_at.is_none() => Some(log(g.id)),
        (
            Op::MilestoneUpdate {
                done: Some(true), ..
            },
            Row::Milestone(m),
        ) if !m.done => Some(log(m.id)),
        _ => None,
    }
}

/// The same command always logs the same record, so a replay cannot log it twice.
pub fn completion_id(task: Uuid, cmd: Uuid) -> Uuid {
    let id = task.as_u128() ^ cmd.as_u128().rotate_left(64);
    Builder::from_random_bytes(id.to_be_bytes()).into_uuid()
}

pub use routine::checkin_id;

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
                "description": "", "due": null, "deadline": null, "duration_min": null,
                "priority": 4, "label_ids": [], "recurrence": null, "goal_id": null, "created_at": 1000, "trashed_at": null,
                "pinned_on": null, "not_today_on": null,
                "links": [], "reminders": [], "blobs": [],
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

    fn recurring(rule: &str, from_completion: bool, due: &str) -> Row {
        let update = cmd(
            "01900000-0000-7000-8000-0000000000d1",
            "task.update",
            serde_json::json!({
                "id": TASK,
                "due": { "date": due, "time": "09:00" },
                "recurrence": { "rule": rule, "from_completion": from_completion },
            }),
            2000,
        );
        apply(Some(created()), &update).unwrap()
    }

    fn complete_on(today: Option<&str>, wall: u64) -> Command {
        cmd(
            "01900000-0000-7000-8000-0000000000c2",
            "task.complete",
            serde_json::json!({ "id": TASK, "today": today }),
            wall,
        )
    }

    #[test]
    fn completing_a_recurring_task_moves_its_due_on_and_logs_it() {
        let Row::Task(t) = recurring("week", false, "2026-11-19") else {
            panic!()
        };
        let c = complete_on(Some("2026-11-19"), 5000);
        let (after, log) = complete_effects(t.clone(), &c);
        assert_eq!(after.completed_at, None);
        assert_eq!(
            after.due,
            Some(Due {
                date: "2026-11-26".into(),
                time: Some("09:00".into())
            })
        );
        let log = log.unwrap();
        assert_eq!((log.item_id, log.at), (t.id, 5000));
        // The same command logs the same record; a different one does not.
        assert_eq!(complete_effects(t.clone(), &c).1.unwrap().id, log.id);
        let other = cmd(
            "01900000-0000-7000-8000-0000000000c9",
            "task.complete",
            serde_json::json!({ "id": TASK }),
            5000,
        );
        assert_ne!(complete_effects(t, &other).1.unwrap().id, log.id);
    }

    #[test]
    fn every_bang_counts_from_the_completers_today() {
        let Row::Task(t) = recurring("week", true, "2026-11-01") else {
            panic!()
        };
        let (after, _) = complete_effects(t.clone(), &complete_on(Some("2026-11-19"), 5000));
        assert_eq!(after.due.unwrap().date, "2026-11-26");
        // Without `today` the UTC date of the clock is used: 2026-11-19T12:00Z.
        let (after, _) = complete_effects(t, &complete_on(None, 1_795_089_600_000));
        assert_eq!(after.due.unwrap().date, "2026-11-26");
    }

    #[test]
    fn a_missed_recurring_task_stays_due_until_it_is_completed() {
        let row = recurring("day", false, "2026-11-18");
        let Row::Task(t) = &row else { panic!() };
        // Nothing runs on its own: the due date is still yesterday's.
        assert_eq!(t.due.as_ref().unwrap().date, "2026-11-18");
        assert_eq!(t.completed_at, None);
        let done = apply(Some(row), &complete_on(Some("2026-11-19"), 5000)).unwrap();
        let Row::Task(t) = done else { panic!() };
        assert_eq!(t.due.unwrap().date, "2026-11-19");
        assert_eq!(t.completed_at, None);
    }

    #[test]
    fn a_plain_task_completes_once_and_logs_once() {
        let Row::Task(t) = created() else { panic!() };
        let (done, log) = complete_effects(t, &complete(5000));
        assert_eq!(done.completed_at, Some(5000));
        assert!(log.is_some());
        let (_, again) = complete_effects(done, &complete(9000));
        assert!(again.is_none());
    }

    #[test]
    fn completing_a_recurring_task_wins_the_due_clock() {
        let row = recurring("day", false, "2026-11-18");
        let mut clocks = Clocks::new();
        clocks.insert("due".into(), complete(1).hlc);
        let done = apply_lww(
            Some(row),
            &mut clocks,
            &complete_on(Some("2026-11-19"), 5000),
        )
        .unwrap();
        let Row::Task(t) = done else { panic!() };
        assert_eq!(t.due.unwrap().date, "2026-11-19");
        assert_eq!(clocks["due"].wall, 5000);
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
                ("blobs", 1000),
                ("completed_at", 1000),
                ("created_at", 1000),
                ("deadline", 1000),
                ("description", 1000),
                ("due", 1000),
                ("duration_min", 1000),
                ("goal_id", 1000),
                ("label_ids", 1000),
                ("links", 1000),
                ("not_today_on", 1000),
                ("order", 1000),
                ("parent_id", 1000),
                ("pinned_on", 1000),
                ("priority", 1000),
                ("recurrence", 1000),
                ("reminders", 1000),
                ("title", 1000),
                ("trashed_at", 1000)
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
            trashed_at: None,
            archived_at: None,
        })
    }
    fn project(n: u128, parent: Uuid) -> Row {
        Row::Project(Project {
            id: uid(n),
            parent_id: parent,
            name: "P".into(),
            order: "a".into(),
            goal_id: None,
            trashed_at: None,
            archived_at: None,
            touched_at: None,
        })
    }
    fn section(n: u128, parent: Uuid) -> Row {
        Row::Section(Section {
            id: uid(n),
            parent_id: parent,
            name: "S".into(),
            order: "a".into(),
            trashed_at: None,
        })
    }
    fn task_row(n: u128, parent: Uuid) -> Row {
        Row::Task(Task {
            id: uid(n),
            parent_id: parent,
            title: "T".into(),
            completed_at: None,
            order: "a".into(),
            description: String::new(),
            due: None,
            deadline: None,
            duration_min: None,
            priority: 4,
            label_ids: vec![],
            recurrence: None,
            goal_id: None,
            created_at: None,
            trashed_at: None,
            pinned_on: None,
            not_today_on: None,
            links: vec![],
            reminders: vec![],
            blobs: vec![],
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
                other => other,
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

    fn update(args: serde_json::Value) -> Command {
        let mut args = args;
        args["id"] = serde_json::json!(TASK);
        cmd(
            "01900000-0000-7000-8000-0000000000d1",
            "task.update",
            args,
            2000,
        )
    }

    fn updated(args: serde_json::Value) -> Task {
        let Row::Task(t) = apply(Some(created()), &update(args)).unwrap() else {
            panic!()
        };
        t
    }

    #[test]
    fn a_new_task_has_default_fields() {
        let Row::Task(t) = created() else { panic!() };
        assert_eq!(t.description, "");
        assert_eq!((t.due, t.deadline, t.duration_min), (None, None, None));
        assert_eq!(t.priority, 4);
        assert!(t.label_ids.is_empty());
    }

    #[test]
    fn an_old_task_row_without_the_new_fields_still_parses() {
        let row: Row = serde_json::from_value(serde_json::json!({
            "kind": "task", "id": TASK, "parent_id": INBOX, "title": "Old", "completed_at": null,
        }))
        .unwrap();
        let Row::Task(t) = row else { panic!() };
        assert_eq!((t.priority, t.description.as_str()), (4, ""));
    }

    #[test]
    fn task_update_sets_each_field() {
        let label = uid(7);
        let t = updated(serde_json::json!({
            "description": "Two litres", "due": { "date": "2026-03-04", "time": "09:30" },
            "deadline": "2026-03-10", "duration_min": 25, "priority": 1, "label_ids": [label],
        }));
        assert_eq!(t.description, "Two litres");
        assert_eq!(
            t.due,
            Some(Due {
                date: "2026-03-04".into(),
                time: Some("09:30".into())
            })
        );
        assert_eq!(t.deadline.as_deref(), Some("2026-03-10"));
        assert_eq!((t.duration_min, t.priority), (Some(25), 1));
        assert_eq!(t.label_ids, vec![label]);
        let date_only = updated(serde_json::json!({ "due": { "date": "2026-03-04" } }));
        assert_eq!(date_only.due.unwrap().time, None);
    }

    #[test]
    fn task_update_null_clears_optional_fields_and_absent_leaves_them() {
        let set = updated(serde_json::json!({
            "due": { "date": "2026-03-04" }, "deadline": "2026-03-10", "duration_min": 25,
            "priority": 2,
        }));
        let step = |args| {
            let Row::Task(t) = apply(Some(Row::Task(set.clone())), &update(args)).unwrap() else {
                panic!()
            };
            t
        };
        let cleared =
            step(serde_json::json!({ "due": null, "deadline": null, "duration_min": null }));
        assert_eq!(
            (cleared.due, cleared.deadline, cleared.duration_min),
            (None, None, None)
        );
        assert_eq!(cleared.priority, 2);
        let untouched = step(serde_json::json!({ "title": "Renamed" }));
        assert_eq!(
            Task {
                title: "Buy milk".into(),
                ..untouched
            },
            set
        );
    }

    #[test]
    fn task_update_fields_lists_exactly_the_fields_present() {
        let c = update(serde_json::json!({ "due": null, "priority": 3 }));
        assert_eq!(c.op.fields(), vec!["due", "priority"]);
        let round: Command = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(round.op.fields(), vec!["due", "priority"]);
    }

    #[test]
    fn task_update_rejects_bad_values() {
        for bad in [
            serde_json::json!({ "priority": 0 }),
            serde_json::json!({ "priority": 5 }),
            serde_json::json!({ "due": { "date": "2026-3-4" } }),
            serde_json::json!({ "due": { "date": "2026-13-04" } }),
            serde_json::json!({ "due": { "date": "2026-03-04", "time": "9:30" } }),
            serde_json::json!({ "due": { "date": "2026-03-04", "time": "24:00" } }),
            serde_json::json!({ "deadline": "tomorrow" }),
        ] {
            assert_eq!(
                apply(Some(created()), &update(bad.clone())),
                Err(ErrorCode::BadField),
                "{bad}"
            );
        }
        assert_eq!(
            apply(
                Some(created()),
                &update(serde_json::json!({ "title": " " }))
            ),
            Err(ErrorCode::TitleRequired)
        );
    }

    #[test]
    fn label_create_trims_the_name_and_rejects_blank_or_duplicate_ids() {
        let create = |name: &str| {
            cmd(
                "01900000-0000-7000-8000-0000000000e1",
                "label.create",
                serde_json::json!({ "id": uid(8), "name": name }),
                1000,
            )
        };
        let row = apply(None, &create("  errand ")).unwrap();
        assert_eq!(
            row,
            Row::Label(Label {
                id: uid(8),
                name: "errand".into()
            })
        );
        assert_eq!(check_parent(&row, None), Ok(()));
        assert_eq!(apply(None, &create(" ")), Err(ErrorCode::TitleRequired));
        assert_eq!(apply(Some(row), &create("x")), Err(ErrorCode::Exists));
    }

    #[test]
    fn settings_update_creates_the_row_then_changes_only_the_fields_present() {
        let up = |args| {
            cmd(
                "01900000-0000-7000-8000-0000000000e2",
                "settings.update",
                args,
                1000,
            )
        };
        let first = apply(None, &up(serde_json::json!({ "theme": "dracula" }))).unwrap();
        assert_eq!(
            first,
            Row::Settings(Settings {
                id: SETTINGS,
                theme: "dracula".into(),
                accent: String::new(),
                start_view: String::new(),
                timezone: String::new(),
                day_start: String::new(),
                next_deadline_days: None,
                next_due_days: None,
                next_undated_p1: None,
                default_reminder: None
            })
        );
        let second = apply(Some(first), &up(serde_json::json!({ "accent": "#ff0000" }))).unwrap();
        let Row::Settings(s) = second else { panic!() };
        assert_eq!(
            (s.theme.as_str(), s.accent.as_str()),
            ("dracula", "#ff0000")
        );
        assert_eq!(
            up(serde_json::json!({ "theme": "x" })).op.target(),
            SETTINGS
        );
    }

    #[test]
    fn pin_and_not_today_clear_each_other_and_null_clears() {
        let pin = |date: serde_json::Value, op: &str| {
            cmd(
                "01900000-0000-7000-8000-0000000000d1",
                op,
                serde_json::json!({ "id": TASK, "date": date }),
                2000,
            )
        };
        let pinned = apply(Some(created()), &pin("2026-03-10".into(), "item.pin")).unwrap();
        assert_eq!(task(&pinned).pinned_on.as_deref(), Some("2026-03-10"));
        let hidden = apply(Some(pinned), &pin("2026-03-10".into(), "item.not_today")).unwrap();
        assert_eq!(task(&hidden).pinned_on, None);
        assert_eq!(task(&hidden).not_today_on.as_deref(), Some("2026-03-10"));
        let cleared = apply(
            Some(hidden),
            &pin(serde_json::Value::Null, "item.not_today"),
        )
        .unwrap();
        assert_eq!(task(&cleared).not_today_on, None);
        assert_eq!(
            apply(Some(cleared), &pin("nope".into(), "item.pin")),
            Err(ErrorCode::BadField)
        );
    }

    #[test]
    fn body_writes_set_the_link_index() {
        const L: &str = "00000000-0000-7000-8000-00000000000a";
        let body = format!("see [Foo](orbit:{L})");
        let create = cmd(
            "01900000-0000-7000-8000-0000000000c2",
            "page.create",
            serde_json::json!({ "id": TASK, "parent_id": INBOX, "title": "P", "body": body, "order": "a" }),
            1000,
        );
        let Row::Page(p) = apply(None, &create).unwrap() else {
            panic!("a page")
        };
        assert_eq!(p.links, vec![L.parse::<Uuid>().unwrap()]);
        let edit = cmd(
            "01900000-0000-7000-8000-0000000000c3",
            "page.edit_body",
            serde_json::json!({ "id": TASK, "body": "none", "base_rev": 0 }),
            1001,
        );
        let Row::Page(p) = apply(Some(Row::Page(p)), &edit).unwrap() else {
            panic!("a page")
        };
        assert!(p.links.is_empty());
        let t = updated(serde_json::json!({ "description": &body }));
        assert_eq!(t.links, vec![L.parse::<Uuid>().unwrap()]);
        assert_eq!(
            updated(serde_json::json!({ "title": "x" })).links,
            Vec::<Uuid>::new()
        );
    }
}
