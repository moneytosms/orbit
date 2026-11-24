//! Turns a Snapshot plus the (edited) project mapping into normal domain commands, run through the
//! same path as `POST /sync`. The import map (Todoist id to Orbit id) makes re-runs create-only.
use super::api::{Comment, Snapshot, Task};
use crate::{now_ms, run_command};
use orbit_core::{
    Command, Due, Hlc, INBOX, Op, Status, Uuid,
    order::between,
    parse::{Known, parse},
    reminder::{Anchor, Reminder, Trigger},
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

/// The Device id of every command the import emits.
const IMPORT_DEVICE: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a1);
pub const RECURRENCE_LABEL: &str = "imported-recurrence";
const DROPPED: [&str; 5] = [
    "saved filters",
    "karma (Stardust starts at 0)",
    "completed and archived items",
    "collaborators and assignees",
    "location reminders",
];

/// Todoist's named palette as `#rrggbb`; an unknown name is no colour.
fn hex(name: &str) -> String {
    let h = match name {
        "berry_red" => "b8255f",
        "red" => "dc4c3e",
        "orange" => "c77100",
        "yellow" => "b29104",
        "olive_green" => "949c31",
        "lime_green" => "65a33a",
        "green" => "369307",
        "mint_green" => "42a393",
        "teal" => "148fad",
        "sky_blue" => "319dc0",
        "light_blue" => "6988a4",
        "blue" => "4180ff",
        "grape" => "692ec2",
        "violet" => "ca3fee",
        "lavender" => "a4698c",
        "magenta" => "e05095",
        "salmon" => "c9766f",
        "charcoal" => "808080",
        "grey" => "999999",
        "taupe" => "8f7a69",
        _ => return String::new(),
    };
    format!("#{h}")
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Area,
    Project,
    Inbox,
}

/// Where one Todoist project goes. The preview lists these; the commit takes them back edited.
/// `area` is the Area's name (the Area itself for `area`, the Project's Area for `project`);
/// `name` is the Project's name. The counts are for the preview only.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Plan {
    pub id: String,
    pub kind: Kind,
    pub area: String,
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub tasks: usize,
    #[serde(default)]
    pub sections: usize,
    /// Tasks already imported by an earlier run.
    #[serde(default)]
    pub imported: usize,
}

/// Top-level Todoist project to Area, child to Project, deeper to a Project named `Parent / Child`
/// in the top-level one's Area; Inbox to Inbox. In tree order.
pub fn default_plans(s: &Snapshot, map: &HashMap<String, Uuid>) -> Vec<Plan> {
    let by_id: HashMap<&str, _> = s.projects.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut chains: Vec<Vec<_>> = s
        .projects
        .iter()
        .map(|p| {
            let mut chain = vec![p];
            // The cap guards against a parent cycle.
            while let Some(parent) = chain
                .last()
                .and_then(|c| c.parent_id.as_deref())
                .and_then(|i| by_id.get(i))
            {
                if chain.len() > 32 {
                    break;
                }
                chain.push(parent);
            }
            chain.reverse();
            chain
        })
        .collect();
    chains.sort_by_key(|c| {
        c.iter()
            .map(|p| (p.child_order, p.id.clone()))
            .collect::<Vec<_>>()
    });
    chains
        .iter()
        .map(|c| {
            let p = c[c.len() - 1];
            let (kind, area, name) = match c.len() {
                _ if p.inbox_project => (Kind::Inbox, String::new(), String::new()),
                1 => (Kind::Area, p.name.clone(), String::new()),
                2 => (Kind::Project, c[0].name.clone(), p.name.clone()),
                n => (
                    Kind::Project,
                    c[0].name.clone(),
                    format!("{} / {}", c[n - 2].name, p.name),
                ),
            };
            let tasks = s.tasks.iter().filter(|t| t.project_id == p.id);
            Plan {
                id: p.id.clone(),
                kind,
                area,
                name,
                path: c
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" / "),
                tasks: tasks.clone().count(),
                sections: s.sections.iter().filter(|x| x.project_id == p.id).count(),
                imported: tasks
                    .filter(|t| map.contains_key(&key("task", &t.id)))
                    .count(),
            }
        })
        .collect()
}

fn key(kind: &str, id: &str) -> String {
    format!("todoist/{kind}/{id}")
}

pub fn load_map(conn: &Connection) -> rusqlite::Result<HashMap<String, Uuid>> {
    let mut stmt = conn.prepare("SELECT key, id FROM import_map")?;
    Ok(stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .filter_map(|(k, id)| Some((k, Uuid::parse_str(&id).ok()?)))
        .collect())
}

#[derive(Serialize, Default)]
pub struct Report {
    pub areas: usize,
    pub projects: usize,
    pub sections: usize,
    pub tasks: usize,
    pub labels: usize,
    pub pages: usize,
    pub reminders: usize,
    /// Tasks skipped because an earlier run already imported them.
    pub existing_tasks: usize,
    /// Things that came over changed or not at all.
    pub issues: Vec<String>,
    pub dropped: Vec<&'static str>,
}

struct Emit<'a> {
    conn: &'a mut Connection,
    data: &'a Path,
    now: i64,
    counter: u32,
    /// The last order key handed out under each parent (`None` for Areas).
    last: HashMap<Option<Uuid>, Option<String>>,
}

impl Emit<'_> {
    /// Runs one command; false if the server rejected it.
    fn send(&mut self, id: Uuid, op: Op) -> rusqlite::Result<bool> {
        self.counter += 1;
        let cmd = Command {
            uuid: id,
            op,
            hlc: Hlc {
                wall: self.now as u64,
                counter: self.counter,
                device: IMPORT_DEVICE,
            },
        };
        Ok(run_command(self.conn, self.data, &cmd, self.now)? == Status::Ok)
    }

    fn id(&self) -> Uuid {
        let mut r = [0u8; 10];
        getrandom::fill(&mut r).expect("OS randomness");
        orbit_core::new_id(self.now as u64, &r)
    }

    /// An order key after every sibling under `parent`, existing or handed out by this run.
    fn next_order(&mut self, parent: Option<Uuid>) -> rusqlite::Result<String> {
        if !self.last.contains_key(&parent) {
            let max: Option<String> = match parent {
                Some(p) => self.conn.query_row(
                    "SELECT MAX(json_extract(data, '$.order')) FROM rows WHERE json_extract(data, '$.parent_id') = ?",
                    [p.to_string()],
                    |r| r.get(0),
                )?,
                None => self.conn.query_row(
                    "SELECT MAX(json_extract(data, '$.order')) FROM rows WHERE json_extract(data, '$.kind') = 'area'",
                    [],
                    |r| r.get(0),
                )?,
            };
            self.last.insert(parent, max);
        }
        let last = self.last.get_mut(&parent).unwrap();
        let next = between(last.as_deref(), None);
        *last = Some(next.clone());
        Ok(next)
    }

    fn remember(&self, key: &str, id: Uuid) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO import_map (key, id) VALUES (?, ?)",
            (key, id.to_string()),
        )?;
        Ok(())
    }

    /// Live rows of one kind as `(lowercase name, id)`.
    fn names(&self, kind: &str) -> rusqlite::Result<HashMap<String, Uuid>> {
        let mut stmt = self.conn.prepare(
            "SELECT json_extract(data, '$.name'), id FROM rows
             WHERE json_extract(data, '$.kind') = ? AND json_extract(data, '$.trashed_at') IS NULL AND id != ?",
        )?;
        Ok(stmt
            .query_map((kind, INBOX.to_string()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(n, id)| Some((n.to_lowercase(), Uuid::parse_str(&id).ok()?)))
            .collect())
    }
}

fn md_link(f: &super::api::File) -> String {
    let target = match &f.blob {
        Some(h) => format!("blob:{h}"),
        None => f.url.clone(),
    };
    format!(
        "{}[{}]({target})",
        if f.image && f.blob.is_some() { "!" } else { "" },
        f.name
    )
}

fn md_comment(c: &Comment) -> String {
    let mut s = format!("**{}**", c.date);
    if !c.content.is_empty() {
        s.push_str(&format!(": {}", c.content));
    }
    if let Some(f) = &c.file {
        s.push_str(&format!("\n\n{}", md_link(f)));
    }
    s
}

/// `YYYY-MM-DD[THH:MM...]` as an Orbit due; the clock part is taken as written (floating).
fn due_of(date: &str) -> Due {
    Due {
        date: date.chars().take(10).collect(),
        time: date.get(11..16).map(String::from),
    }
}

fn blank_update(id: Uuid) -> Op {
    Op::TaskUpdate {
        id,
        title: None,
        description: None,
        due: None,
        deadline: None,
        duration_min: None,
        priority: None,
        label_ids: None,
        recurrence: None,
        goal_id: None,
    }
}

/// Runs the import. `edits` override the default mapping by Todoist project id.
pub fn run(
    conn: &mut Connection,
    data: &Path,
    s: &Snapshot,
    edits: &[Plan],
) -> rusqlite::Result<Report> {
    let mut map = load_map(conn)?;
    let mut plans: HashMap<String, Plan> = default_plans(s, &map)
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();
    for e in edits {
        if let Some(p) = plans.get_mut(&e.id) {
            let (area, name) = (e.area.trim(), e.name.trim());
            // A blank name keeps the default; Inbox needs none.
            if e.kind == Kind::Inbox
                || (!area.is_empty() && (e.kind == Kind::Area || !name.is_empty()))
            {
                p.kind = e.kind;
                p.area = area.into();
                p.name = name.into();
            }
        }
    }
    let mut r = Report {
        dropped: DROPPED.to_vec(),
        ..Default::default()
    };
    let mut e = Emit {
        now: now_ms(),
        counter: 0,
        last: HashMap::new(),
        conn,
        data,
    };
    let mut areas = e.names("area")?;
    let mut labels = e.names("label")?;

    // Labels merged by name.
    let mut label_id =
        |e: &mut Emit, r: &mut Report, name: &str, color: &str| -> rusqlite::Result<Option<Uuid>> {
            if let Some(id) = labels.get(&name.to_lowercase()) {
                return Ok(Some(*id));
            }
            let id = e.id();
            if !e.send(
                id,
                Op::LabelCreate {
                    color: hex(color),
                    id,
                    name: name.into(),
                },
            )? {
                r.issues.push(format!("label {name:?} was rejected"));
                return Ok(None);
            }
            labels.insert(name.to_lowercase(), id);
            r.labels += 1;
            Ok(Some(id))
        };
    let task_labels = s.tasks.iter().flat_map(|t| &t.labels);
    for (name, color) in s
        .labels
        .iter()
        .map(|l| (l.name.as_str(), l.color.as_str()))
        .chain(task_labels.map(|n| (n.as_str(), "")))
    {
        label_id(&mut e, &mut r, name, color)?;
    }

    // Projects: where each Todoist project's tasks land, `None` parent being the Inbox.
    let mut home: HashMap<&str, Option<Uuid>> = HashMap::new();
    let mut in_inbox: HashSet<&str> = HashSet::new();
    let mut order: Vec<&Plan> = default_plans(s, &map)
        .iter()
        .filter_map(|p| plans.get(&p.id))
        .collect();
    order.sort_by_key(|p| p.kind != Kind::Area);
    let colors: HashMap<&str, &str> = s
        .projects
        .iter()
        .map(|p| (p.id.as_str(), p.color.as_str()))
        .collect();
    // An Area takes the colour of its top-level Todoist project, a Project its own.
    let color_of = |p: &Plan, want: Kind| {
        if p.kind == want {
            colors.get(p.id.as_str()).copied().unwrap_or("")
        } else {
            ""
        }
    };
    for p in order {
        if p.kind == Kind::Inbox {
            in_inbox.insert(&p.id);
            home.insert(&p.id, None);
            continue;
        }
        let area = match areas.get(&p.area.to_lowercase()) {
            Some(a) => *a,
            None => {
                let id = e.id();
                let order = e.next_order(None)?;
                if !e.send(
                    id,
                    Op::AreaCreate {
                        color: hex(color_of(p, Kind::Area)),
                        id,
                        name: p.area.clone(),
                        order,
                    },
                )? {
                    r.issues.push(format!(
                        "area {:?} was rejected, so its tasks went to the Inbox",
                        p.area
                    ));
                    in_inbox.insert(&p.id);
                    home.insert(&p.id, None);
                    continue;
                }
                areas.insert(p.area.to_lowercase(), id);
                r.areas += 1;
                id
            }
        };
        let id = match (p.kind, map.get(&key("project", &p.id))) {
            (Kind::Area, _) => {
                e.remember(&key("project", &p.id), area)?;
                map.insert(key("project", &p.id), area);
                area
            }
            (_, Some(id)) => *id,
            _ => {
                let id = e.id();
                let order = e.next_order(Some(area))?;
                if !e.send(
                    id,
                    Op::ProjectCreate {
                        color: hex(color_of(p, Kind::Project)),
                        id,
                        parent_id: area,
                        name: p.name.clone(),
                        order,
                    },
                )? {
                    r.issues.push(format!(
                        "project {:?} was rejected, so its tasks went to the Inbox",
                        p.name
                    ));
                    in_inbox.insert(&p.id);
                    home.insert(&p.id, None);
                    continue;
                }
                e.remember(&key("project", &p.id), id)?;
                map.insert(key("project", &p.id), id);
                r.projects += 1;
                id
            }
        };
        home.insert(&p.id, Some(id));
    }

    // Sections, then Project comments as one Page each. The Inbox holds neither.
    let mut sections = s.sections.clone();
    sections.sort_by_key(|x| x.section_order);
    let mut section_ids: HashMap<&str, Uuid> = HashMap::new();
    for x in &sections {
        let Some(Some(parent)) = home.get(x.project_id.as_str()) else {
            if in_inbox.contains(x.project_id.as_str()) {
                r.issues.push(format!(
                    "section {:?} dropped: the Inbox holds only Tasks",
                    x.name
                ));
            }
            continue;
        };
        if let Some(id) = map.get(&key("section", &x.id)) {
            section_ids.insert(&x.id, *id);
            continue;
        }
        let id = e.id();
        let order = e.next_order(Some(*parent))?;
        if e.send(
            id,
            Op::SectionCreate {
                id,
                parent_id: *parent,
                name: x.name.clone(),
                order,
            },
        )? {
            e.remember(&key("section", &x.id), id)?;
            section_ids.insert(&x.id, id);
            r.sections += 1;
        } else {
            r.issues.push(format!("section {:?} was rejected", x.name));
        }
    }
    for p in &s.projects {
        let Some(comments) = s.project_comments.get(&p.id) else {
            continue;
        };
        match home.get(p.id.as_str()) {
            Some(Some(parent)) if !map.contains_key(&key("comments", &p.id)) => {
                let id = e.id();
                let order = e.next_order(Some(*parent))?;
                let body = comments
                    .iter()
                    .map(md_comment)
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if e.send(
                    id,
                    Op::PageCreate {
                        id,
                        parent_id: *parent,
                        title: "Todoist comments".into(),
                        body,
                        order,
                    },
                )? {
                    e.remember(&key("comments", &p.id), id)?;
                    r.pages += 1;
                } else {
                    r.issues
                        .push(format!("comments of project {:?} were rejected", p.name));
                }
            }
            Some(None) => r.issues.push(format!(
                "comments of project {:?} dropped: the Inbox holds only Tasks",
                p.name
            )),
            _ => {}
        }
    }

    // Tasks, parents first, siblings in Todoist order.
    let by_id: HashMap<&str, &Task> = s.tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let depth = |t: &Task| {
        let (mut n, mut cur) = (0, t);
        while let Some(p) = cur.parent_id.as_deref().and_then(|i| by_id.get(i)) {
            n += 1;
            cur = p;
            if n > 32 {
                break;
            }
        }
        n
    };
    let mut tasks: Vec<&Task> = s.tasks.iter().collect();
    tasks.sort_by_key(|t| (depth(t), t.child_order));
    for t in tasks {
        let Some(&container) = home.get(t.project_id.as_str()) else {
            r.issues.push(format!(
                "task {:?} skipped: its project was not in the list",
                t.content
            ));
            continue;
        };
        if map.contains_key(&key("task", &t.id)) {
            r.existing_tasks += 1;
            continue;
        }
        let parent = t
            .parent_id
            .as_deref()
            .and_then(|p| map.get(&key("task", p)).copied())
            .or_else(|| {
                t.section_id
                    .as_deref()
                    .and_then(|x| section_ids.get(x).copied())
            })
            .or(container);
        let id = e.id();
        let order = e.next_order(Some(parent.unwrap_or(INBOX)))?;
        if !e.send(
            id,
            Op::TaskCreate {
                id,
                title: t.content.clone(),
                parent_id: parent,
                order,
            },
        )? {
            r.issues.push(format!("task {:?} was rejected", t.content));
            continue;
        }
        e.remember(&key("task", &t.id), id)?;
        map.insert(key("task", &t.id), id);
        r.tasks += 1;

        let mut label_ids = Vec::new();
        for name in &t.labels {
            label_ids.extend(label_id(&mut e, &mut r, name, "")?);
        }
        let mut description = t.description.clone();
        let mut recurrence = None;
        if let Some(due) = t.due.as_ref().filter(|d| d.is_recurring)
            && let Some(text) = &due.string
        {
            // `every!` is read by the core parser as after-completion. Words it leaves over mean it
            // only understood a prefix (`every 2nd tuesday` reads as the 2nd of the month), so no recurrence.
            let parsed = parse(
                text,
                &format!("{}T00:00", due_of(&due.date).date),
                &Known::default(),
            );
            match parsed.recurrence.filter(|_| parsed.title.is_empty()) {
                Some(rec) => recurrence = Some(rec),
                None => {
                    description = format!("{description}\n\nOriginal recurrence: {text}")
                        .trim()
                        .to_string();
                    label_ids.extend(label_id(&mut e, &mut r, RECURRENCE_LABEL, "")?);
                    r.issues.push(format!(
                        "recurrence {text:?} of task {:?} not understood: kept as a dated Task",
                        t.content
                    ));
                }
            }
        }
        if let Some(cs) = s.task_comments.get(&t.id) {
            let body = cs.iter().map(md_comment).collect::<Vec<_>>().join("\n\n");
            description = format!("{description}\n\n## Todoist comments\n\n{body}")
                .trim()
                .to_string();
            for c in cs {
                if let Some(f) = c.file.as_ref().filter(|f| f.blob.is_none()) {
                    r.issues.push(format!(
                        "attachment {:?} of task {:?} not downloaded: URL kept",
                        f.name, t.content
                    ));
                }
            }
        }
        let minutes = t.duration.as_ref().and_then(|d| match d.unit.as_str() {
            "minute" => Some(d.amount),
            "day" => Some(d.amount * 24 * 60),
            _ => None,
        });
        let mut update = blank_update(id);
        if let Op::TaskUpdate {
            description: d,
            due,
            deadline,
            duration_min,
            priority,
            label_ids: l,
            recurrence: rec,
            ..
        } = &mut update
        {
            *d = (!description.is_empty()).then_some(description);
            *due = t.due.as_ref().map(|x| Some(due_of(&x.date)));
            *deadline = t.deadline.as_ref().map(|x| Some(x.date.clone()));
            *duration_min = minutes.map(Some);
            // Todoist 4 is the app's p1; Orbit's P1 is 1.
            *priority = Some(5 - t.priority.clamp(1, 4)).filter(|p| *p != 4);
            *l = (!label_ids.is_empty()).then_some(label_ids);
            *rec = recurrence.map(Some);
        }
        if !e.send(e.id(), update)? {
            r.issues
                .push(format!("fields of task {:?} were rejected", t.content));
        }
        for rem in s.reminders.iter().filter(|x| x.item_id == t.id) {
            let trigger = match (rem.kind.as_str(), rem.minute_offset, rem.due.as_ref()) {
                ("relative", Some(minutes), _) => Some(Trigger::Before {
                    field: Anchor::Due,
                    minutes,
                }),
                ("absolute", _, Some(d)) if d.date.len() >= 16 => Some(Trigger::At {
                    at: d.date.chars().take(16).collect(),
                }),
                _ => None,
            };
            let added = match trigger {
                Some(trigger) => {
                    let reminder = Reminder {
                        id: e.id(),
                        trigger,
                        spent: false,
                    };
                    e.send(
                        e.id(),
                        Op::ReminderAdd {
                            task_id: id,
                            reminder,
                        },
                    )?
                }
                None => false,
            };
            if added {
                r.reminders += 1;
            } else {
                r.issues.push(format!(
                    "{} reminder of task {:?} skipped",
                    rem.kind, t.content
                ));
            }
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::todoist::api::File;
    use serde_json::json;

    fn file(image: bool, blob: Option<&str>) -> File {
        File {
            name: "shot.png".into(),
            image,
            url: "https://x/shot.png".into(),
            blob: blob.map(String::from),
        }
    }

    #[test]
    fn md_link_embeds_images_only_once_downloaded() {
        assert_eq!(md_link(&file(true, Some("ab"))), "![shot.png](blob:ab)");
        assert_eq!(md_link(&file(false, Some("ab"))), "[shot.png](blob:ab)");
        assert_eq!(md_link(&file(true, None)), "[shot.png](https://x/shot.png)");
    }

    #[test]
    fn md_comment_has_the_date_then_text_then_attachment() {
        let c = |content: &str, file| Comment {
            date: "2026-03-01".into(),
            content: content.into(),
            file,
        };
        assert_eq!(md_comment(&c("hi", None)), "**2026-03-01**: hi");
        assert_eq!(md_comment(&c("", None)), "**2026-03-01**");
        assert_eq!(
            md_comment(&c("hi", Some(file(false, None)))),
            "**2026-03-01**: hi\n\n[shot.png](https://x/shot.png)"
        );
    }

    fn titles(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare(
                "SELECT json_extract(data, '$.title') FROM rows
                 WHERE json_extract(data, '$.kind') = 'task' ORDER BY 1",
            )
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    /// A rejected task mid-run (a blank title) is reported and not remembered, so a re-run
    /// creates just that task once its content is fixed and never duplicates the others.
    #[test]
    fn a_rejected_task_mid_run_is_retried_without_duplicating_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let _app = crate::app(dir.path()).unwrap();
        let mut conn = crate::open(dir.path()).unwrap();
        let task = |id: &str, content: &str, order: i64| json!({ "id": id, "project_id": "pi", "content": content, "child_order": order });
        let mut snap = Snapshot {
            projects: serde_json::from_value(
                json!([{ "id": "pi", "name": "Inbox", "inbox_project": true }]),
            )
            .unwrap(),
            tasks: serde_json::from_value(json!([
                task("a", "A", 1),
                task("b", "  ", 2),
                task("c", "C", 3)
            ]))
            .unwrap(),
            ..Default::default()
        };

        let first = run(&mut conn, dir.path(), &snap, &[]).unwrap();
        assert_eq!((first.tasks, first.existing_tasks), (2, 0));
        assert_eq!(first.issues.len(), 1, "{:?}", first.issues);
        assert!(
            first.issues[0].contains("was rejected"),
            "{:?}",
            first.issues
        );
        assert_eq!(titles(&conn), ["A", "C"]);

        snap.tasks[1].content = "B".into();
        let second = run(&mut conn, dir.path(), &snap, &[]).unwrap();
        assert_eq!((second.tasks, second.existing_tasks), (1, 2));
        assert!(second.issues.is_empty(), "{:?}", second.issues);
        assert_eq!(titles(&conn), ["A", "B", "C"]);

        let third = run(&mut conn, dir.path(), &snap, &[]).unwrap();
        assert_eq!((third.tasks, third.existing_tasks), (0, 3));
        assert_eq!(titles(&conn), ["A", "B", "C"]);
    }
}
