//! Views: saved filters over Tasks, Projects, Goals and Pages, and the pure engine that runs them.
use crate::goal::effective_goal;
use crate::parse::{fmt_date, parse_ymd};
use crate::{Command, ErrorCode, INBOX, Op, Row, Uuid, roadmap, trash};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Deepest nesting of filter groups, the root group being level 1.
pub const MAX_DEPTH: usize = 3;

const DAY_MS: u64 = 86_400_000;

/// Walk limit for ancestors, a guard against cycles in bad data.
const MAX_WALK: usize = 64;

/// What a View lists. A Plan is a Project or a Goal.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ViewKind {
    Task,
    Project,
    Goal,
    Page,
    Plan,
    Roadmap,
    Phase,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Sort {
    pub field: String,
    #[serde(default)]
    pub desc: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CondOp {
    Is,
    IsNot,
    Contains,
    IsEmpty,
    Before,
    After,
    Within,
}

/// A filter tree: groups of conditions, nested up to `MAX_DEPTH` levels.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Node {
    /// `all`: every child must match; otherwise any one. An empty group matches everything.
    Group { all: bool, children: Vec<Node> },
    Cond {
        path: String,
        op: CondOp,
        #[serde(default)]
        value: Value,
    },
}

/// A saved view. Its `kind` is stored as `entity`: `kind` is the tag of a `Row`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct View {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(rename = "entity")]
    pub kind: ViewKind,
    /// Only rows under this Area, Project or Goal.
    #[serde(default)]
    pub scope: Option<Uuid>,
    pub filter: Node,
    #[serde(default)]
    pub sorts: Vec<Sort>,
    #[serde(default)]
    pub group_by: Option<String>,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub show_completed: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub order: String,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// The key of the built-in this row customises.
    #[serde(default)]
    pub builtin: Option<String>,
}

fn default_layout() -> String {
    "list".into()
}

/// One group of a run: its key, and the ids in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Group {
    pub key: String,
    pub ids: Vec<Uuid>,
}

/// How many group levels `node` nests.
pub fn depth(node: &Node) -> usize {
    match node {
        Node::Group { children, .. } => 1 + children.iter().map(depth).max().unwrap_or(0),
        Node::Cond { .. } => 0,
    }
}

fn match_all() -> Node {
    Node::Group {
        all: true,
        children: vec![],
    }
}

fn check(v: &View) -> Result<(), ErrorCode> {
    if depth(&v.filter) > MAX_DEPTH {
        return Err(ErrorCode::BadField);
    }
    Ok(())
}

// ---- Built-ins ----

pub const VIEW_INBOX: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0101);
pub const VIEW_UPCOMING: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0102);
pub const VIEW_SLIPPING: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0103);
pub const VIEW_RECENT: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0104);
pub const VIEW_HORIZON: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0105);
pub const VIEW_CALENDAR: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_0106);

fn cond(path: &str, op: CondOp, value: Value) -> Node {
    Node::Cond {
        path: path.into(),
        op,
        value,
    }
}

fn sort(field: &str, desc: bool) -> Vec<Sort> {
    vec![Sort {
        field: field.into(),
        desc,
    }]
}

#[allow(clippy::too_many_arguments)]
fn builtin(
    id: Uuid,
    key: &str,
    name: &str,
    icon: &str,
    kind: ViewKind,
    filter: Node,
    sorts: Vec<Sort>,
    group_by: Option<&str>,
    show_completed: bool,
    order: &str,
) -> View {
    View {
        id,
        name: name.into(),
        icon: icon.into(),
        kind,
        scope: None,
        filter,
        sorts,
        group_by: group_by.map(Into::into),
        layout: default_layout(),
        fields: vec![],
        show_completed,
        pinned: true,
        hidden: false,
        order: order.into(),
        trashed_at: None,
        builtin: Some(key.into()),
    }
}

/// The default views every account has, until it customises them.
pub fn builtins() -> Vec<View> {
    let all = |children| Node::Group {
        all: true,
        children,
    };
    vec![
        builtin(
            VIEW_INBOX,
            "inbox",
            "Inbox",
            "inbox",
            ViewKind::Task,
            all(vec![cond("parent", CondOp::Is, json!(INBOX.to_string()))]),
            vec![],
            None,
            false,
            "1",
        ),
        builtin(
            VIEW_UPCOMING,
            "upcoming",
            "Upcoming",
            "calendar",
            ViewKind::Task,
            all(vec![cond("due", CondOp::Before, json!("+8d"))]),
            sort("due", false),
            Some("due"),
            false,
            "2",
        ),
        builtin(
            VIEW_SLIPPING,
            "slipping",
            "Slipping",
            "alert",
            ViewKind::Plan,
            Node::Group {
                all: false,
                children: vec![
                    cond("touched_at", CondOp::Before, json!("-14d")),
                    cond("touched_at", CondOp::IsEmpty, Value::Null),
                ],
            },
            sort("touched_at", false),
            None,
            false,
            "3",
        ),
        builtin(
            VIEW_RECENT,
            "recent",
            "Recently captured",
            "clock",
            ViewKind::Task,
            all(vec![cond("created_at", CondOp::After, json!("-8d"))]),
            sort("created_at", true),
            None,
            true,
            "4",
        ),
        View {
            layout: "timeline".into(),
            ..builtin(
                VIEW_HORIZON,
                "horizon",
                "Horizon",
                "clock",
                ViewKind::Goal,
                all(vec![]),
                vec![],
                None,
                false,
                "5",
            )
        },
        View {
            layout: "calendar".into(),
            ..builtin(
                VIEW_CALENDAR,
                "calendar",
                "Calendar",
                "calendar",
                ViewKind::Task,
                Node::Group {
                    all: false,
                    children: vec![
                        cond("due", CondOp::Before, json!("+36500d")),
                        cond("deadline", CondOp::Before, json!("+36500d")),
                    ],
                },
                sort("due", false),
                None,
                false,
                "6",
            )
        },
    ]
}

fn builtin_row(id: Uuid) -> Option<Row> {
    builtins().into_iter().find(|v| v.id == id).map(Row::View)
}

/// The built-in a view op on `op`'s target starts from when nothing is stored yet.
pub(crate) fn builtin_for(op: &Op) -> Option<Row> {
    match op {
        Op::ViewUpdate { id, .. } | Op::ViewReset { id } | Op::ViewDelete { id } => {
            builtin_row(*id)
        }
        _ => None,
    }
}

/// The stored view `id`, else the built-in of that id.
pub fn view_or_builtin(id: Uuid, rows: &[Row]) -> Option<View> {
    rows.iter()
        .find_map(|r| match r {
            Row::View(v) if v.id == id => Some(v.clone()),
            _ => None,
        })
        .or_else(|| builtins().into_iter().find(|v| v.id == id))
}

// ---- Ops ----

/// Applies `view.create`, `view.update`, `view.reset` or `view.delete`.
pub(crate) fn apply(existing: Option<Row>, cmd: &Command) -> Result<Row, ErrorCode> {
    match &cmd.op {
        Op::ViewCreate {
            id,
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
            order,
        } => {
            let name = name.trim();
            if existing.is_some() || builtin_row(*id).is_some() {
                return Err(ErrorCode::Exists);
            }
            if name.is_empty() {
                return Err(ErrorCode::TitleRequired);
            }
            let v = View {
                id: *id,
                name: name.into(),
                icon: icon.clone().unwrap_or_default(),
                kind: *kind,
                scope: *scope,
                filter: filter.clone().unwrap_or_else(match_all),
                sorts: sorts.clone().unwrap_or_default(),
                group_by: group_by.clone(),
                layout: layout.clone().unwrap_or_else(default_layout),
                fields: fields.clone().unwrap_or_default(),
                show_completed: show_completed.unwrap_or(false),
                pinned: pinned.unwrap_or(false),
                hidden: false,
                order: order.clone().unwrap_or_default(),
                trashed_at: None,
                builtin: None,
            };
            check(&v)?;
            Ok(Row::View(v))
        }
        Op::ViewUpdate {
            id,
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
        } => {
            let Some(Row::View(mut v)) = existing.or_else(|| builtin_row(*id)) else {
                return Err(ErrorCode::NotFound);
            };
            if let Some(name) = name {
                let name = name.trim();
                if name.is_empty() {
                    return Err(ErrorCode::TitleRequired);
                }
                v.name = name.into();
            }
            if let Some(x) = icon {
                v.icon = x.clone();
            }
            if let Some(x) = kind {
                v.kind = *x;
            }
            if let Some(x) = scope {
                v.scope = *x;
            }
            if let Some(x) = filter {
                v.filter = x.clone();
            }
            if let Some(x) = sorts {
                v.sorts = x.clone();
            }
            if let Some(x) = group_by {
                v.group_by = x.clone();
            }
            if let Some(x) = layout {
                v.layout = x.clone();
            }
            if let Some(x) = fields {
                v.fields = x.clone();
            }
            if let Some(x) = show_completed {
                v.show_completed = *x;
            }
            if let Some(x) = pinned {
                v.pinned = *x;
            }
            if let Some(x) = hidden {
                v.hidden = *x;
            }
            if let Some(x) = order {
                v.order = x.clone();
            }
            check(&v)?;
            Ok(Row::View(v))
        }
        Op::ViewReset { id } => match existing.or_else(|| builtin_row(*id)) {
            Some(Row::View(v)) => match v.builtin.as_deref() {
                Some(key) => builtins()
                    .into_iter()
                    .find(|b| b.builtin.as_deref() == Some(key))
                    .map(Row::View)
                    .ok_or(ErrorCode::BadField),
                None => Err(ErrorCode::BadField),
            },
            _ => Err(ErrorCode::NotFound),
        },
        Op::ViewDelete { id } => match existing.or_else(|| builtin_row(*id)) {
            Some(Row::View(v)) if v.builtin.is_some() => Err(ErrorCode::BadField),
            Some(Row::View(v)) => Ok(Row::View(View {
                trashed_at: Some(v.trashed_at.unwrap_or(cmd.hlc.wall)),
                ..v
            })),
            _ => Err(ErrorCode::NotFound),
        },
        _ => Err(ErrorCode::BadField),
    }
}

// ---- Evaluation ----

type ById<'a> = BTreeMap<Uuid, &'a Row>;

const FIELDS: [&str; 13] = [
    "title",
    "name",
    "priority",
    "due",
    "deadline",
    "label",
    "completed",
    "created_at",
    "touched_at",
    "kind",
    "parent",
    "recurring",
    "due_time",
];

fn kind_name(row: &Row) -> &'static str {
    match row {
        Row::Area(_) => "area",
        Row::Project(_) => "project",
        Row::Section(_) => "section",
        Row::Task(_) => "task",
        Row::Label(_) => "label",
        Row::Settings(_) => "settings",
        Row::Completion(_) => "completion",
        Row::Goal(_) => "goal",
        Row::Milestone(_) => "milestone",
        Row::Page(_) => "page",
        Row::Purged(_) => "purged",
        Row::View(_) => "view",
        Row::Routine(_) => "routine",
        Row::Checkin(_) => "checkin",
        Row::Roadmap(_) => "roadmap",
        Row::Phase(_) => "phase",
        Row::Placement(_) => "placement",
        Row::GraphNode(_) => "roadmap_node",
        Row::GraphEdge(_) => "roadmap_edge",
        Row::Award(_) => "award",
    }
}

fn opt<T: Serialize>(v: &Option<T>) -> Value {
    v.as_ref().map_or(Value::Null, |v| json!(v))
}

/// A field of `row` itself. An unknown name is `None`; a known one the row lacks is null.
fn own(row: &Row, field: &str) -> Option<Value> {
    if !FIELDS.contains(&field) {
        return None;
    }
    Some(match (field, row) {
        ("kind", _) => json!(kind_name(row)),
        ("parent", _) => row
            .parent_id()
            .map_or(Value::Null, |p| json!(p.to_string())),
        ("title" | "name", Row::Task(t)) => json!(t.title),
        ("title" | "name", Row::Project(p)) => json!(p.name),
        ("title" | "name", Row::Goal(g)) => json!(g.name),
        ("title" | "name", Row::Area(a)) => json!(a.name),
        ("title" | "name", Row::Page(p)) => json!(p.title),
        ("title" | "name", Row::Roadmap(r)) => json!(r.name),
        ("title" | "name", Row::Phase(p)) => json!(p.name),
        ("priority", Row::Task(t)) => json!(t.priority),
        ("due", Row::Task(t)) => t.due.as_ref().map_or(Value::Null, |d| json!(d.date)),
        ("due", Row::Goal(g)) => opt(&g.target_date),
        ("deadline", Row::Task(t)) => opt(&t.deadline),
        ("due_time", Row::Task(t)) => t
            .due
            .as_ref()
            .and_then(|d| d.time.as_ref())
            .map_or(Value::Null, |t| json!(t)),
        ("recurring", Row::Task(t)) => json!(t.recurrence.is_some()),
        ("label", Row::Task(t)) => json!(
            t.label_ids
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
        ),
        ("completed", Row::Task(t)) => json!(t.completed_at.is_some()),
        ("completed", Row::Goal(g)) => json!(g.completed_at.is_some()),
        ("created_at", Row::Task(t)) => opt(&t.created_at),
        ("touched_at", Row::Project(p)) => opt(&p.touched_at),
        ("touched_at", Row::Goal(g)) => opt(&g.touched_at),
        _ => Value::Null,
    })
}

/// The Project, Area or Goal `row` sits under (`kind` is "project", "area" or "goal").
fn hop<'a>(row: &Row, kind: &str, rows: &[Row], by_id: &ById<'a>) -> Option<&'a Row> {
    if kind == "goal" {
        let goal = match row {
            Row::Task(t) => Some(effective_goal(t, rows)),
            Row::Project(p) => Some(p.goal_id),
            _ => None,
        };
        if let Some(goal) = goal {
            return by_id
                .get(&goal?)
                .copied()
                .filter(|r| matches!(r, Row::Goal(_)));
        }
    }
    let mut next = row.parent_id();
    for _ in 0..MAX_WALK {
        let r = *by_id.get(&next?)?;
        if kind_name(r) == kind {
            return Some(r);
        }
        next = r.parent_id();
    }
    None
}

/// The value at `path`: an own field, or one hop out as `project.<f>`, `area.<f>`, `goal.<f>`.
fn value_at(row: &Row, path: &str, rows: &[Row], by_id: &ById) -> Option<Value> {
    match path {
        // A Task whose parent is another Task.
        "subtask" => {
            return Some(json!(match row {
                Row::Task(t) => matches!(by_id.get(&t.parent_id), Some(Row::Task(_))),
                _ => false,
            }));
        }
        // Title and description, for `search:`.
        "search" => {
            return Some(match row {
                Row::Task(t) => json!(format!("{}\n{}", t.title, t.description)),
                _ => own(row, "title")?,
            });
        }
        _ => {}
    }
    match path.split_once('.') {
        None => own(row, path),
        Some((kind @ ("project" | "area" | "goal"), field)) => match hop(row, kind, rows, by_id) {
            Some(target) => own(target, field),
            None => FIELDS.contains(&field).then_some(Value::Null),
        },
        Some(_) => None,
    }
}

/// A date as days since the epoch: `YYYY-MM-DD`, `today`, `tomorrow`, `yesterday`,
/// or an offset such as `+3d`, `-2d`, `+1w`.
pub fn resolve_date(s: &str, today: i64) -> Option<i64> {
    match s {
        "today" => return Some(today),
        "tomorrow" => return Some(today + 1),
        "yesterday" => return Some(today - 1),
        _ => {}
    }
    if let Some(d) = parse_ymd(s) {
        return Some(d);
    }
    let (sign, rest) = match s.as_bytes().first()? {
        b'+' => (1, &s[1..]),
        b'-' => (-1, &s[1..]),
        _ => return None,
    };
    let unit = rest.chars().last()?;
    let n: i64 = rest[..rest.len() - unit.len_utf8()].parse().ok()?;
    let mult = match unit {
        'd' => 1,
        'w' => 7,
        _ => return None,
    };
    Some(today + sign * n * mult)
}

/// `search:` matching: every word of `query` occurs in `text`, in any order; `*` in a word
/// stands for any run of characters.
fn search_match(text: &str, query: &str) -> bool {
    let text = text.to_lowercase();
    query.to_lowercase().split_whitespace().all(|word| {
        let mut at = 0;
        word.split('*').all(|part| match text[at..].find(part) {
            Some(i) => {
                at += i + part.len();
                true
            }
            None => false,
        })
    })
}

/// A field value as a day: a date string, or a Unix ms timestamp by its UTC date.
fn field_days(v: &Value) -> Option<i64> {
    match v {
        Value::String(s) => parse_ymd(s),
        Value::Number(n) => n.as_u64().map(|ms| (ms / DAY_MS) as i64),
        _ => None,
    }
}

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

fn loose_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x.to_lowercase() == y.to_lowercase(),
        (Value::Number(x), Value::String(y)) | (Value::String(y), Value::Number(x)) => {
            x.to_string() == *y
        }
        _ => a == b,
    }
}

fn same(v: &Value, value: &Value, today: i64) -> bool {
    if let Value::Array(items) = v {
        return items.iter().any(|x| loose_eq(x, value));
    }
    let bound = value.as_str().and_then(|s| resolve_date(s, today));
    if let (Some(a), Some(b)) = (field_days(v), bound) {
        return a == b;
    }
    loose_eq(v, value)
}

fn eval_cond(v: &Value, op: CondOp, value: &Value, today: i64) -> bool {
    match op {
        CondOp::IsEmpty => is_empty(v),
        CondOp::Is => same(v, value, today),
        CondOp::IsNot => !same(v, value, today),
        CondOp::Contains => match v {
            Value::String(s) => value
                .as_str()
                .is_some_and(|n| s.to_lowercase().contains(&n.to_lowercase())),
            Value::Array(items) => items.iter().any(|x| loose_eq(x, value)),
            _ => false,
        },
        CondOp::Before | CondOp::After => {
            let (Some(f), Some(b)) = (
                field_days(v),
                value.as_str().and_then(|s| resolve_date(s, today)),
            ) else {
                return false;
            };
            if op == CondOp::Before { f < b } else { f > b }
        }
        CondOp::Within => {
            let bound = |k: &str| {
                value
                    .get(k)
                    .and_then(Value::as_str)
                    .and_then(|s| resolve_date(s, today))
            };
            let (Some(f), Some(from), Some(to)) = (field_days(v), bound("from"), bound("to"))
            else {
                return false;
            };
            from <= f && f <= to
        }
    }
}

struct Ctx<'a> {
    rows: &'a [Row],
    by_id: ById<'a>,
    today: i64,
}

fn eval(node: &Node, row: &Row, cx: &Ctx) -> bool {
    match node {
        Node::Group { all, children } => {
            if children.is_empty() {
                true
            } else if *all {
                children.iter().all(|c| eval(c, row, cx))
            } else {
                children.iter().any(|c| eval(c, row, cx))
            }
        }
        Node::Cond { path, op, value }
            if path.starts_with("phase.") || path.starts_with("roadmap.") =>
        {
            let today = crate::parse::fmt_date(cx.today);
            let values = roadmap::path_values(row, path, cx.rows, &cx.by_id, &today);
            let hit = |v: &Value| eval_cond(v, CondOp::Is, value, cx.today);
            match op {
                CondOp::IsEmpty => values.iter().all(is_empty),
                CondOp::IsNot => !values.iter().any(hit),
                CondOp::Is => values.iter().any(hit),
                _ => values.iter().any(|v| eval_cond(v, *op, value, cx.today)),
            }
        }
        Node::Cond { path, op, value } if path == "search" => {
            let hit = match (value_at(row, path, cx.rows, &cx.by_id), value.as_str()) {
                (Some(Value::String(text)), Some(q)) => search_match(&text, q),
                _ => false,
            };
            match op {
                CondOp::IsNot => !hit,
                _ => hit,
            }
        }
        Node::Cond { path, op, value } => value_at(row, path, cx.rows, &cx.by_id)
            .is_some_and(|v| eval_cond(&v, *op, value, cx.today)),
    }
}

fn row_order(row: &Row) -> &str {
    match row {
        Row::Task(t) => &t.order,
        Row::Project(p) => &p.order,
        Row::Goal(g) => &g.order,
        Row::Page(p) => &p.order,
        _ => "",
    }
}

fn cmp_value(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .partial_cmp(&y.as_f64())
            .unwrap_or(Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        _ => Ordering::Equal,
    }
}

/// Empty values sort last, whichever way the sort runs.
fn cmp_sorted(a: &Option<Value>, b: &Option<Value>, desc: bool) -> Ordering {
    let blank = |v: &Option<Value>| v.as_ref().is_none_or(|v| v.is_null());
    match (blank(a), blank(b)) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            let ord = cmp_value(a.as_ref().unwrap(), b.as_ref().unwrap());
            if desc { ord.reverse() } else { ord }
        }
    }
}

fn group_key(view: &View, row: &Row, cx: &Ctx) -> String {
    match view.group_by.as_deref() {
        Some("due") => match own(row, "due") {
            Some(Value::String(s)) => {
                if parse_ymd(&s).is_some_and(|d| d < cx.today) {
                    "Overdue".into()
                } else {
                    s
                }
            }
            _ => "No date".into(),
        },
        Some("priority") => match own(row, "priority") {
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        },
        Some(kind @ ("project" | "area" | "goal")) => hop(row, kind, cx.rows, &cx.by_id)
            .map(|r| r.id().to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn is_candidate(kind: ViewKind, row: &Row) -> bool {
    matches!(
        (kind, row),
        (ViewKind::Task, Row::Task(_))
            | (ViewKind::Project, Row::Project(_))
            | (ViewKind::Goal, Row::Goal(_))
            | (ViewKind::Page, Row::Page(_))
            | (ViewKind::Plan, Row::Project(_) | Row::Goal(_))
            | (ViewKind::Roadmap, Row::Roadmap(_))
            | (ViewKind::Phase, Row::Phase(_))
    )
}

fn is_completed(row: &Row) -> bool {
    match row {
        Row::Task(t) => t.completed_at.is_some(),
        Row::Goal(g) => g.completed_at.is_some(),
        Row::Phase(p) => p.completed_at.is_some(),
        _ => false,
    }
}

/// A filter that names `completed` shows completed rows whatever `show_completed` says.
fn mentions_completed(n: &Node) -> bool {
    match n {
        Node::Group { children, .. } => children.iter().any(mentions_completed),
        Node::Cond { path, .. } => path == "completed",
    }
}

fn in_scope(row: &Row, scope: Uuid, by_id: &ById) -> bool {
    let mut cur = Some(row);
    for _ in 0..MAX_WALK {
        let Some(r) = cur else { return false };
        if r.id() == scope {
            return true;
        }
        cur = r.parent_id().and_then(|p| by_id.get(&p).copied());
    }
    false
}

/// Runs `view` over `rows`; `today` is the caller's local `YYYY-MM-DD`.
pub fn evaluate(view: &View, rows: &[Row], today: &str) -> Vec<Group> {
    let by_id: ById = rows.iter().map(|r| (r.id(), r)).collect();
    let cx = Ctx {
        rows,
        by_id,
        today: parse_ymd(today).unwrap_or(0),
    };
    let lookup = |id: Uuid| cx.by_id.get(&id).map(|r| (*r).clone());
    let show_completed = view.show_completed || mentions_completed(&view.filter);
    let mut found: Vec<&Row> = rows
        .iter()
        .filter(|r| is_candidate(view.kind, r))
        .filter(|r| show_completed || !is_completed(r))
        .filter(|r| !trash::hidden(r, &lookup))
        .filter(|r| view.scope.is_none_or(|scope| in_scope(r, scope, &cx.by_id)))
        .filter(|r| eval(&view.filter, r, &cx))
        .collect();
    let keys: BTreeMap<Uuid, Vec<Option<Value>>> = found
        .iter()
        .map(|r| {
            let k = view
                .sorts
                .iter()
                .map(|s| value_at(r, &s.field, rows, &cx.by_id))
                .collect();
            (r.id(), k)
        })
        .collect();
    found.sort_by(|a, b| {
        for (i, s) in view.sorts.iter().enumerate() {
            let ord = cmp_sorted(&keys[&a.id()][i], &keys[&b.id()][i], s.desc);
            if ord != Ordering::Equal {
                return ord;
            }
        }
        row_order(a)
            .cmp(row_order(b))
            .then_with(|| a.id().cmp(&b.id()))
    });
    let mut groups: Vec<Group> = vec![];
    for r in found {
        let key = group_key(view, r, &cx);
        match groups.iter_mut().find(|g| g.key == key) {
            Some(g) => g.ids.push(r.id()),
            None => groups.push(Group {
                key,
                ids: vec![r.id()],
            }),
        }
    }
    if view.group_by.as_deref() == Some("due") {
        let rank = |k: &str| match k {
            "Overdue" => 0,
            "No date" => 2,
            _ => 1,
        };
        groups.sort_by(|a, b| {
            rank(&a.key)
                .cmp(&rank(&b.key))
                .then_with(|| a.key.cmp(&b.key))
        });
    } else {
        groups.sort_by_key(|g| g.key.is_empty());
    }
    if view.group_by.is_none() && groups.is_empty() {
        groups.push(Group {
            key: String::new(),
            ids: vec![],
        });
    }
    groups
}

/// The day `today` names, for tests and callers that hold a day number.
pub fn today_string(days: i64) -> String {
    fmt_date(days)
}

/// The dates in `from..=to` on which Routine `routine_id` is due (unpaused, inside its length), for the Calendar.
pub fn routine_days(rows: &[Row], routine_id: Uuid, from: &str, to: &str) -> Vec<String> {
    let Some(Row::Routine(r)) = rows.iter().find(|r| r.id() == routine_id) else {
        return vec![];
    };
    let (Some(a), Some(b)) = (parse_ymd(from), parse_ymd(to)) else {
        return vec![];
    };
    (a..=b.min(a + 366))
        .filter(|d| crate::routine::due_on(r, *d))
        .map(fmt_date)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hlc, apply as core_apply, apply_lww};

    const TODAY: &str = "2026-11-19";

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 + n)
    }

    fn n_of(id: Uuid) -> u128 {
        id.as_u128() - uid(0).as_u128()
    }

    fn today() -> i64 {
        parse_ymd(TODAY).unwrap()
    }

    fn ms(days: i64) -> u64 {
        days as u64 * DAY_MS
    }

    fn op(kind: &str, args: Value, wall: u64) -> Command {
        serde_json::from_value(json!({
            "uuid": uid(900 + wall as u128 % 50),
            "type": kind,
            "args": args,
            "hlc": Hlc { wall, counter: 0, device: uid(999) },
        }))
        .unwrap()
    }

    fn area(n: u128, name: &str) -> Row {
        core_apply(
            None,
            &op("area.create", json!({"id": uid(n), "name": name}), 1),
        )
        .unwrap()
    }

    fn project(n: u128, parent: Uuid, name: &str, touched: Option<u64>) -> Row {
        let mut row = core_apply(
            None,
            &op(
                "project.create",
                json!({"id": uid(n), "parent_id": parent, "name": name}),
                1,
            ),
        )
        .unwrap();
        if let Row::Project(p) = &mut row {
            p.touched_at = touched;
        }
        row
    }

    /// A task created `age` days before today, then edited with `extra`.
    fn task(n: u128, parent: Uuid, extra: Value, age: i64) -> Row {
        let create = op(
            "task.create",
            json!({"id": uid(n), "title": format!("T{n}"), "parent_id": parent}),
            ms(today() - age),
        );
        let mut row = core_apply(None, &create).unwrap();
        if extra.as_object().is_some_and(|o| !o.is_empty()) {
            let mut args = extra;
            args["id"] = json!(uid(n));
            row = core_apply(Some(row), &op("task.update", args, ms(today()))).unwrap();
        }
        row
    }

    fn due(offset: i64) -> Value {
        json!({"due": {"date": fmt_date(today() + offset)}})
    }

    fn custom(filter: Value) -> View {
        serde_json::from_value(json!({
            "id": uid(500),
            "name": "V",
            "entity": "task",
            "filter": filter,
        }))
        .unwrap()
    }

    fn ids(view: &View, rows: &[Row]) -> Vec<u128> {
        evaluate(view, rows, TODAY)
            .iter()
            .flat_map(|g| g.ids.iter().map(|i| n_of(*i)))
            .collect()
    }

    fn c(path: &str, op: &str, value: Value) -> Value {
        json!({"type": "cond", "path": path, "op": op, "value": value})
    }

    fn all(children: Vec<Value>) -> Value {
        json!({"type": "group", "all": true, "children": children})
    }

    fn any(children: Vec<Value>) -> Value {
        json!({"type": "group", "all": false, "children": children})
    }

    fn run(filter: Value, rows: &[Row]) -> Vec<u128> {
        let mut got = ids(&custom(filter), rows);
        got.sort();
        got
    }

    fn base_tasks() -> Vec<Row> {
        vec![
            task(
                1,
                INBOX,
                json!({"title": "Buy milk", "priority": 1, "due": {"date": TODAY}}),
                0,
            ),
            task(
                2,
                INBOX,
                json!({"title": "Call Bob", "priority": 2, "due": {"date": "2026-11-25"}}),
                0,
            ),
            task(3, INBOX, json!({"title": "Write"}), 0),
            task(
                4,
                INBOX,
                json!({"title": "Old", "due": {"date": "2026-11-10"}}),
                0,
            ),
        ]
    }

    #[test]
    fn every_condition_operator() {
        let rows = base_tasks();
        assert_eq!(run(c("priority", "is", json!(1)), &rows), [1]);
        assert_eq!(run(c("priority", "is_not", json!(1)), &rows), [2, 3, 4]);
        assert_eq!(run(c("title", "contains", json!("MILK")), &rows), [1]);
        assert_eq!(run(c("due", "is_empty", Value::Null), &rows), [3]);
        assert_eq!(run(c("due", "before", json!("today")), &rows), [4]);
        assert_eq!(run(c("due", "after", json!("today")), &rows), [2]);
        assert_eq!(run(c("due", "is", json!("today")), &rows), [1]);
        assert_eq!(run(c("due", "is", json!("2026-11-25")), &rows), [2]);
        assert_eq!(
            run(
                c("due", "within", json!({"from": "today", "to": "+6d"})),
                &rows
            ),
            [1, 2]
        );
        assert_eq!(
            run(
                c("due", "within", json!({"from": "+1d", "to": "+5d"})),
                &rows
            ),
            Vec::<u128>::new()
        );
        // An unknown path never matches, whatever the operator.
        assert!(run(c("nope", "is_not", json!(1)), &rows).is_empty());
        assert!(run(c("project.nope", "is_empty", Value::Null), &rows).is_empty());
    }

    #[test]
    fn relative_dates_resolve_from_today() {
        let t = today();
        assert_eq!(resolve_date("today", t), Some(t));
        assert_eq!(resolve_date("tomorrow", t), Some(t + 1));
        assert_eq!(resolve_date("yesterday", t), Some(t - 1));
        assert_eq!(resolve_date("+3d", t), Some(t + 3));
        assert_eq!(resolve_date("-2d", t), Some(t - 2));
        assert_eq!(resolve_date("+1w", t), Some(t + 7));
        assert_eq!(resolve_date("-2w", t), Some(t - 14));
        assert_eq!(resolve_date("2026-12-01", t), parse_ymd("2026-12-01"));
        assert_eq!(resolve_date("soon", t), None);
        assert_eq!(resolve_date("+3x", t), None);
        assert_eq!(today_string(t), TODAY);
    }

    #[test]
    fn created_at_compares_by_utc_date() {
        let rows = vec![task(1, INBOX, json!({}), 3), task(2, INBOX, json!({}), 10)];
        assert_eq!(run(c("created_at", "after", json!("-8d")), &rows), [1]);
        assert_eq!(run(c("created_at", "before", json!("-8d")), &rows), [2]);
        assert_eq!(run(c("created_at", "is", json!("-3d")), &rows), [1]);
    }

    #[test]
    fn nested_groups_combine_all_and_any() {
        let rows = base_tasks();
        let f = all(vec![
            any(vec![
                c("priority", "is", json!(1)),
                c("title", "contains", json!("bob")),
            ]),
            all(vec![
                c("due", "after", json!("yesterday")),
                any(vec![
                    c("priority", "is", json!(2)),
                    c("priority", "is", json!(1)),
                ]),
            ]),
        ]);
        assert_eq!(run(f, &rows), [1, 2]);
        assert_eq!(run(any(vec![]), &rows), [1, 2, 3, 4]);
    }

    #[test]
    fn filters_nest_three_deep_and_not_four() {
        let three = all(vec![all(vec![all(vec![c("priority", "is", json!(1))])])]);
        let four = all(vec![three.clone()]);
        let create = |filter: Value| {
            op(
                "view.create",
                json!({"id": uid(7), "name": "V", "kind": "task", "filter": filter}),
                5,
            )
        };
        assert!(core_apply(None, &create(three.clone())).is_ok());
        assert_eq!(
            core_apply(None, &create(four.clone())),
            Err(ErrorCode::BadField)
        );
        let Row::View(v) = core_apply(None, &create(three)).unwrap() else {
            panic!("a view")
        };
        let update = op("view.update", json!({"id": uid(7), "filter": four}), 6);
        assert_eq!(
            core_apply(Some(Row::View(v)), &update),
            Err(ErrorCode::BadField)
        );
    }

    #[test]
    fn one_hop_paths_reach_project_area_and_goal() {
        let goal = core_apply(
            None,
            &op(
                "goal.create",
                json!({"id": uid(3), "parent_id": uid(1), "name": "Fitness"}),
                1,
            ),
        )
        .unwrap();
        let mut rows = vec![area(1, "Home"), goal, project(2, uid(1), "Alpha", None)];
        if let Row::Project(p) = &mut rows[2] {
            p.goal_id = Some(uid(3));
        }
        rows.push(task(10, uid(2), json!({}), 0));
        rows.push(task(11, INBOX, json!({}), 0));
        rows.push(task(12, uid(10), json!({}), 0));
        assert_eq!(
            run(c("project.name", "is", json!("alpha")), &rows),
            [10, 12]
        );
        assert_eq!(run(c("goal.name", "is", json!("Fitness")), &rows), [10, 12]);
        assert_eq!(run(c("area.name", "is", json!("Home")), &rows), [10, 12]);
        assert_eq!(run(c("project.name", "is_empty", Value::Null), &rows), [11]);
        assert_eq!(
            run(c("area.name", "is", json!("Inbox")), &rows),
            Vec::<u128>::new()
        );
        // A project reaches its area and goal.
        let plan = |filter| {
            let mut v = custom(filter);
            v.kind = ViewKind::Project;
            v
        };
        assert_eq!(
            ids(&plan(c("goal.name", "is", json!("Fitness"))), &rows),
            [2]
        );
        assert_eq!(ids(&plan(c("area.name", "is", json!("Home"))), &rows), [2]);
    }

    #[test]
    fn inbox_lists_open_tasks_directly_in_the_inbox() {
        let rows = vec![
            area(1, "Home"),
            project(2, uid(1), "P", None),
            task(3, INBOX, json!({}), 0),
            task(4, uid(2), json!({}), 0),
        ];
        let v = view_or_builtin(VIEW_INBOX, &rows).unwrap();
        assert_eq!(ids(&v, &rows), [3]);
    }

    #[test]
    fn upcoming_runs_through_the_next_week_and_groups_by_day() {
        let rows = vec![
            task(1, INBOX, due(-2), 0),
            task(2, INBOX, due(0), 0),
            task(3, INBOX, due(7), 0),
            task(4, INBOX, due(8), 0),
            task(5, INBOX, due(9), 0),
            task(6, INBOX, json!({}), 0),
            task(7, INBOX, due(3), 0),
        ];
        let v = view_or_builtin(VIEW_UPCOMING, &rows).unwrap();
        let groups = evaluate(&v, &rows, TODAY);
        let shape: Vec<(String, Vec<u128>)> = groups
            .iter()
            .map(|g| (g.key.clone(), g.ids.iter().map(|i| n_of(*i)).collect()))
            .collect();
        assert_eq!(
            shape,
            vec![
                ("Overdue".to_string(), vec![1]),
                (fmt_date(today()), vec![2]),
                (fmt_date(today() + 3), vec![7]),
                (fmt_date(today() + 7), vec![3]),
            ]
        );
    }

    #[test]
    fn group_by_due_puts_undated_rows_last() {
        let rows = vec![task(1, INBOX, json!({}), 0), task(2, INBOX, due(-1), 0)];
        let mut v = custom(all(vec![]));
        v.group_by = Some("due".into());
        let keys: Vec<String> = evaluate(&v, &rows, TODAY)
            .into_iter()
            .map(|g| g.key)
            .collect();
        assert_eq!(keys, ["Overdue", "No date"]);
    }

    #[test]
    fn slipping_lists_plans_untouched_for_two_weeks() {
        let rows = vec![
            area(1, "Home"),
            project(2, uid(1), "Stale", Some(ms(today() - 20))),
            project(3, uid(1), "Fresh", Some(ms(today() - 2))),
            project(4, uid(1), "Never", None),
        ];
        let v = view_or_builtin(VIEW_SLIPPING, &rows).unwrap();
        // Oldest first, and a project with no touch counts as slipping.
        assert_eq!(ids(&v, &rows), [2, 4]);
    }

    #[test]
    fn recent_lists_the_last_week_newest_first_with_done_tasks() {
        let mut done = task(3, INBOX, json!({}), 1);
        if let Row::Task(t) = &mut done {
            t.completed_at = Some(1);
        }
        let rows = vec![
            task(1, INBOX, json!({}), 3),
            task(2, INBOX, json!({}), 10),
            done,
        ];
        let v = view_or_builtin(VIEW_RECENT, &rows).unwrap();
        assert_eq!(v.name, "Recently captured");
        assert_eq!(ids(&v, &rows), [3, 1]);
    }

    #[test]
    fn candidates_skip_hidden_completed_and_out_of_scope_rows() {
        let mut gone = task(4, uid(2), json!({}), 0);
        gone.set_trashed(Some(1));
        let mut done = task(5, uid(2), json!({}), 0);
        if let Row::Task(t) = &mut done {
            t.completed_at = Some(1);
        }
        let rows = vec![
            area(1, "Home"),
            project(2, uid(1), "P", None),
            task(3, uid(2), json!({}), 0),
            gone,
            done,
            task(6, INBOX, json!({}), 0),
        ];
        let mut v = custom(all(vec![]));
        assert_eq!(ids(&v, &rows), [3, 6]);
        v.scope = Some(uid(2));
        assert_eq!(ids(&v, &rows), [3]);
        v.show_completed = true;
        assert_eq!(ids(&v, &rows), [3, 5]);
    }

    #[test]
    fn a_filter_on_completed_shows_completed_rows() {
        let mut done = task(2, INBOX, json!({}), 0);
        if let Row::Task(t) = &mut done {
            t.completed_at = Some(1);
        }
        let rows = vec![task(1, INBOX, json!({}), 0), done];
        assert_eq!(
            run(all(vec![c("completed", "is", json!(true))]), &rows),
            [2]
        );
        assert_eq!(
            run(any(vec![c("completed", "is", json!(false))]), &rows),
            [1]
        );
    }

    #[test]
    fn sorts_run_in_order_with_empty_values_last() {
        let rows = base_tasks();
        let mut v = custom(all(vec![]));
        v.sorts = vec![Sort {
            field: "priority".into(),
            desc: false,
        }];
        assert_eq!(ids(&v, &rows), [1, 2, 3, 4]);
        v.sorts = vec![
            Sort {
                field: "due".into(),
                desc: true,
            },
            Sort {
                field: "title".into(),
                desc: false,
            },
        ];
        assert_eq!(ids(&v, &rows), [2, 1, 4, 3]);
    }

    #[test]
    fn calendar_builtin_and_routine_days() {
        let c = builtins()
            .into_iter()
            .find(|v| v.id == VIEW_CALENDAR)
            .unwrap();
        assert_eq!((c.kind, c.layout.as_str()), (ViewKind::Task, "calendar"));
        let r = crate::routine::Routine {
            id: Uuid::from_u128(1),
            parent_id: Uuid::from_u128(2),
            name: "R".into(),
            icon: String::new(),
            schedule: crate::routine::Schedule::Every { n: 2 },
            start: "2026-03-02".into(),
            length_days: Some(5),
            time: None,
            goal_id: None,
            reminder: None,
            pauses: vec![],
            order: String::new(),
            created_at: None,
            trashed_at: None,
            archived_at: None,
        };
        let rows = vec![Row::Routine(r)];
        assert_eq!(
            routine_days(&rows, Uuid::from_u128(1), "2026-03-01", "2026-03-31"),
            ["2026-03-02", "2026-03-04", "2026-03-06"]
        );
        assert!(routine_days(&rows, Uuid::from_u128(9), "2026-03-01", "2026-03-31").is_empty());
    }

    #[test]
    fn builtins_are_pinned_with_stable_distinct_ids() {
        let b = builtins();
        assert_eq!(b.len(), 6);
        assert!(b.iter().all(|v| v.pinned && v.builtin.is_some()));
        let mut seen: Vec<Uuid> = b.iter().map(|v| v.id).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 6);
        let h = b.iter().find(|v| v.id == VIEW_HORIZON).unwrap();
        assert_eq!((h.kind, h.layout.as_str()), (ViewKind::Goal, "timeline"));
        assert_eq!(
            VIEW_HORIZON.to_string(),
            "00000000-0000-7000-8000-000000000105"
        );
        assert_eq!(builtins(), b);
    }

    #[test]
    fn a_builtin_is_customised_reset_and_never_deleted() {
        let hide = op(
            "view.update",
            json!({"id": VIEW_INBOX, "hidden": true, "name": "In"}),
            10,
        );
        let Row::View(v) = core_apply(None, &hide).unwrap() else {
            panic!("a view")
        };
        assert!(v.hidden && v.name == "In" && v.builtin.as_deref() == Some("inbox"));
        let reset = op("view.reset", json!({"id": VIEW_INBOX}), 11);
        let back = core_apply(Some(Row::View(v.clone())), &reset).unwrap();
        assert_eq!(back, builtin_row(VIEW_INBOX).unwrap());
        let del = op("view.delete", json!({"id": VIEW_INBOX}), 12);
        assert_eq!(core_apply(None, &del), Err(ErrorCode::BadField));
        assert_eq!(
            core_apply(Some(Row::View(v)), &del),
            Err(ErrorCode::BadField)
        );
        let create = op(
            "view.create",
            json!({"id": VIEW_INBOX, "name": "X", "kind": "task"}),
            13,
        );
        assert_eq!(core_apply(None, &create), Err(ErrorCode::Exists));
    }

    #[test]
    fn deleting_a_custom_view_trashes_it() {
        let create = op(
            "view.create",
            json!({"id": uid(7), "name": " Mine ", "kind": "goal"}),
            5,
        );
        let row = core_apply(None, &create).unwrap();
        let Row::View(v) = &row else { panic!("a view") };
        assert_eq!(
            (v.name.as_str(), v.kind, v.pinned),
            ("Mine", ViewKind::Goal, false)
        );
        let del = op("view.delete", json!({"id": uid(7)}), 9);
        let gone = core_apply(Some(row), &del).unwrap();
        assert_eq!(gone.trashed_at(), Some(9));
        let reset = op("view.reset", json!({"id": uid(7)}), 10);
        assert_eq!(core_apply(Some(gone), &reset), Err(ErrorCode::BadField));
    }

    #[test]
    fn the_filter_is_one_field_for_last_writer_wins() {
        let create = op(
            "view.create",
            json!({"id": uid(7), "name": "V", "kind": "task"}),
            5,
        );
        let mut clocks = crate::Clocks::new();
        let row = apply_lww(None, &mut clocks, &create).unwrap();
        let newer = op(
            "view.update",
            json!({"id": uid(7), "filter": all(vec![c("priority", "is", json!(1))])}),
            50,
        );
        let row = apply_lww(Some(row), &mut clocks, &newer).unwrap();
        let older = op(
            "view.update",
            json!({"id": uid(7), "name": "Renamed", "filter": all(vec![c("priority", "is", json!(2))])}),
            20,
        );
        let Row::View(v) = apply_lww(Some(row), &mut clocks, &older).unwrap() else {
            panic!("a view")
        };
        // The older filter loses whole; the name, on its own clock, still lands.
        assert_eq!(v.name, "Renamed");
        let Node::Group { children, .. } = &v.filter else {
            panic!("a group")
        };
        assert_eq!(
            children,
            &vec![serde_json::from_value(c("priority", "is", json!(1))).unwrap()]
        );
    }
}
