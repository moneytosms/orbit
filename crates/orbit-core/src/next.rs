//! What's next: a tiered ranking of Tasks and dated Milestones, with a reason for each.
use crate::goal::{behind_pace, effective_goal, progress};
use crate::parse::parse_ymd;
use crate::{Row, Uuid, trash};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The What's next settings.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Cfg {
    /// Days ahead a deadline counts as near.
    pub deadline_days: u32,
    /// Days ahead a due date counts as upcoming.
    pub due_days: u32,
    /// Whether P1 items without any date are listed.
    pub undated_p1: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            deadline_days: 3,
            due_days: 7,
            undated_p1: true,
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct NextItem {
    pub id: Uuid,
    /// "task" or "milestone".
    pub kind: &'static str,
    pub title: String,
    /// 1 (pinned) to 7 (due soon).
    pub tier: u8,
    pub reason: String,
    /// Area / Project / parent Task names, outermost first.
    pub breadcrumb: Vec<String>,
}

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

fn weekday(day: i64) -> &'static str {
    // Day 0 is 1970-01-01, a Thursday.
    WEEKDAYS[(day + 4).rem_euclid(7) as usize]
}

/// "tomorrow", "in N days" for a deadline or `None` past the window.
fn due_phrase(date: &str, day: i64, today: i64) -> String {
    let diff = day - today;
    if diff == 1 {
        "tomorrow".into()
    } else if diff < 7 {
        weekday(day).into()
    } else {
        date.into()
    }
}

struct Cand {
    tier: u8,
    reason: String,
    priority: u8,
    date: String,
    time: String,
    created: u64,
    id: Uuid,
    kind: &'static str,
    title: String,
    breadcrumb: Vec<String>,
}

/// The list, best first. Items matching no tier are left off.
pub fn whats_next(rows: &[Row], today: &str, now_time: &str, cfg: &Cfg) -> Vec<NextItem> {
    let Some(today_day) = parse_ymd(today) else {
        return vec![];
    };
    let by_id: BTreeMap<Uuid, &Row> = rows.iter().map(|r| (r.id(), r)).collect();
    let lookup = |id: Uuid| by_id.get(&id).map(|r| (*r).clone());
    let mut behind: BTreeMap<Uuid, Option<String>> = BTreeMap::new();
    let mut behind_name = |goal: Option<Uuid>| -> Option<String> {
        let id = goal?;
        behind
            .entry(id)
            .or_insert_with(|| match by_id.get(&id) {
                Some(Row::Goal(g))
                    if g.completed_at.is_none()
                        && g.trashed_at.is_none()
                        && behind_pace(g, progress(g, rows), today) =>
                {
                    Some(g.name.clone())
                }
                _ => None,
            })
            .clone()
    };

    let mut out: Vec<Cand> = vec![];
    for row in rows {
        let (id, pinned, not_today, priority, due, deadline, created, goal) = match row {
            Row::Task(t) if t.completed_at.is_none() => (
                t.id,
                &t.pinned_on,
                &t.not_today_on,
                t.priority,
                t.due.clone(),
                t.deadline.clone(),
                t.created_at.unwrap_or(u64::MAX),
                effective_goal(t, rows),
            ),
            Row::Milestone(m) if !m.done && m.date.is_some() => (
                m.id,
                &m.pinned_on,
                &m.not_today_on,
                4,
                None,
                m.date.clone(),
                u64::MAX,
                Some(m.goal_id),
            ),
            _ => continue,
        };
        if not_today.as_deref() == Some(today) || trash::hidden(row, &lookup) {
            continue;
        }
        let is_milestone = matches!(row, Row::Milestone(_));
        let deadline_day = deadline.as_deref().and_then(parse_ymd);
        let due_day = due.as_ref().and_then(|d| parse_ymd(&d.date));
        let due_time = due.as_ref().and_then(|d| d.time.clone());

        let (tier, reason): (u8, String) = if pinned.as_deref() == Some(today) {
            (1, "Pinned for today".into())
        } else if deadline_day.is_some_and(|d| d < today_day) {
            (2, "Deadline passed".into())
        } else if deadline_day.is_some_and(|d| d == today_day) {
            (2, "Deadline today".into())
        } else if let Some(d) = due
            .as_ref()
            .filter(|_| due_day.is_some_and(|d| d <= today_day))
        {
            let past = due_day.is_some_and(|x| x < today_day);
            let reason = match &d.time {
                _ if past => "Overdue".into(),
                Some(t) if t.as_str() < now_time => "Overdue".into(),
                Some(t) => format!("Due {t}"),
                None => "Due today".into(),
            };
            (3, reason)
        } else if let Some(d) =
            deadline_day.filter(|d| (d - today_day) <= i64::from(cfg.deadline_days))
        {
            let diff = d - today_day;
            let r = if diff == 1 {
                "Deadline tomorrow".into()
            } else {
                format!("Deadline in {diff} days")
            };
            (4, r)
        } else if let Some(name) = behind_name(goal) {
            (5, format!("{name} behind pace"))
        } else if cfg.undated_p1 && priority == 1 && due.is_none() && deadline.is_none() {
            (6, "P1, no date".into())
        } else if let Some((d, day)) = due
            .as_ref()
            .and_then(|d| due_day.map(|day| (d.date.clone(), day)))
            .or_else(|| {
                // A Milestone's date doubles as its due date for the due window.
                if is_milestone {
                    deadline
                        .clone()
                        .and_then(|d| parse_ymd(&d).map(|day| (d, day)))
                } else {
                    None
                }
            })
            .filter(|(_, day)| day - today_day <= i64::from(cfg.due_days))
        {
            (7, format!("Due {}", due_phrase(&d, day, today_day)))
        } else {
            continue;
        };

        let date = match tier {
            2 | 4 => deadline.clone().unwrap_or_default(),
            _ => due
                .as_ref()
                .map(|d| d.date.clone())
                .or(deadline.clone())
                .unwrap_or_else(|| "9999-99-99".into()),
        };
        let time = if tier == 3 {
            due_time.unwrap_or_else(|| "99:99".into())
        } else {
            String::new()
        };
        out.push(Cand {
            tier,
            reason,
            priority,
            date,
            time,
            created,
            id,
            kind: if is_milestone { "milestone" } else { "task" },
            title: match row {
                Row::Task(t) => t.title.clone(),
                Row::Milestone(m) => m.title.clone(),
                _ => String::new(),
            },
            breadcrumb: breadcrumb(row, &by_id),
        });
    }
    out.sort_by(|a, b| {
        (a.tier, a.priority, &a.date, &a.time, a.created, a.id)
            .cmp(&(b.tier, b.priority, &b.date, &b.time, b.created, b.id))
    });
    out.into_iter()
        .map(|c| NextItem {
            id: c.id,
            kind: c.kind,
            title: c.title,
            tier: c.tier,
            reason: c.reason,
            breadcrumb: c.breadcrumb,
        })
        .collect()
}

/// Names of the ancestors, outermost first. Sections are left out.
fn breadcrumb(row: &Row, by_id: &BTreeMap<Uuid, &Row>) -> Vec<String> {
    let mut names = vec![];
    let mut next = row.parent_id();
    for _ in 0..32 {
        let Some(parent) = next.and_then(|id| by_id.get(&id)) else {
            break;
        };
        match parent {
            Row::Area(a) => names.push(a.name.clone()),
            Row::Project(p) => names.push(p.name.clone()),
            Row::Task(t) => names.push(t.title.clone()),
            Row::Goal(g) => names.push(g.name.clone()),
            _ => {}
        }
        next = parent.parent_id();
    }
    names.reverse();
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{Goal, Milestone, Progress};
    use crate::{Area, Due, Project, Task};

    const TODAY: &str = "2026-03-10"; // a Tuesday

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn task(n: u128) -> Task {
        Task {
            id: uid(n),
            parent_id: uid(1),
            title: format!("T{n}"),
            completed_at: None,
            order: String::new(),
            description: String::new(),
            due: None,
            deadline: None,
            duration_min: None,
            priority: 4,
            label_ids: vec![],
            recurrence: None,
            goal_id: None,
            created_at: Some(n as u64),
            trashed_at: None,
            pinned_on: None,
            not_today_on: None,
            links: vec![],
            reminders: vec![],
        }
    }

    fn area() -> Row {
        Row::Area(Area {
            id: uid(1),
            name: "Home".into(),
            order: String::new(),
            archived_at: None,
            trashed_at: None,
        })
    }

    fn due(date: &str, time: Option<&str>) -> Option<Due> {
        Some(Due {
            date: date.into(),
            time: time.map(Into::into),
        })
    }

    fn list(tasks: Vec<Task>) -> Vec<(u128, u8, String)> {
        let mut rows = vec![area()];
        rows.extend(tasks.into_iter().map(Row::Task));
        whats_next(&rows, TODAY, "12:00", &Cfg::default())
            .into_iter()
            .map(|i| (i.id.as_u128(), i.tier, i.reason))
            .collect()
    }

    fn one(t: Task) -> Option<(u8, String)> {
        list(vec![t]).pop().map(|(_, tier, r)| (tier, r))
    }

    #[test]
    fn tier_one_pinned_today_only() {
        let mut t = task(2);
        t.pinned_on = Some(TODAY.into());
        assert_eq!(one(t.clone()), Some((1, "Pinned for today".into())));
        t.pinned_on = Some("2026-03-09".into());
        assert_eq!(one(t), None);
    }

    #[test]
    fn tier_two_deadline_passed_or_today() {
        let mut t = task(2);
        t.deadline = Some("2026-03-09".into());
        assert_eq!(one(t.clone()), Some((2, "Deadline passed".into())));
        t.deadline = Some(TODAY.into());
        assert_eq!(one(t), Some((2, "Deadline today".into())));
    }

    #[test]
    fn tier_three_overdue_or_due_today() {
        let mut t = task(2);
        t.due = due("2026-03-01", None);
        assert_eq!(one(t.clone()), Some((3, "Overdue".into())));
        t.due = due(TODAY, None);
        assert_eq!(one(t.clone()), Some((3, "Due today".into())));
        t.due = due(TODAY, Some("14:30"));
        assert_eq!(one(t.clone()), Some((3, "Due 14:30".into())));
        t.due = due(TODAY, Some("09:00"));
        assert_eq!(one(t), Some((3, "Overdue".into())));
    }

    #[test]
    fn tier_three_timed_items_sort_by_time() {
        let (mut a, mut b, mut c) = (task(2), task(3), task(4));
        a.due = due(TODAY, Some("18:00"));
        b.due = due(TODAY, Some("13:00"));
        c.due = due(TODAY, None);
        let ids: Vec<u128> = list(vec![a, b, c]).into_iter().map(|x| x.0).collect();
        assert_eq!(ids, vec![3, 2, 4]);
    }

    #[test]
    fn tier_four_deadline_within_window() {
        let mut t = task(2);
        t.deadline = Some("2026-03-11".into());
        assert_eq!(one(t.clone()), Some((4, "Deadline tomorrow".into())));
        t.deadline = Some("2026-03-13".into());
        assert_eq!(one(t.clone()), Some((4, "Deadline in 3 days".into())));
        t.deadline = Some("2026-03-14".into());
        assert_eq!(one(t), None);
    }

    fn goal_rows(progress_done: bool) -> Vec<Row> {
        let mut rows = vec![
            area(),
            Row::Goal(Goal {
                id: uid(50),
                parent_id: uid(1),
                name: "Ship".into(),
                done_condition: String::new(),
                progress: Progress::Manual,
                start: Some("2026-03-01".into()),
                target_date: Some("2026-03-20".into()),
                completed_at: None,
                order: String::new(),
                touched_at: None,
                archived_at: None,
                trashed_at: None,
            }),
        ];
        let mut t = task(2);
        t.goal_id = Some(uid(50));
        rows.push(Row::Task(t));
        if progress_done && let Row::Goal(g) = &mut rows[1] {
            g.completed_at = Some(1);
        }
        rows
    }

    #[test]
    fn tier_five_behind_pace_goal() {
        let got = whats_next(&goal_rows(false), TODAY, "12:00", &Cfg::default());
        assert_eq!(got.len(), 1);
        assert_eq!(
            (got[0].tier, got[0].reason.as_str()),
            (5, "Ship behind pace")
        );
        assert!(whats_next(&goal_rows(true), TODAY, "12:00", &Cfg::default()).is_empty());
    }

    #[test]
    fn tier_six_undated_p1_toggle() {
        let mut t = task(2);
        t.priority = 1;
        assert_eq!(one(t.clone()), Some((6, "P1, no date".into())));
        let rows = vec![area(), Row::Task(t)];
        let cfg = Cfg {
            undated_p1: false,
            ..Cfg::default()
        };
        assert!(whats_next(&rows, TODAY, "12:00", &cfg).is_empty());
    }

    #[test]
    fn tier_seven_due_within_window() {
        let mut t = task(2);
        t.due = due("2026-03-11", None);
        assert_eq!(one(t.clone()), Some((7, "Due tomorrow".into())));
        t.due = due("2026-03-13", None);
        assert_eq!(one(t.clone()), Some((7, "Due Fri".into())));
        t.due = due("2026-03-17", None);
        assert_eq!(one(t.clone()), Some((7, "Due 2026-03-17".into())));
        t.due = due("2026-03-18", None);
        assert_eq!(one(t), None);
    }

    #[test]
    fn highest_tier_wins_and_excluded_items_drop() {
        let mut t = task(2);
        t.due = due(TODAY, None);
        t.deadline = Some(TODAY.into());
        assert_eq!(one(t.clone()), Some((2, "Deadline today".into())));
        t.not_today_on = Some(TODAY.into());
        assert_eq!(one(t.clone()), None);
        t.not_today_on = None;
        t.completed_at = Some(1);
        assert_eq!(one(t.clone()), None);
        t.completed_at = None;
        t.trashed_at = Some(1);
        assert_eq!(one(t), None);
    }

    #[test]
    fn within_a_tier_priority_then_date_then_created() {
        let (mut a, mut b, mut c, mut d) = (task(2), task(3), task(4), task(5));
        for t in [&mut a, &mut b, &mut c, &mut d] {
            t.due = due("2026-03-01", None);
        }
        a.priority = 3;
        b.priority = 2;
        c.priority = 2;
        c.due = due("2026-02-01", None);
        d.priority = 2;
        d.created_at = Some(1);
        let ids: Vec<u128> = list(vec![a, b, c, d]).into_iter().map(|x| x.0).collect();
        assert_eq!(ids, vec![4, 5, 3, 2]);
    }

    #[test]
    fn breadcrumb_and_dated_milestones() {
        let mut t = task(3);
        t.parent_id = uid(2);
        t.pinned_on = Some(TODAY.into());
        let rows = vec![
            area(),
            Row::Project(Project {
                id: uid(2),
                parent_id: uid(1),
                name: "Proj".into(),
                order: String::new(),
                goal_id: None,
                touched_at: None,
                trashed_at: None,
                archived_at: None,
            }),
            Row::Task(t),
            Row::Milestone(Milestone {
                id: uid(9),
                goal_id: uid(50),
                title: "M".into(),
                date: Some(TODAY.into()),
                done: false,
                order: String::new(),
                trashed_at: None,
                pinned_on: None,
                not_today_on: None,
            }),
        ];
        let got = whats_next(&rows, TODAY, "12:00", &Cfg::default());
        assert_eq!(got[0].breadcrumb, vec!["Home", "Proj"]);
        // The Milestone's Goal is missing, so it is not hidden and ranks as a deadline.
        assert_eq!((got[1].id, got[1].tier), (uid(9), 2));
    }
}
