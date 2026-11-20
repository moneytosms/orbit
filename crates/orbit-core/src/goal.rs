//! Goals, Milestones and their pure progress maths.
use crate::parse::parse_ymd;
use crate::{ErrorCode, Row, Task, Uuid};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One logged amount towards a Target Goal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub amount: f64,
    /// `YYYY-MM-DD`.
    pub date: String,
    #[serde(default)]
    pub note: String,
}

/// How a Goal's progress is measured.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Progress {
    /// Done linked Tasks, Milestones and Sub-goals over their total.
    Count,
    /// Logged entries over a target amount.
    Target {
        target: f64,
        #[serde(default)]
        entries: Vec<Entry>,
    },
    /// Done when marked complete.
    Manual,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Goal {
    pub id: Uuid,
    /// An Area, or a Goal for a Sub-goal.
    pub parent_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub done_condition: String,
    pub progress: Progress,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub start: Option<String>,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub target_date: Option<String>,
    #[serde(default)]
    pub completed_at: Option<u64>,
    #[serde(default)]
    pub order: String,
    /// Unix ms of the last edit.
    #[serde(default)]
    pub touched_at: Option<u64>,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// Unix ms when it was archived.
    #[serde(default)]
    pub archived_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Milestone {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub title: String,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub order: String,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// `YYYY-MM-DD`; pinned to the top of What's next only on that day.
    #[serde(default)]
    pub pinned_on: Option<String>,
    /// `YYYY-MM-DD`; hidden from What's next only on that day.
    #[serde(default)]
    pub not_today_on: Option<String>,
}

/// The Goal a Task counts towards: its own link, else its Project ancestor's.
pub fn effective_goal(task: &Task, rows: &[Row]) -> Option<Uuid> {
    if task.goal_id.is_some() {
        return task.goal_id;
    }
    let by_id: BTreeMap<Uuid, &Row> = rows.iter().map(|r| (r.id(), r)).collect();
    let mut next = Some(task.parent_id);
    // The depth cap guards against a parent cycle.
    for _ in 0..32 {
        match by_id.get(&next?)? {
            Row::Project(p) => return p.goal_id,
            other => next = other.parent_id(),
        }
    }
    None
}

/// `(done, total)` for a Goal counted by units or a target amount; `None` for a Manual Goal.
pub fn counts(goal: &Goal, rows: &[Row]) -> Option<(f64, f64)> {
    match &goal.progress {
        Progress::Count => {
            let (mut units, mut done) = (0u32, 0u32);
            for r in rows {
                let (is_unit, is_done) = match r {
                    Row::Task(t) if effective_goal(t, rows) == Some(goal.id) => {
                        (true, t.completed_at.is_some())
                    }
                    Row::Milestone(m) if m.goal_id == goal.id => (true, m.done),
                    Row::Goal(g) if g.parent_id == goal.id => (true, g.completed_at.is_some()),
                    _ => (false, false),
                };
                units += is_unit as u32;
                done += (is_unit && is_done) as u32;
            }
            Some((done as f64, units as f64))
        }
        Progress::Target { target, entries } => {
            let sum: f64 = entries.iter().map(|e| e.amount).sum::<f64>()
                + crate::routine::done_checkins(goal.id, rows) as f64;
            Some((sum, *target))
        }
        Progress::Manual => None,
    }
}

/// Fraction of `goal` done, 0..=1.
pub fn progress(goal: &Goal, rows: &[Row]) -> f64 {
    match counts(goal, rows) {
        Some((_, total)) if total <= 0.0 => 0.0,
        Some((done, total)) => (done / total).clamp(0.0, 1.0),
        None => f64::from(goal.completed_at.is_some()),
    }
}

/// Below the straight line from `start` to `target_date`. Without both dates: never behind.
pub fn behind_pace(goal: &Goal, progress: f64, today: &str) -> bool {
    let (Some(start), Some(target)) = (&goal.start, &goal.target_date) else {
        return false;
    };
    let (Some(start), Some(target), Some(today)) =
        (parse_ymd(start), parse_ymd(target), parse_ymd(today))
    else {
        return false;
    };
    let span = target - start;
    let expected = if span <= 0 {
        if today >= target { 1.0 } else { 0.0 }
    } else {
        ((today - start) as f64 / span as f64).clamp(0.0, 1.0)
    };
    progress < expected
}

/// A Goal sits under an Area, or is a Sub-goal under a top-level Goal (`parent`, whose own
/// parent is `grandparent`). Sub-goals go one level deep only.
pub fn check_nesting(parent: &Row, grandparent: Option<&Row>) -> Result<(), ErrorCode> {
    match parent {
        Row::Area(_) => Ok(()),
        Row::Goal(_) if matches!(grandparent, Some(Row::Area(_))) => Ok(()),
        _ => Err(ErrorCode::BadParent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Project};

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 + n)
    }

    fn goal(n: u128, parent: Uuid, p: Progress) -> Goal {
        Goal {
            id: uid(n),
            parent_id: parent,
            name: "G".into(),
            done_condition: String::new(),
            progress: p,
            start: None,
            target_date: None,
            completed_at: None,
            order: String::new(),
            touched_at: None,
            trashed_at: None,
            archived_at: None,
        }
    }

    fn task(n: u128, parent: Uuid, goal_id: Option<Uuid>, done: bool) -> Row {
        Row::Task(Task {
            id: uid(n),
            parent_id: parent,
            title: "T".into(),
            completed_at: done.then_some(1),
            order: String::new(),
            description: String::new(),
            due: None,
            deadline: None,
            duration_min: None,
            priority: 4,
            label_ids: vec![],
            recurrence: None,
            goal_id,
            created_at: None,
            trashed_at: None,
            pinned_on: None,
            not_today_on: None,
            links: vec![],
            reminders: vec![],
            blobs: vec![],
        })
    }

    fn project(n: u128, parent: Uuid, goal_id: Option<Uuid>) -> Row {
        Row::Project(Project {
            color: String::new(),
            icon: String::new(),
            cover: String::new(),
            id: uid(n),
            parent_id: parent,
            name: "P".into(),
            order: String::new(),
            goal_id,
            trashed_at: None,
            archived_at: None,
            touched_at: None,
        })
    }

    fn milestone(n: u128, goal: Uuid, done: bool) -> Row {
        Row::Milestone(Milestone {
            id: uid(n),
            goal_id: goal,
            title: "M".into(),
            date: None,
            done,
            order: String::new(),
            trashed_at: None,
            pinned_on: None,
            not_today_on: None,
        })
    }

    fn area(n: u128) -> Row {
        Row::Area(Area {
            color: String::new(),
            icon: String::new(),
            cover: String::new(),
            id: uid(n),
            name: "A".into(),
            order: String::new(),
            trashed_at: None,
            archived_at: None,
        })
    }

    #[test]
    fn count_is_done_tasks_milestones_and_subgoals_over_total() {
        let g = goal(1, uid(100), Progress::Count);
        assert_eq!(progress(&g, &[]), 0.0);
        let mut sub = goal(5, g.id, Progress::Manual);
        sub.completed_at = Some(1);
        let rows = vec![
            task(2, uid(100), Some(g.id), true),
            task(3, uid(100), Some(g.id), false),
            milestone(4, g.id, true),
            Row::Goal(sub),
            task(6, uid(100), None, true),
        ];
        assert_eq!(progress(&g, &rows), 3.0 / 4.0);
    }

    #[test]
    fn target_sums_entries_and_clamps() {
        let entry = |amount| Entry {
            amount,
            date: "2026-01-01".into(),
            note: String::new(),
        };
        let g = |amounts: Vec<f64>| {
            goal(
                1,
                uid(100),
                Progress::Target {
                    target: 10.0,
                    entries: amounts.into_iter().map(entry).collect(),
                },
            )
        };
        assert_eq!(progress(&g(vec![]), &[]), 0.0);
        assert_eq!(progress(&g(vec![2.0, 3.0]), &[]), 0.5);
        assert_eq!(progress(&g(vec![20.0]), &[]), 1.0);
    }

    #[test]
    fn each_done_check_in_of_a_linked_routine_adds_one_to_a_target() {
        use crate::routine::{CheckState, Checkin, Schedule, checkin_id};
        let g = goal(
            1,
            uid(100),
            Progress::Target {
                target: 4.0,
                entries: vec![],
            },
        );
        let routine = |n: u128, link: Option<Uuid>| {
            Row::Routine(crate::routine::Routine {
                id: uid(n),
                parent_id: uid(100),
                name: "R".into(),
                schedule: Schedule::Daily,
                start: "2026-03-02".into(),
                length_days: None,
                time: None,
                goal_id: link,
                reminder: None,
                pauses: vec![],
                order: String::new(),
                created_at: None,
                trashed_at: None,
                archived_at: None,
            })
        };
        let tick = |r: u128, date: &str, state, cleared| {
            Row::Checkin(Checkin {
                id: checkin_id(uid(r), date),
                routine_id: uid(r),
                date: date.into(),
                state,
                cleared,
            })
        };
        let rows = vec![
            routine(2, Some(g.id)),
            routine(3, None),
            tick(2, "2026-03-02", CheckState::Done, false),
            tick(2, "2026-03-03", CheckState::Done, false),
            tick(2, "2026-03-04", CheckState::Skipped, false),
            tick(2, "2026-03-05", CheckState::Done, true),
            tick(3, "2026-03-02", CheckState::Done, false),
        ];
        assert_eq!(progress(&g, &rows), 0.5);
    }

    #[test]
    fn manual_is_all_or_nothing() {
        let mut g = goal(1, uid(100), Progress::Manual);
        assert_eq!(progress(&g, &[]), 0.0);
        g.completed_at = Some(5);
        assert_eq!(progress(&g, &[]), 1.0);
    }

    #[test]
    fn a_task_inherits_its_projects_goal_unless_it_overrides() {
        let (g1, g2) = (uid(10), uid(11));
        let rows = vec![
            area(100),
            project(1, uid(100), Some(g1)),
            project(2, uid(100), None),
        ];
        let Row::Task(inherits) = task(3, uid(1), None, false) else {
            panic!()
        };
        let Row::Task(overrides) = task(4, uid(1), Some(g2), false) else {
            panic!()
        };
        let Row::Task(nowhere) = task(5, uid(2), None, false) else {
            panic!()
        };
        assert_eq!(effective_goal(&inherits, &rows), Some(g1));
        assert_eq!(effective_goal(&overrides, &rows), Some(g2));
        assert_eq!(effective_goal(&nowhere, &rows), None);
        // A subtask of a task in the project inherits too.
        let mut all = rows;
        all.push(task(6, uid(1), None, false));
        let Row::Task(sub) = task(7, uid(6), None, false) else {
            panic!()
        };
        assert_eq!(effective_goal(&sub, &all), Some(g1));
    }

    #[test]
    fn behind_pace_follows_the_straight_line() {
        let mut g = goal(1, uid(100), Progress::Count);
        assert!(!behind_pace(&g, 0.0, "2026-06-01"));
        g.target_date = Some("2026-11-01".into());
        assert!(
            !behind_pace(&g, 0.0, "2026-06-01"),
            "no start: never behind"
        );
        g.start = Some("2026-01-01".into());
        g.target_date = Some("2026-01-11".into());
        assert!(!behind_pace(&g, 0.5, "2026-01-06"));
        assert!(!behind_pace(&g, 0.6, "2026-01-06"));
        assert!(behind_pace(&g, 0.4, "2026-01-06"));
        assert!(!behind_pace(&g, 0.0, "2025-12-01"));
        assert!(behind_pace(&g, 0.9, "2026-03-01"));
        assert!(!behind_pace(&g, 1.0, "2026-03-01"));
    }

    #[test]
    fn a_subgoal_of_a_subgoal_is_rejected() {
        let a = area(100);
        let top = Row::Goal(goal(1, uid(100), Progress::Count));
        let sub = Row::Goal(goal(2, uid(1), Progress::Count));
        assert_eq!(check_nesting(&a, None), Ok(()));
        assert_eq!(check_nesting(&top, Some(&a)), Ok(()));
        assert_eq!(check_nesting(&sub, Some(&top)), Err(ErrorCode::BadParent));
        assert_eq!(check_nesting(&top, None), Err(ErrorCode::BadParent));
    }
}
