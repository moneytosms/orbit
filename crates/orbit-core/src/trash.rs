//! Trash and Archive: per-item flags, and the rules that hide what sits under a flagged item.
use crate::{Command, ErrorCode, INBOX, Op, Row, Uuid};

/// Trashed items are purged after this long.
pub const KEEP_TRASH_MS: u64 = 30 * 24 * 60 * 60 * 1000;

/// Purge tombstones are kept this long; a client token older than that gets a full resync.
pub const KEEP_PURGED_MS: u64 = 90 * 24 * 60 * 60 * 1000;

/// Deepest chain walked, a guard against cycles in bad data.
const MAX_DEPTH: usize = 64;

impl Row {
    pub fn trashed_at(&self) -> Option<u64> {
        match self {
            Row::Area(r) => r.trashed_at,
            Row::Project(r) => r.trashed_at,
            Row::Section(r) => r.trashed_at,
            Row::Task(r) => r.trashed_at,
            Row::Goal(r) => r.trashed_at,
            Row::Milestone(r) => r.trashed_at,
            Row::Page(r) => r.trashed_at,
            Row::View(r) => r.trashed_at,
            Row::Routine(r) => r.trashed_at,
            Row::Label(_)
            | Row::Settings(_)
            | Row::Completion(_)
            | Row::Purged(_)
            | Row::Checkin(_) => None,
        }
    }

    pub fn archived_at(&self) -> Option<u64> {
        match self {
            Row::Area(r) => r.archived_at,
            Row::Project(r) => r.archived_at,
            Row::Goal(r) => r.archived_at,
            Row::Routine(r) => r.archived_at,
            _ => None,
        }
    }

    /// Sets or clears `trashed_at`. False (and no change) for kinds that can't be trashed.
    pub fn set_trashed(&mut self, at: Option<u64>) -> bool {
        match self {
            Row::Area(r) => r.trashed_at = at,
            Row::Project(r) => r.trashed_at = at,
            Row::Section(r) => r.trashed_at = at,
            Row::Task(r) => r.trashed_at = at,
            Row::Goal(r) => r.trashed_at = at,
            Row::Milestone(r) => r.trashed_at = at,
            Row::Page(r) => r.trashed_at = at,
            Row::View(r) => r.trashed_at = at,
            Row::Routine(r) => r.trashed_at = at,
            Row::Label(_)
            | Row::Settings(_)
            | Row::Completion(_)
            | Row::Purged(_)
            | Row::Checkin(_) => return false,
        }
        true
    }

    /// Sets or clears `archived_at`. False (and no change) unless an Area, Project or Goal.
    pub fn set_archived(&mut self, at: Option<u64>) -> bool {
        match self {
            Row::Area(r) => r.archived_at = at,
            Row::Project(r) => r.archived_at = at,
            Row::Goal(r) => r.archived_at = at,
            Row::Routine(r) => r.archived_at = at,
            _ => return false,
        }
        true
    }
}

/// Applies `item.delete`, `item.restore`, `item.archive` or `item.unarchive` to `existing`.
pub(crate) fn apply_flag(existing: Option<Row>, cmd: &Command) -> Result<Row, ErrorCode> {
    let Some(mut row) = existing else {
        return Err(ErrorCode::NotFound);
    };
    if row.id() == INBOX {
        return Err(ErrorCode::BadField);
    }
    let wall = cmd.hlc.wall;
    let ok = match &cmd.op {
        Op::ItemDelete { .. } => row.set_trashed(Some(row.trashed_at().unwrap_or(wall))),
        Op::ItemRestore { .. } => row.set_trashed(None),
        Op::ItemArchive { .. } => row.set_archived(Some(row.archived_at().unwrap_or(wall))),
        Op::ItemUnarchive { .. } => row.set_archived(None),
        _ => false,
    };
    if ok {
        Ok(row)
    } else {
        Err(ErrorCode::BadField)
    }
}

/// `row` followed by its ancestors, nearest first. `lookup` finds a row by id.
pub fn chain(row: &Row, lookup: &dyn Fn(Uuid) -> Option<Row>) -> Vec<Row> {
    let mut out = vec![row.clone()];
    let mut next = row.parent_id();
    while let Some(pid) = next {
        if out.len() >= MAX_DEPTH {
            break;
        }
        let Some(parent) = lookup(pid) else { break };
        next = parent.parent_id();
        out.push(parent);
    }
    out
}

/// Whether the row, or any ancestor, is trashed or archived.
pub fn hidden(row: &Row, lookup: &dyn Fn(Uuid) -> Option<Row>) -> bool {
    chain(row, lookup)
        .iter()
        .any(|r| r.trashed_at().is_some() || r.archived_at().is_some())
}

/// The topmost `trashed_at` along a chain from `chain`.
pub fn top_trashed(chain: &[Row]) -> Option<u64> {
    chain.iter().rev().find_map(Row::trashed_at)
}

/// Whether any row of a chain is archived.
pub fn any_archived(chain: &[Row]) -> bool {
    chain.iter().any(|r| r.archived_at().is_some())
}

/// A create under a trashed parent chain (`trashed_at` is the topmost one's). A Task is
/// captured to the Inbox instead; any other kind is created already trashed.
/// Returns the command to apply and the `trashed_at` to stamp on the new row, if any.
pub fn adjust_create(cmd: &Command, trashed_at: u64) -> (Command, Option<u64>) {
    match &cmd.op {
        Op::TaskCreate {
            id, title, order, ..
        } => (
            Command {
                op: Op::TaskCreate {
                    id: *id,
                    title: title.clone(),
                    parent_id: Some(INBOX),
                    order: order.clone(),
                },
                ..cmd.clone()
            },
            None,
        ),
        _ => (cmd.clone(), Some(trashed_at)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Hlc, Milestone, Project, Task, Uuid, apply};
    use std::collections::HashMap;

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 + n)
    }

    fn area(n: u128) -> Row {
        Row::Area(Area {
            id: uid(n),
            name: "A".into(),
            order: String::new(),
            trashed_at: None,
            archived_at: None,
        })
    }

    fn project(n: u128, parent: Uuid) -> Row {
        Row::Project(Project {
            id: uid(n),
            parent_id: parent,
            name: "P".into(),
            order: String::new(),
            goal_id: None,
            trashed_at: None,
            archived_at: None,
            touched_at: None,
        })
    }

    fn task(n: u128, parent: Uuid) -> Row {
        let create = op(
            "task.create",
            serde_json::json!({"id": uid(n), "title": "T", "parent_id": parent}),
            1,
        );
        apply(None, &create).unwrap()
    }

    fn op(kind: &str, args: serde_json::Value, wall: u64) -> Command {
        serde_json::from_value(serde_json::json!({
            "uuid": uid(900 + wall as u128),
            "type": kind,
            "args": args,
            "hlc": Hlc { wall, counter: 0, device: uid(999) },
        }))
        .unwrap()
    }

    fn lookup(rows: &[Row]) -> impl Fn(Uuid) -> Option<Row> + use<> {
        let map: HashMap<Uuid, Row> = rows.iter().map(|r| (r.id(), r.clone())).collect();
        move |id| map.get(&id).cloned()
    }

    #[test]
    fn delete_sets_trashed_at_once_and_restore_clears_it() {
        let p = project(1, uid(0));
        let del = |wall| op("item.delete", serde_json::json!({"id": uid(1)}), wall);
        let trashed = apply(Some(p), &del(50)).unwrap();
        assert_eq!(trashed.trashed_at(), Some(50));
        let again = apply(Some(trashed), &del(70)).unwrap();
        assert_eq!(again.trashed_at(), Some(50));
        let back = apply(
            Some(again),
            &op("item.restore", serde_json::json!({"id": uid(1)}), 80),
        )
        .unwrap();
        assert_eq!(back.trashed_at(), None);
    }

    #[test]
    fn delete_and_restore_reject_missing_rows_the_inbox_and_untrashable_kinds() {
        let del = op("item.delete", serde_json::json!({"id": uid(1)}), 5);
        assert_eq!(apply(None, &del), Err(ErrorCode::NotFound));
        let inbox = op("item.delete", serde_json::json!({"id": INBOX}), 5);
        assert_eq!(apply(Some(Row::inbox()), &inbox), Err(ErrorCode::BadField));
        let label = Row::Label(crate::Label {
            id: uid(1),
            name: "x".into(),
        });
        assert_eq!(apply(Some(label), &del), Err(ErrorCode::BadField));
    }

    #[test]
    fn only_areas_projects_and_goals_archive() {
        let arch = |id| op("item.archive", serde_json::json!({"id": id}), 9);
        let a = apply(Some(area(1)), &arch(uid(1))).unwrap();
        assert_eq!(a.archived_at(), Some(9));
        let t = task(2, uid(1));
        assert_eq!(apply(Some(t), &arch(uid(2))), Err(ErrorCode::BadField));
        let un = op("item.unarchive", serde_json::json!({"id": uid(1)}), 10);
        assert_eq!(apply(Some(a), &un).unwrap().archived_at(), None);
    }

    #[test]
    fn edits_to_a_trashed_task_apply_and_keep_it_trashed() {
        let mut t = task(2, uid(1));
        t.set_trashed(Some(5));
        let rename = op(
            "task.update",
            serde_json::json!({"id": uid(2), "title": "New"}),
            20,
        );
        let Row::Task(Task {
            title, trashed_at, ..
        }) = apply(Some(t), &rename).unwrap()
        else {
            panic!("a task")
        };
        assert_eq!((title.as_str(), trashed_at), ("New", Some(5)));
    }

    #[test]
    fn hidden_when_the_row_or_any_ancestor_is_trashed_or_archived() {
        let mut a = area(1);
        let p = project(2, uid(1));
        let t = task(3, uid(2));
        let mut rows = vec![a.clone(), p.clone(), t.clone()];
        assert!(!hidden(&t, &lookup(&rows)));
        a.set_archived(Some(1));
        rows[0] = a.clone();
        assert!(hidden(&t, &lookup(&rows)));
        assert!(hidden(&p, &lookup(&rows)));
        a.set_archived(None);
        let mut p2 = p.clone();
        p2.set_trashed(Some(1));
        rows = vec![a, p2.clone(), t.clone()];
        assert!(hidden(&t, &lookup(&rows)));
        assert!(!hidden(&rows[0], &lookup(&rows)));
    }

    #[test]
    fn a_milestone_is_hidden_with_its_goal() {
        let goal = apply(
            None,
            &op(
                "goal.create",
                serde_json::json!({"id": uid(2), "parent_id": uid(1), "name": "G"}),
                1,
            ),
        )
        .unwrap();
        let m = Row::Milestone(Milestone {
            id: uid(3),
            goal_id: uid(2),
            title: "M".into(),
            date: None,
            done: false,
            order: String::new(),
            trashed_at: None,
        });
        let mut g = goal;
        let rows = vec![area(1), g.clone(), m.clone()];
        assert!(!hidden(&m, &lookup(&rows)));
        g.set_trashed(Some(3));
        let rows = vec![area(1), g, m.clone()];
        assert!(hidden(&m, &lookup(&rows)));
    }

    #[test]
    fn a_task_created_under_a_trashed_parent_goes_to_the_inbox() {
        let create = op(
            "task.create",
            serde_json::json!({"id": uid(5), "title": "Offline", "parent_id": uid(2)}),
            7,
        );
        let (adjusted, stamp) = adjust_create(&create, 40);
        assert_eq!(stamp, None);
        let row = apply(None, &adjusted).unwrap();
        assert_eq!(row.parent_id(), Some(INBOX));
    }

    #[test]
    fn any_other_create_under_a_trashed_parent_is_stamped_with_the_ancestors_time() {
        let create = op(
            "section.create",
            serde_json::json!({"id": uid(5), "parent_id": uid(2), "name": "S"}),
            7,
        );
        let (adjusted, stamp) = adjust_create(&create, 40);
        assert_eq!(stamp, Some(40));
        assert_eq!(apply(None, &adjusted).unwrap().parent_id(), Some(uid(2)));
    }

    #[test]
    fn top_trashed_takes_the_topmost_trashed_row_of_a_chain() {
        let mut a = area(1);
        let mut p = project(2, uid(1));
        let t = task(3, uid(2));
        p.set_trashed(Some(10));
        a.set_trashed(Some(20));
        assert_eq!(top_trashed(&[t.clone(), p.clone(), a]), Some(20));
        assert_eq!(top_trashed(&[t, p]), Some(10));
        assert!(!any_archived(&[]));
    }

    fn routine(n: u128, parent: Uuid) -> Row {
        Row::Routine(crate::routine::Routine {
            id: uid(n),
            parent_id: parent,
            name: "R".into(),
            schedule: crate::routine::Schedule::Daily,
            start: "2026-03-02".into(),
            length_days: None,
            time: None,
            goal_id: None,
            pauses: vec![],
            order: String::new(),
            created_at: None,
            trashed_at: None,
            archived_at: None,
        })
    }

    #[test]
    fn a_routine_archives_and_unarchives_and_its_check_ins_are_hidden() {
        let arch = |id| op("item.archive", serde_json::json!({"id": id}), 9);
        let r = apply(Some(routine(4, uid(1))), &arch(uid(4))).unwrap();
        assert_eq!(r.archived_at(), Some(9));
        let un = op("item.unarchive", serde_json::json!({"id": uid(4)}), 10);
        assert_eq!(apply(Some(r.clone()), &un).unwrap().archived_at(), None);
        let c = Row::Checkin(crate::routine::Checkin {
            id: crate::routine::checkin_id(uid(4), "2026-03-02"),
            routine_id: uid(4),
            date: "2026-03-02".into(),
            state: crate::routine::CheckState::Done,
            cleared: false,
        });
        assert_eq!(c.archived_at(), None);
        assert!(!c.clone().set_archived(Some(1)));
        assert!(hidden(&c, &lookup(&[area(1), r.clone(), c.clone()])));
        assert!(!hidden(
            &c,
            &lookup(&[area(1), routine(4, uid(1)), c.clone()])
        ));
    }

    #[test]
    fn a_routine_goes_to_the_trash_and_a_check_in_does_not() {
        let del = op("item.delete", serde_json::json!({"id": uid(4)}), 9);
        let r = apply(Some(routine(4, uid(1))), &del).unwrap();
        assert_eq!(r.trashed_at(), Some(9));
    }
}
