//! Stardust, Levels and targets, all derived from completion records and Check-ins.
//! Nothing here is stored: every figure is recomputed from the rows.
use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::parse::{fmt_date, parse_ymd, weekday};
use crate::routine::{CheckState, local_day};
use crate::{Row, Settings, Uuid};

pub const CHECKIN: u32 = 20;
pub const MILESTONE: u32 = 250;
pub const SUB_GOAL: u32 = 250;
pub const GOAL: u32 = 500;
/// What a Task whose row and recorded points are both gone counts for.
pub const DEFAULT_TASK: u32 = 15;
pub const DEFAULT_DAILY: u32 = 5;
pub const DEFAULT_WEEKLY: u32 = 25;

/// `(name, total Stardust to reach it)`, Level 1 first.
pub const LEVELS: [(&str, u64); 13] = [
    ("Dust", 0),
    ("Meteor", 1500),
    ("Comet", 5000),
    ("Moon", 12000),
    ("Planet", 22000),
    ("Star", 36000),
    ("Nova", 53000),
    ("Nebula", 73000),
    ("Pulsar", 96000),
    ("Quasar", 122000),
    ("Galaxy", 153000),
    ("Black Hole", 188000),
    ("Universe", 229400),
];

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Level {
    pub n: u32,
    pub name: &'static str,
    /// Total at which this Level began.
    pub floor: u64,
    /// Total at which the next Level begins; `None` at the top.
    pub next: Option<u64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct DayPoints {
    pub day: String,
    pub points: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Targets {
    pub today_done: u32,
    pub week_done: u32,
    pub daily: u32,
    pub weekly: u32,
    pub streak: u32,
    pub vacation: bool,
    pub days_off: Vec<u8>,
}

/// One award: the Day-start date it falls on, its Stardust, and whether a Task earned it.
struct Award {
    day: String,
    points: u32,
    task: bool,
}

/// Stardust for a Task of this priority.
pub fn task_points(priority: u8) -> u32 {
    match priority {
        1 => 80,
        2 => 40,
        3 => 25,
        _ => 15,
    }
}

/// The Level a total of Stardust is at.
pub fn level(total: u64) -> Level {
    let i = LEVELS.iter().rposition(|(_, f)| total >= *f).unwrap_or(0);
    Level {
        n: i as u32 + 1,
        name: LEVELS[i].0,
        floor: LEVELS[i].1,
        next: LEVELS.get(i + 1).map(|(_, f)| *f),
    }
}

/// Every award in `rows`. A completion whose item is open again earns nothing, so un-completing
/// removes it; trashing keeps it. A non-recurring item completed twice counts once.
fn awards(rows: &[Row], utc_offset_min: i32, day_start: &str) -> Vec<Award> {
    let by_id: HashMap<Uuid, &Row> = rows.iter().map(|r| (r.id(), r)).collect();
    let mut out = Vec::new();
    let mut seen: HashSet<Uuid> = HashSet::new();
    let mut comps: Vec<_> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Completion(c) => Some(c),
            _ => None,
        })
        .collect();
    comps.sort_by_key(|c| std::cmp::Reverse(c.at));
    for c in comps {
        let (points, task, once) = match by_id.get(&c.item_id) {
            Some(Row::Task(t)) => {
                if t.recurrence.is_none() && t.completed_at.is_none() {
                    continue;
                }
                (task_points(t.priority), true, t.recurrence.is_none())
            }
            Some(Row::Goal(g)) => {
                if g.completed_at.is_none() {
                    continue;
                }
                let sub = matches!(by_id.get(&g.parent_id), Some(Row::Goal(_)));
                (if sub { SUB_GOAL } else { GOAL }, false, true)
            }
            Some(Row::Milestone(m)) => {
                if !m.done {
                    continue;
                }
                (MILESTONE, false, true)
            }
            Some(_) => continue,
            None => {
                let p = c.points.unwrap_or(DEFAULT_TASK);
                (p, matches!(p, 15 | 25 | 40 | 80), false)
            }
        };
        if once && !seen.insert(c.item_id) {
            continue;
        }
        out.push(Award {
            day: local_day(c.at, utc_offset_min, day_start),
            points,
            task,
        });
    }
    for r in rows {
        if let Row::Checkin(k) = r
            && !k.cleared
            && k.state == CheckState::Done
        {
            out.push(Award {
                day: k.date.clone(),
                points: CHECKIN,
                task: false,
            });
        }
        if let Row::Award(a) = r {
            out.push(Award {
                day: local_day(a.earned_at, utc_offset_min, day_start),
                points: a.points,
                task: false,
            });
        }
    }
    out
}

/// Completed Tasks in all, and per local day (`YYYY-MM-DD`), counting the same as Stardust.
pub fn task_counts(
    rows: &[Row],
    utc_offset_min: i32,
    day_start: &str,
) -> (u32, HashMap<String, u32>) {
    let mut total = 0;
    let mut per_day: HashMap<String, u32> = HashMap::new();
    for a in awards(rows, utc_offset_min, day_start)
        .iter()
        .filter(|a| a.task)
    {
        total += 1;
        *per_day.entry(a.day.clone()).or_insert(0) += 1;
    }
    (total, per_day)
}

/// Stardust per completion record that still counts, as `(record id, points)`.
pub fn points(rows: &[Row]) -> Vec<(Uuid, u32)> {
    let by_id: HashMap<Uuid, &Row> = rows.iter().map(|r| (r.id(), r)).collect();
    rows.iter()
        .filter_map(|r| match r {
            Row::Completion(c) => Some(c),
            _ => None,
        })
        .filter_map(|c| {
            let p = match by_id.get(&c.item_id) {
                Some(Row::Task(t)) if t.recurrence.is_some() || t.completed_at.is_some() => {
                    task_points(t.priority)
                }
                Some(Row::Goal(g)) if g.completed_at.is_some() => {
                    if matches!(by_id.get(&g.parent_id), Some(Row::Goal(_))) {
                        SUB_GOAL
                    } else {
                        GOAL
                    }
                }
                Some(Row::Milestone(m)) if m.done => MILESTONE,
                Some(_) => return None,
                None => c.points.unwrap_or(DEFAULT_TASK),
            };
            Some((c.id, p))
        })
        .collect()
}

/// All Stardust ever earned. It keeps counting past the top Level.
pub fn total(rows: &[Row], utc_offset_min: i32, day_start: &str) -> u64 {
    awards(rows, utc_offset_min, day_start)
        .iter()
        .map(|a| a.points as u64)
        .sum()
}

/// Stardust per day for the `days` days ending at `today`, oldest first.
pub fn day_totals(
    rows: &[Row],
    utc_offset_min: i32,
    day_start: &str,
    today: &str,
    days: u32,
) -> Vec<DayPoints> {
    let Some(end) = parse_ymd(today) else {
        return vec![];
    };
    let mut by_day: HashMap<String, u64> = HashMap::new();
    for a in awards(rows, utc_offset_min, day_start) {
        *by_day.entry(a.day).or_default() += a.points as u64;
    }
    (0..days as i64)
        .rev()
        .map(|i| {
            let day = fmt_date(end - i);
            let points = by_day.get(&day).copied().unwrap_or(0);
            DayPoints { day, points }
        })
        .collect()
}

/// Today's and this week's (Monday on) completed Tasks against the targets, and the target streak.
/// A day off, or today while on vacation, neither breaks nor extends the streak; a working day
/// below the daily target ends it. Today itself only counts once it is hit.
pub fn target_status(
    rows: &[Row],
    settings: Option<&Settings>,
    utc_offset_min: i32,
    day_start: &str,
    today: &str,
) -> Targets {
    let daily = settings
        .and_then(|s| s.target_daily)
        .unwrap_or(DEFAULT_DAILY);
    let weekly = settings
        .and_then(|s| s.target_weekly)
        .unwrap_or(DEFAULT_WEEKLY);
    let days_off: Vec<u8> = settings
        .and_then(|s| s.days_off.clone())
        .unwrap_or_default();
    let vacation = settings.and_then(|s| s.vacation).unwrap_or(false);
    let vacation_from = settings
        .and_then(|s| s.vacation_from.as_deref())
        .and_then(parse_ymd);
    let ranges: Vec<(i64, i64)> = settings
        .and_then(|s| s.vacations.as_ref())
        .map(|v| {
            v.iter()
                .filter_map(|[a, b]| Some((parse_ymd(a)?, parse_ymd(b)?)))
                .collect()
        })
        .unwrap_or_default();
    let mut per_day: HashMap<i64, u32> = HashMap::new();
    for a in awards(rows, utc_offset_min, day_start) {
        if a.task
            && let Some(d) = parse_ymd(&a.day)
        {
            *per_day.entry(d).or_default() += 1;
        }
    }
    let mut targets = Targets {
        today_done: 0,
        week_done: 0,
        daily,
        weekly,
        streak: 0,
        vacation,
        days_off: days_off.clone(),
    };
    let Some(t) = parse_ymd(today) else {
        return targets;
    };
    targets.today_done = per_day.get(&t).copied().unwrap_or(0);
    let monday = t - weekday(t);
    targets.week_done = (monday..=t)
        .map(|d| per_day.get(&d).copied().unwrap_or(0))
        .sum();
    let Some(first) = per_day.keys().min().copied() else {
        return targets;
    };
    let mut d = t;
    while d >= first {
        let done = per_day.get(&d).copied().unwrap_or(0);
        let off = days_off.contains(&(weekday(d) as u8));
        let away = ranges.iter().any(|&(a, b)| (a..=b).contains(&d))
            || (vacation && vacation_from.is_some_and(|f| d >= f));
        if done >= daily && !off {
            targets.streak += 1;
        } else if off || away || (d == t && (vacation || done < daily)) {
            // Skipped: off, on vacation, or today not yet hit.
        } else {
            break;
        }
        d -= 1;
    }
    targets
}

/// Everything the Profile screen shows.
#[derive(Serialize)]
pub struct Summary {
    pub total: u64,
    pub level: Level,
    pub history: Vec<DayPoints>,
    pub targets: Targets,
}

pub fn summary(rows: &[Row], today: &str, utc_offset_min: i32, day_start: &str) -> Summary {
    let settings = rows.iter().find_map(|r| match r {
        Row::Settings(s) => Some(s),
        _ => None,
    });
    let total = total(rows, utc_offset_min, day_start);
    Summary {
        total,
        level: level(total),
        history: day_totals(rows, utc_offset_min, day_start, today, 30),
        targets: target_status(rows, settings, utc_offset_min, day_start, today),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{Goal, Milestone, Progress};
    use crate::routine::Checkin;
    use crate::{Completion, SETTINGS, Task};

    const DAY: u64 = 86_400_000;
    // 2026-03-10, a Tuesday, 12:00 UTC.
    const NOON: u64 = 1_773_144_000_000;

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn task(n: u128, priority: u8, done: bool) -> Task {
        Task {
            id: uid(n),
            parent_id: uid(1),
            title: format!("T{n}"),
            completed_at: done.then_some(NOON),
            order: String::new(),
            description: String::new(),
            due: None,
            deadline: None,
            duration_min: None,
            priority,
            label_ids: vec![],
            recurrence: None,
            goal_id: None,
            created_at: Some(n as u64),
            trashed_at: None,
            pinned_on: None,
            not_today_on: None,
            links: vec![],
            reminders: vec![],
            blobs: vec![],
        }
    }

    fn goal(n: u128, parent: u128, done: bool) -> Goal {
        Goal {
            id: uid(n),
            parent_id: uid(parent),
            name: "G".into(),
            done_condition: String::new(),
            progress: Progress::Manual,
            start: None,
            target_date: None,
            completed_at: done.then_some(NOON),
            order: String::new(),
            touched_at: None,
            trashed_at: None,
            archived_at: None,
        }
    }

    fn comp(n: u128, item: u128, at: u64, points: Option<u32>) -> Row {
        Row::Completion(Completion {
            id: uid(n),
            item_id: uid(item),
            at,
            points,
            source_id: None,
            completion_hlc: None,
            expected_due: None,
            restore_due: None,
        })
    }

    fn settings(f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = Settings {
            id: SETTINGS,
            display_name: String::new(),
            avatar: String::new(),
            theme: String::new(),
            accent: String::new(),
            start_view: String::new(),
            timezone: String::new(),
            day_start: String::new(),
            next_deadline_days: None,
            next_due_days: None,
            next_undated_p1: None,
            default_reminder: None,
            target_daily: None,
            target_weekly: None,
            days_off: None,
            vacation: None,
            week_start: None,
            dashboard: None,
            vacation_from: None,
            vacations: None,
            gamification: None,
        };
        f(&mut s);
        s
    }

    fn total_of(rows: &[Row]) -> u64 {
        total(rows, 0, "")
    }

    #[test]
    fn task_points_by_priority() {
        let rows = vec![
            Row::Task(task(10, 1, true)),
            Row::Task(task(11, 2, true)),
            Row::Task(task(12, 3, true)),
            Row::Task(task(13, 4, true)),
            comp(20, 10, NOON, None),
            comp(21, 11, NOON, None),
            comp(22, 12, NOON, None),
            comp(23, 13, NOON, None),
        ];
        assert_eq!(total_of(&rows), 80 + 40 + 25 + 15);
        assert_eq!(points(&rows).iter().map(|p| p.1).sum::<u32>(), 160);
    }

    #[test]
    fn goal_subgoal_and_milestone() {
        let rows = vec![
            Row::Goal(goal(10, 1, true)),
            Row::Goal(goal(11, 10, true)),
            Row::Milestone(Milestone {
                id: uid(12),
                goal_id: uid(10),
                title: "M".into(),
                date: None,
                done: true,
                order: String::new(),
                trashed_at: None,
                pinned_on: None,
                not_today_on: None,
            }),
            comp(20, 10, NOON, None),
            comp(21, 11, NOON, None),
            comp(22, 12, NOON, None),
        ];
        assert_eq!(total_of(&rows), 500 + 250 + 250);
    }

    #[test]
    fn checkin_done_counts_skipped_does_not() {
        let k = |n: u128, state, cleared| {
            Row::Checkin(Checkin {
                id: uid(n),
                routine_id: uid(1),
                date: "2026-03-10".into(),
                state,
                cleared,
            })
        };
        let rows = vec![
            k(10, CheckState::Done, false),
            k(11, CheckState::Skipped, false),
            k(12, CheckState::Done, true),
        ];
        assert_eq!(total_of(&rows), 20);
    }

    #[test]
    fn uncomplete_removes_and_gone_item_keeps() {
        let open = vec![Row::Task(task(10, 1, false)), comp(20, 10, NOON, None)];
        assert_eq!(total_of(&open), 0);
        // Purged: only the record remains, with the points it logged.
        let gone = vec![comp(20, 10, NOON, Some(80)), comp(21, 11, NOON, None)];
        assert_eq!(total_of(&gone), 80 + 15);
    }

    #[test]
    fn recurring_occurrences_each_count() {
        let mut t = task(10, 2, false);
        t.recurrence = Some(crate::recur::Recurrence {
            rule: "day".into(),
            from_completion: false,
        });
        let rows = vec![
            Row::Task(t),
            comp(20, 10, NOON, None),
            comp(21, 10, NOON + DAY, None),
        ];
        assert_eq!(total_of(&rows), 80);
    }

    #[test]
    fn removing_an_undone_recurring_task_completion_reverses_its_stardust() {
        let mut t = task(2, 4, false);
        t.recurrence = Some(crate::recur::Recurrence {
            rule: "week".into(),
            from_completion: false,
        });
        let completion = comp(3, 2, NOON, Some(100));
        let rows = vec![Row::Task(t), completion.clone()];
        assert_eq!(total(&rows, 0, "04:00"), task_points(4) as u64);
        let rows = rows
            .into_iter()
            .filter(|r| r.id() != completion.id())
            .collect::<Vec<_>>();
        assert_eq!(total(&rows, 0, "04:00"), 0);
    }

    #[test]
    fn level_boundaries() {
        assert_eq!(level(0).n, 1);
        assert_eq!(level(1499).n, 1);
        assert_eq!(level(1499).next, Some(1500));
        let l = level(1500);
        assert_eq!((l.n, l.name, l.floor), (2, "Meteor", 1500));
        assert_eq!(level(229_399).n, 12);
        let u = level(229_400);
        assert_eq!((u.n, u.name, u.next), (13, "Universe", None));
        assert_eq!(level(300_000).n, 13);
    }

    fn day_rows(counts: &[(i64, u32)]) -> Vec<Row> {
        // `counts` are (days before 2026-03-10, tasks done that day).
        let mut rows = vec![];
        let mut n = 100;
        for (ago, k) in counts {
            for _ in 0..*k {
                n += 1;
                rows.push(Row::Task(task(n, 4, true)));
                rows.push(comp(n + 1000, n, NOON - *ago as u64 * DAY, None));
            }
        }
        rows
    }

    #[test]
    fn streak_misses_end_it() {
        let mut rows = day_rows(&[(0, 2), (1, 2), (3, 2), (4, 2)]);
        rows.push(Row::Settings(settings(|s| s.target_daily = Some(2))));
        let s = target_status(
            &rows,
            rows.iter().find_map(|r| match r {
                Row::Settings(s) => Some(s),
                _ => None,
            }),
            0,
            "",
            "2026-03-10",
        );
        // Today and yesterday hit; the 8th was missed.
        assert_eq!((s.streak, s.today_done, s.daily), (2, 2, 2));
        // Monday the 9th + Tuesday the 10th.
        assert_eq!(s.week_done, 4);
    }

    #[test]
    fn streak_skips_days_off_and_vacation() {
        let st = settings(|s| {
            s.target_daily = Some(1);
            // The 8th (Sunday) is off.
            s.days_off = Some(vec![6]);
        });
        // Done on the 10th, 9th, 7th, 6th; nothing on the 8th.
        let rows = day_rows(&[(0, 1), (1, 1), (3, 1), (4, 1)]);
        let s = target_status(&rows, Some(&st), 0, "", "2026-03-10");
        assert_eq!(s.streak, 4);
        let none = target_status(&rows, None, 0, "", "2026-03-10");
        assert_eq!(none.daily, 5);
        assert_eq!(none.streak, 0);
        // Vacation: today is neither a miss nor counted.
        let vac = settings(|s| {
            s.target_daily = Some(1);
            s.vacation = Some(true);
        });
        let rows = day_rows(&[(1, 1), (2, 1)]);
        let s = target_status(&rows, Some(&vac), 0, "", "2026-03-10");
        assert_eq!(s.streak, 2);
        // Without it, today unhit is simply not yet a miss either.
        let s = target_status(&rows, None, 0, "", "2026-03-10");
        assert_eq!(s.streak, 0);
        // A finished vacation (the 7th to the 9th) does not break the streak.
        let past = settings(|s| {
            s.target_daily = Some(1);
            s.vacations = Some(vec![["2026-03-07".into(), "2026-03-09".into()]]);
        });
        let rows = day_rows(&[(0, 1), (4, 1), (5, 1)]);
        let s = target_status(&rows, Some(&past), 0, "", "2026-03-10");
        assert_eq!(s.streak, 3);
        let no_range = settings(|s| s.target_daily = Some(1));
        let s = target_status(&rows, Some(&no_range), 0, "", "2026-03-10");
        assert_eq!(s.streak, 1);
        // One in progress skips days from its start on.
        let on = settings(|s| {
            s.target_daily = Some(1);
            s.vacation = Some(true);
            s.vacation_from = Some("2026-03-07".into());
        });
        let s = target_status(&rows, Some(&on), 0, "", "2026-03-10");
        assert_eq!(s.streak, 3);
    }

    #[test]
    fn day_start_shifts_the_day() {
        // 01:00 UTC on the 11th with a 03:00 day start still belongs to the 10th.
        let at = NOON + 13 * 3_600_000;
        let rows = vec![Row::Task(task(10, 4, true)), comp(20, 10, at, None)];
        let h = day_totals(&rows, 0, "03:00", "2026-03-11", 2);
        assert_eq!(
            h,
            vec![
                DayPoints {
                    day: "2026-03-10".into(),
                    points: 15
                },
                DayPoints {
                    day: "2026-03-11".into(),
                    points: 0
                },
            ]
        );
        let h = day_totals(&rows, 0, "", "2026-03-11", 2);
        assert_eq!(h[1].points, 15);
    }

    #[test]
    fn gamification_off_still_computes_awards() {
        let rows = vec![
            Row::Settings(settings(|s| s.gamification = Some(false))),
            Row::Award(crate::badge::Award {
                id: uid(30),
                badge: "tasks-10".into(),
                key: "tasks-10".into(),
                earned_at: NOON,
                points: 50,
            }),
        ];
        let s = summary(&rows, "2026-03-10", 0, "");
        assert_eq!(s.total, 50);
        assert_eq!(s.level.n, 1);
    }

    #[test]
    fn history_is_thirty_days_oldest_first() {
        let s = summary(&[], "2026-03-10", 0, "");
        assert_eq!(s.history.len(), 30);
        assert_eq!(s.history[29].day, "2026-03-10");
        assert_eq!(s.total, 0);
        assert_eq!(s.level.n, 1);
    }
}
