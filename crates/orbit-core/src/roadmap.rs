//! Roadmaps: ordered Phases that reference items without moving them, and the progress maths.
//!
//! A Placement is the membership of one item in one Roadmap. Its id is derived from the pair, so
//! two devices placing the same item write the same row and moving it is a last-writer-wins edit
//! of `phase_id`: one Phase per item per Roadmap holds by construction.
use crate::{Command, ErrorCode, Op, Row, Uuid, is_date, trash};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Roadmap {
    pub id: Uuid,
    /// An Area, Project or Goal.
    pub parent_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub order: String,
    /// Unix ms when it was archived.
    #[serde(default)]
    pub archived_at: Option<u64>,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    #[serde(default)]
    pub created_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Phase {
    pub id: Uuid,
    pub roadmap_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub order: String,
    /// `YYYY-MM-DD`; a Phase without dates is a Now/Next/Later bucket.
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub completed_at: Option<u64>,
    #[serde(default)]
    pub archived_at: Option<u64>,
    #[serde(default)]
    pub trashed_at: Option<u64>,
}

/// An item's place in a Roadmap. `phase_id` is `None` once it is taken out.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Placement {
    pub id: Uuid,
    pub roadmap_id: Uuid,
    pub item_id: Uuid,
    #[serde(default)]
    pub phase_id: Option<Uuid>,
}

/// The same Roadmap and item always give the same Placement id.
pub fn placement_id(roadmap_id: Uuid, item_id: Uuid) -> Uuid {
    let id = roadmap_id.as_u128() ^ item_id.as_u128().rotate_left(64);
    uuid::Builder::from_random_bytes(id.to_be_bytes()).into_uuid()
}

fn check_dates(start: &Option<String>, end: &Option<String>) -> Result<(), ErrorCode> {
    if start.as_deref().is_some_and(|d| !is_date(d))
        || end.as_deref().is_some_and(|d| !is_date(d))
        || matches!((start, end), (Some(s), Some(e)) if s > e)
    {
        return Err(ErrorCode::BadField);
    }
    Ok(())
}

fn name_of(name: &str) -> Result<String, ErrorCode> {
    let name = name.trim();
    if name.is_empty() {
        Err(ErrorCode::TitleRequired)
    } else {
        Ok(name.into())
    }
}

pub(crate) fn apply(existing: Option<Row>, cmd: &Command) -> Result<Row, ErrorCode> {
    match &cmd.op {
        Op::RoadmapCreate {
            id,
            parent_id,
            name,
            order,
        } => {
            if existing.is_some() {
                return Err(ErrorCode::Exists);
            }
            Ok(Row::Roadmap(Roadmap {
                id: *id,
                parent_id: *parent_id,
                name: name_of(name)?,
                order: order.clone(),
                archived_at: None,
                trashed_at: None,
                created_at: Some(cmd.hlc.wall),
            }))
        }
        Op::RoadmapUpdate { name, .. } => {
            let Some(Row::Roadmap(old)) = existing else {
                return Err(ErrorCode::NotFound);
            };
            Ok(Row::Roadmap(Roadmap {
                name: match name {
                    Some(n) => name_of(n)?,
                    None => old.name.clone(),
                },
                ..old
            }))
        }
        Op::PhaseCreate {
            id,
            roadmap_id,
            name,
            order,
            start,
            end,
        } => {
            if existing.is_some() {
                return Err(ErrorCode::Exists);
            }
            check_dates(start, end)?;
            Ok(Row::Phase(Phase {
                id: *id,
                roadmap_id: *roadmap_id,
                name: name_of(name)?,
                order: order.clone(),
                start: start.clone(),
                end: end.clone(),
                completed_at: None,
                archived_at: None,
                trashed_at: None,
            }))
        }
        Op::PhaseUpdate {
            name,
            order,
            start,
            end,
            ..
        } => {
            let Some(Row::Phase(old)) = existing else {
                return Err(ErrorCode::NotFound);
            };
            let p = Phase {
                name: match name {
                    Some(n) => name_of(n)?,
                    None => old.name.clone(),
                },
                order: order.clone().unwrap_or(old.order.clone()),
                start: start.clone().unwrap_or(old.start.clone()),
                end: end.clone().unwrap_or(old.end.clone()),
                ..old
            };
            check_dates(&p.start, &p.end)?;
            Ok(Row::Phase(p))
        }
        Op::PhaseComplete { .. } | Op::PhaseUncomplete { .. } => {
            let Some(Row::Phase(mut p)) = existing else {
                return Err(ErrorCode::NotFound);
            };
            p.completed_at = match &cmd.op {
                Op::PhaseComplete { .. } => Some(p.completed_at.unwrap_or(cmd.hlc.wall)),
                _ => None,
            };
            Ok(Row::Phase(p))
        }
        Op::PlacementSet {
            id,
            roadmap_id,
            item_id,
            phase_id,
        } => {
            if *id != placement_id(*roadmap_id, *item_id) {
                return Err(ErrorCode::BadField);
            }
            match existing {
                None => Ok(Row::Placement(Placement {
                    id: *id,
                    roadmap_id: *roadmap_id,
                    item_id: *item_id,
                    phase_id: *phase_id,
                })),
                Some(Row::Placement(p)) => Ok(Row::Placement(Placement {
                    phase_id: *phase_id,
                    ..p
                })),
                Some(_) => Err(ErrorCode::BadField),
            }
        }
        _ => Err(ErrorCode::BadField),
    }
}

/// Whether `row` can sit in a Phase.
pub fn is_placeable(row: &Row) -> bool {
    matches!(
        row,
        Row::Goal(_) | Row::Project(_) | Row::Milestone(_) | Row::Task(_)
    )
}

type ById<'a> = BTreeMap<Uuid, &'a Row>;

fn by_id(rows: &[Row]) -> ById<'_> {
    rows.iter().map(|r| (r.id(), r)).collect()
}

fn hidden(row: &Row, by_id: &ById) -> bool {
    trash::hidden(row, &|id| by_id.get(&id).map(|r| (*r).clone()))
}

fn project_of<'a>(task: &Row, by_id: &ById<'a>) -> Option<&'a Row> {
    let mut next = task.parent_id();
    for _ in 0..32 {
        let r = *by_id.get(&next?)?;
        if matches!(r, Row::Project(_)) {
            return Some(r);
        }
        next = r.parent_id();
    }
    None
}

/// How far along one item is, 0..=1: a Goal by its progress, a Project by the share of its
/// Tasks done, a Milestone or Task 0 or 1.
pub fn item_progress(rows: &[Row], item_id: Uuid) -> f64 {
    let Some(row) = rows.iter().find(|r| r.id() == item_id) else {
        return 0.0;
    };
    let done = |b: bool| if b { 1.0 } else { 0.0 };
    match row {
        Row::Goal(g) => crate::goal::progress(g, rows),
        Row::Milestone(m) => done(m.done),
        Row::Task(t) => done(t.completed_at.is_some()),
        Row::Project(p) => {
            let ids = by_id(rows);
            let (mut total, mut finished) = (0u32, 0u32);
            for r in rows {
                if let Row::Task(t) = r
                    && project_of(r, &ids).is_some_and(|q| q.id() == p.id)
                    && !hidden(r, &ids)
                {
                    total += 1;
                    finished += t.completed_at.is_some() as u32;
                }
            }
            if total == 0 {
                0.0
            } else {
                finished as f64 / total as f64
            }
        }
        _ => 0.0,
    }
}

/// The placements in `phase_id` whose item still exists and isn't trashed or archived.
fn phase_items<'a>(rows: &'a [Row], phase_id: Uuid, ids: &ById<'a>) -> Vec<&'a Row> {
    let mut items: Vec<&Row> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Placement(p) if p.phase_id == Some(phase_id) => ids.get(&p.item_id).copied(),
            _ => None,
        })
        .filter(|r| is_placeable(r) && !hidden(r, ids))
        .collect();
    items.sort_by_key(|r| r.id());
    items
}

/// Mean progress of the items in a Phase; an empty Phase is 0.
pub fn phase_progress(rows: &[Row], phase_id: Uuid) -> f64 {
    let ids = by_id(rows);
    let items = phase_items(rows, phase_id, &ids);
    if items.is_empty() {
        return 0.0;
    }
    items
        .iter()
        .map(|r| item_progress(rows, r.id()))
        .sum::<f64>()
        / items.len() as f64
}

fn live_phases(rows: &[Row], roadmap_id: Uuid) -> Vec<&Phase> {
    let mut phases: Vec<&Phase> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Phase(p) if p.roadmap_id == roadmap_id && p.trashed_at.is_none() => Some(p),
            _ => None,
        })
        .collect();
    phases.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    phases
}

/// The Phase whose dates cover `today`, else the first unfinished one.
pub fn current_phase(rows: &[Row], roadmap_id: Uuid, today: &str) -> Option<Uuid> {
    let phases = live_phases(rows, roadmap_id);
    let covers = |p: &&&Phase| {
        (p.start.is_some() || p.end.is_some())
            && p.start.as_deref().is_none_or(|s| s <= today)
            && p.end.as_deref().is_none_or(|e| e >= today)
    };
    phases
        .iter()
        .find(covers)
        .or_else(|| phases.iter().find(|p| p.completed_at.is_none()))
        .map(|p| p.id)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ItemSummary {
    pub id: Uuid,
    pub kind: &'static str,
    pub title: String,
    pub progress: f64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PhaseSummary {
    pub id: Uuid,
    pub name: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub completed: bool,
    pub progress: f64,
    pub current: bool,
    pub items: Vec<ItemSummary>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Summary {
    pub phases: Vec<PhaseSummary>,
}

/// Every live Phase of a Roadmap in order, with progress, the current flag and its items.
pub fn summary(rows: &[Row], roadmap_id: Uuid, today: &str) -> Summary {
    let ids = by_id(rows);
    let current = current_phase(rows, roadmap_id, today);
    let phases = live_phases(rows, roadmap_id)
        .into_iter()
        .map(|p| {
            let items: Vec<ItemSummary> = phase_items(rows, p.id, &ids)
                .into_iter()
                .map(|r| ItemSummary {
                    id: r.id(),
                    kind: match r {
                        Row::Goal(_) => "goal",
                        Row::Project(_) => "project",
                        Row::Milestone(_) => "milestone",
                        _ => "task",
                    },
                    title: match r {
                        Row::Goal(g) => g.name.clone(),
                        Row::Project(p) => p.name.clone(),
                        Row::Milestone(m) => m.title.clone(),
                        Row::Task(t) => t.title.clone(),
                        _ => String::new(),
                    },
                    progress: item_progress(rows, r.id()),
                })
                .collect();
            let progress = if items.is_empty() {
                0.0
            } else {
                items.iter().map(|i| i.progress).sum::<f64>() / items.len() as f64
            };
            PhaseSummary {
                id: p.id,
                name: p.name.clone(),
                start: p.start.clone(),
                end: p.end.clone(),
                completed: p.completed_at.is_some(),
                progress,
                current: current == Some(p.id),
                items,
            }
        })
        .collect();
    Summary { phases }
}

/// The values of a `phase.*` or `roadmap.*` filter path for `row`, one per Placement (or its own).
pub(crate) fn path_values(
    row: &Row,
    path: &str,
    rows: &[Row],
    ids: &ById,
    today: &str,
) -> Vec<serde_json::Value> {
    use serde_json::json;
    let phase_val = |p: &Phase, field: &str| match field {
        "name" => json!(p.name),
        "current" => json!(current_phase(rows, p.roadmap_id, today) == Some(p.id)),
        _ => serde_json::Value::Null,
    };
    let roadmap_val = |rid: Uuid, field: &str| match (field, ids.get(&rid)) {
        ("name", Some(Row::Roadmap(r))) => json!(r.name),
        _ => serde_json::Value::Null,
    };
    let (kind, field) = path.split_once('.').unwrap_or((path, ""));
    match row {
        Row::Roadmap(r) if kind == "roadmap" => vec![roadmap_val(r.id, field)],
        Row::Phase(p) if kind == "phase" => vec![phase_val(p, field)],
        Row::Phase(p) if kind == "roadmap" => vec![roadmap_val(p.roadmap_id, field)],
        _ => rows
            .iter()
            .filter_map(|r| match r {
                Row::Placement(pl) if pl.item_id == row.id() => {
                    match pl.phase_id.and_then(|id| ids.get(&id)) {
                        Some(Row::Phase(p)) if p.trashed_at.is_none() => Some(match kind {
                            "phase" => phase_val(p, field),
                            _ => roadmap_val(p.roadmap_id, field),
                        }),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hlc, Milestone, Task};

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn task(n: u128, parent: Uuid, done: bool) -> Row {
        let mut t: Task = serde_json::from_value(serde_json::json!({
            "id": uid(n), "parent_id": parent, "title": format!("t{n}"),
        }))
        .unwrap();
        t.completed_at = done.then_some(1);
        Row::Task(t)
    }

    fn project(n: u128) -> Row {
        serde_json::from_value(serde_json::json!({
            "kind": "project", "id": uid(n), "parent_id": uid(1), "name": "P",
        }))
        .unwrap()
    }

    fn phase(n: u128, order: &str, start: Option<&str>, end: Option<&str>, done: bool) -> Row {
        Row::Phase(Phase {
            id: uid(n),
            roadmap_id: uid(50),
            name: format!("ph{n}"),
            order: order.into(),
            start: start.map(Into::into),
            end: end.map(Into::into),
            completed_at: done.then_some(1),
            archived_at: None,
            trashed_at: None,
        })
    }

    fn place(phase: u128, item: u128) -> Row {
        Row::Placement(Placement {
            id: placement_id(uid(50), uid(item)),
            roadmap_id: uid(50),
            item_id: uid(item),
            phase_id: Some(uid(phase)),
        })
    }

    fn milestone(n: u128, done: bool) -> Row {
        Row::Milestone(Milestone {
            id: uid(n),
            goal_id: uid(9),
            title: "m".into(),
            done,
            ..serde_json::from_value(serde_json::json!({
                "id": uid(n), "goal_id": uid(9), "title": "m"
            }))
            .unwrap()
        })
    }

    #[test]
    fn project_progress_is_the_done_share_of_its_tasks() {
        let rows = vec![
            project(10),
            task(11, uid(10), true),
            task(12, uid(10), false),
            task(13, uid(10), false),
            task(14, uid(10), true),
        ];
        assert_eq!(item_progress(&rows, uid(10)), 0.5);
        assert_eq!(item_progress(&[project(10)], uid(10)), 0.0);
        assert_eq!(item_progress(&rows, uid(11)), 1.0);
        assert_eq!(item_progress(&rows, uid(12)), 0.0);
        let rows = vec![milestone(20, true), milestone(21, false)];
        assert_eq!(item_progress(&rows, uid(20)), 1.0);
        assert_eq!(item_progress(&rows, uid(21)), 0.0);
    }

    #[test]
    fn phase_progress_is_the_mean_and_an_empty_phase_is_zero() {
        let rows = vec![
            phase(60, "a", None, None, false),
            phase(61, "b", None, None, false),
            task(11, uid(1), true),
            task(12, uid(1), false),
            place(60, 11),
            place(60, 12),
        ];
        assert_eq!(phase_progress(&rows, uid(60)), 0.5);
        assert_eq!(phase_progress(&rows, uid(61)), 0.0);
    }

    #[test]
    fn a_removed_or_moved_placement_leaves_the_phase() {
        let mut rows = vec![
            phase(60, "a", None, None, false),
            phase(61, "b", None, None, false),
            task(11, uid(1), true),
            place(60, 11),
        ];
        assert_eq!(phase_progress(&rows, uid(60)), 1.0);
        rows[3] = place(61, 11);
        assert_eq!(phase_progress(&rows, uid(60)), 0.0);
        assert_eq!(phase_progress(&rows, uid(61)), 1.0);
    }

    #[test]
    fn current_phase_is_the_dated_cover_else_the_first_unfinished() {
        let dated = vec![
            phase(60, "a", Some("2026-01-01"), Some("2026-01-31"), false),
            phase(61, "b", Some("2026-02-01"), Some("2026-02-28"), false),
        ];
        assert_eq!(current_phase(&dated, uid(50), "2026-02-10"), Some(uid(61)));
        // Outside every range: the first unfinished.
        assert_eq!(current_phase(&dated, uid(50), "2026-06-01"), Some(uid(60)));
        let undated = vec![
            phase(62, "b", None, None, false),
            phase(60, "a", None, None, true),
            phase(61, "aa", None, None, false),
        ];
        assert_eq!(
            current_phase(&undated, uid(50), "2026-06-01"),
            Some(uid(61))
        );
        let finished = vec![phase(60, "a", None, None, true)];
        assert_eq!(current_phase(&finished, uid(50), "2026-06-01"), None);
        assert_eq!(current_phase(&[], uid(50), "2026-06-01"), None);
    }

    #[test]
    fn placement_ids_are_stable_per_pair() {
        assert_eq!(placement_id(uid(1), uid(2)), placement_id(uid(1), uid(2)));
        assert_ne!(placement_id(uid(1), uid(2)), placement_id(uid(1), uid(3)));
        assert_ne!(placement_id(uid(1), uid(2)), placement_id(uid(2), uid(1)));
    }

    #[test]
    fn placement_set_rejects_a_wrong_id() {
        let cmd = Command {
            uuid: uid(99),
            op: Op::PlacementSet {
                id: uid(7),
                roadmap_id: uid(50),
                item_id: uid(11),
                phase_id: None,
            },
            hlc: Hlc {
                wall: 1,
                counter: 0,
                device: uid(2),
            },
        };
        assert_eq!(apply(None, &cmd).unwrap_err(), ErrorCode::BadField);
    }

    #[test]
    fn views_filter_items_by_phase_and_roadmap_paths() {
        let rows = vec![
            Row::Roadmap(Roadmap {
                id: uid(50),
                parent_id: uid(1),
                name: "Launch".into(),
                order: String::new(),
                archived_at: None,
                trashed_at: None,
                created_at: None,
            }),
            phase(60, "a", Some("2026-01-01"), Some("2026-01-31"), false),
            phase(61, "b", Some("2026-02-01"), Some("2026-02-28"), false),
            task(11, uid(1), false),
            task(12, uid(1), false),
            task(13, uid(1), false),
            place(60, 11),
            place(61, 12),
        ];
        let run = |path: &str, op: &str, value: serde_json::Value| {
            let view: crate::view::View = serde_json::from_value(serde_json::json!({
                "id": uid(70), "name": "v", "entity": "task",
                "filter": { "type": "group", "all": true, "children": [
                    { "type": "cond", "path": path, "op": op, "value": value }
                ]},
            }))
            .unwrap();
            crate::view::evaluate(&view, &rows, "2026-02-10")
                .into_iter()
                .flat_map(|g| g.ids)
                .collect::<Vec<_>>()
        };
        assert_eq!(run("phase.current", "is", true.into()), vec![uid(12)]);
        assert_eq!(run("phase.name", "is", "ph60".into()), vec![uid(11)]);
        assert_eq!(
            run("roadmap.name", "contains", "launch".into()),
            vec![uid(11), uid(12)]
        );
        assert_eq!(
            run("phase.name", "is_empty", serde_json::Value::Null),
            vec![uid(13)]
        );
    }
}
