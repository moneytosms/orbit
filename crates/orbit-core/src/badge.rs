//! Badges and Patches. The catalog is data; the server decides what is earned and writes one
//! synced `Award` row per grant. Awards are never revoked, and their Stardust counts in `stardust`.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Builder;

use crate::routine::{CheckState, Checkin, stats};
use crate::{Row, Uuid, roadmap, stardust};

/// Inbox Tasks that must be open before completing one that empties the Inbox counts.
pub const INBOX_MIN: usize = 5;

/// Award `badge` value of a Patch.
pub const PATCH: &str = "patch";

/// One earned Badge or Patch. `key` names the grant: the catalog id, `id@day` for a repeat, or
/// `patch:<roadmap>`. Rows are written once per key and never removed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Award {
    pub id: Uuid,
    pub badge: String,
    pub key: String,
    pub earned_at: u64,
    pub points: u32,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Meteorite,
    Comet,
    Planet,
    Star,
    BlackHole,
}

impl Tier {
    /// Stardust per award of this tier.
    pub fn points(self) -> u32 {
        match self {
            Tier::Meteorite => 50,
            Tier::Comet => 250,
            Tier::Planet => 1000,
            Tier::Star => 5000,
            Tier::BlackHole => 20000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Tasks,
    Checkins,
    Streak,
    BusyDay,
    Milestones,
    Goals,
    Patches,
    Stardust,
    InboxZero,
}

pub struct Badge {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: Tier,
    pub metric: Metric,
    pub need: u64,
    /// Awarded again each time the condition recurs (shown ×n).
    pub repeatable: bool,
    /// Text shown on a locked outline.
    pub condition: &'static str,
}

const fn badge(
    id: &'static str,
    name: &'static str,
    tier: Tier,
    metric: Metric,
    need: u64,
    repeatable: bool,
    condition: &'static str,
) -> Badge {
    Badge {
        id,
        name,
        tier,
        metric,
        need,
        repeatable,
        condition,
    }
}

use Metric::*;
use Tier::*;

/// The 24 Badges in core. Ids are stable: they are stored in award rows.
pub const CATALOG: [Badge; 24] = [
    badge(
        "tasks-10",
        "Ten Done",
        Meteorite,
        Tasks,
        10,
        false,
        "Complete 10 tasks",
    ),
    badge(
        "tasks-100",
        "Hundred Done",
        Comet,
        Tasks,
        100,
        false,
        "Complete 100 tasks",
    ),
    badge(
        "tasks-1000",
        "Thousand Done",
        Planet,
        Tasks,
        1000,
        false,
        "Complete 1,000 tasks",
    ),
    badge(
        "tasks-5000",
        "Five Thousand Done",
        Star,
        Tasks,
        5000,
        false,
        "Complete 5,000 tasks",
    ),
    badge(
        "tasks-20000",
        "Twenty Thousand Done",
        BlackHole,
        Tasks,
        20000,
        false,
        "Complete 20,000 tasks",
    ),
    badge(
        "checkins-10",
        "Ten Check-ins",
        Meteorite,
        Checkins,
        10,
        false,
        "Check in 10 times",
    ),
    badge(
        "checkins-100",
        "Hundred Check-ins",
        Comet,
        Checkins,
        100,
        false,
        "Check in 100 times",
    ),
    badge(
        "checkins-500",
        "Five Hundred Check-ins",
        Planet,
        Checkins,
        500,
        false,
        "Check in 500 times",
    ),
    badge(
        "streak-7",
        "Week Streak",
        Meteorite,
        Streak,
        7,
        false,
        "Keep a Check-in streak of 7 days",
    ),
    badge(
        "streak-30",
        "Month Streak",
        Comet,
        Streak,
        30,
        false,
        "Keep a Check-in streak of 30 days",
    ),
    badge(
        "streak-100",
        "Hundred-Day Streak",
        Planet,
        Streak,
        100,
        false,
        "Keep a Check-in streak of 100 days",
    ),
    badge(
        "streak-365",
        "Year Streak",
        Star,
        Streak,
        365,
        false,
        "Keep a Check-in streak of 365 days",
    ),
    badge(
        "busy-10",
        "Busy Day",
        Meteorite,
        BusyDay,
        10,
        true,
        "Complete 10 tasks in one day",
    ),
    badge(
        "busy-25",
        "Packed Day",
        Comet,
        BusyDay,
        25,
        true,
        "Complete 25 tasks in one day",
    ),
    badge(
        "milestones-1",
        "First Milestone",
        Meteorite,
        Milestones,
        1,
        false,
        "Complete 1 milestone",
    ),
    badge(
        "milestones-10",
        "Ten Milestones",
        Comet,
        Milestones,
        10,
        false,
        "Complete 10 milestones",
    ),
    badge(
        "milestones-50",
        "Fifty Milestones",
        Planet,
        Milestones,
        50,
        false,
        "Complete 50 milestones",
    ),
    badge(
        "goals-1",
        "First Goal",
        Comet,
        Goals,
        1,
        false,
        "Complete 1 goal",
    ),
    badge(
        "goals-5",
        "Five Goals",
        Planet,
        Goals,
        5,
        false,
        "Complete 5 goals",
    ),
    badge(
        "goals-25",
        "Twenty-Five Goals",
        Star,
        Goals,
        25,
        false,
        "Complete 25 goals",
    ),
    badge(
        "patches-1",
        "First Patch",
        Planet,
        Patches,
        1,
        false,
        "Earn 1 Patch",
    ),
    badge(
        "patches-5",
        "Five Patches",
        Star,
        Patches,
        5,
        false,
        "Earn 5 Patches",
    ),
    badge(
        "universe",
        "Universe",
        BlackHole,
        Stardust,
        229400,
        false,
        "Reach 229,400 Stardust",
    ),
    badge(
        "inbox-zero",
        "Inbox Zero",
        Meteorite,
        InboxZero,
        INBOX_MIN as u64,
        true,
        "Empty the Inbox after 5 or more open Tasks",
    ),
];

/// Stardust for a Patch on a Roadmap with `phases` Phases: 500 + 100 per Phase, at most 2,000.
pub fn patch_points(phases: usize) -> u32 {
    (500 + 100 * phases as u32).min(2000)
}

/// Deterministic id for an award key, so the same grant always gets the same row id.
fn award_id(key: &str) -> Uuid {
    // FNV-1a, 128 bit.
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for b in key.bytes() {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    Builder::from_random_bytes(h.to_be_bytes()).into_uuid()
}

/// Current counts behind every Badge metric, so awards and shelf progress agree.
struct Counts {
    tasks: u64,
    checkins: u64,
    streak: u64,
    milestones: u64,
    goals: u64,
    patches: u64,
    stardust: u64,
    /// Tasks completed per local day.
    per_day: std::collections::HashMap<String, u32>,
    today: String,
}

impl Counts {
    fn new(rows: &[Row], utc_offset_min: i32, day_start: &str, today: &str) -> Counts {
        let (tasks, per_day) = stardust::task_counts(rows, utc_offset_min, day_start);
        let n = |f: &dyn Fn(&Row) -> bool| rows.iter().filter(|r| f(r)).count() as u64;
        let streak = rows
            .iter()
            .filter_map(|r| match r {
                Row::Routine(rt) if rt.trashed_at.is_none() => Some(rt),
                _ => None,
            })
            .map(|rt| {
                let cks: Vec<Checkin> = rows
                    .iter()
                    .filter_map(|x| match x {
                        Row::Checkin(c) if c.routine_id == rt.id => Some(c.clone()),
                        _ => None,
                    })
                    .collect();
                stats(rt, &cks, today).best_streak as u64
            })
            .max()
            .unwrap_or(0);
        Counts {
            tasks: tasks as u64,
            checkins: n(
                &|r| matches!(r, Row::Checkin(c) if c.state == CheckState::Done && !c.cleared),
            ),
            streak,
            milestones: n(&|r| matches!(r, Row::Milestone(m) if m.done)),
            goals: n(&|r| matches!(r, Row::Goal(g) if g.completed_at.is_some())),
            patches: n(&|r| matches!(r, Row::Award(a) if a.badge == PATCH)),
            stardust: stardust::total(rows, utc_offset_min, day_start),
            per_day,
            today: today.to_string(),
        }
    }

    /// Value of a counting metric; `BusyDay` is today's completions, `InboxZero` is not counted here.
    fn of(&self, m: Metric) -> u64 {
        match m {
            Tasks => self.tasks,
            Checkins => self.checkins,
            Streak => self.streak,
            Milestones => self.milestones,
            Goals => self.goals,
            Patches => self.patches,
            Stardust => self.stardust,
            BusyDay => self.per_day.get(&self.today).copied().unwrap_or(0) as u64,
            InboxZero => 0,
        }
    }
}

/// Inbox Tasks completed since the last award of Badge `id`.
fn inbox_done(rows: &[Row], id: &str) -> u64 {
    let last = rows
        .iter()
        .filter_map(|r| match r {
            Row::Award(a) if a.badge == id => Some(a.earned_at),
            _ => None,
        })
        .max();
    rows.iter()
        .filter(|r| {
            matches!(r, Row::Task(t) if t.parent_id == crate::INBOX
                && t.completed_at.is_some_and(|c| last.is_none_or(|l| c > l)))
        })
        .count() as u64
}

/// Awards newly earned by `rows` that have no row yet. `today` is `YYYY-MM-DD` for Check-in
/// streaks; `emptied` is the command id of a Task completion that left no open Inbox Tasks.
pub fn earned(
    rows: &[Row],
    now: u64,
    utc_offset_min: i32,
    day_start: &str,
    today: &str,
    emptied: Option<Uuid>,
) -> Vec<Award> {
    let have: HashSet<String> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Award(a) => Some(a.key.clone()),
            _ => None,
        })
        .collect();
    let mut out: Vec<Award> = Vec::new();
    let grant = |out: &mut Vec<Award>, badge: &str, key: String, points: u32| {
        if have.contains(&key) || out.iter().any(|a| a.key == key) {
            return;
        }
        out.push(Award {
            id: award_id(&key),
            badge: badge.into(),
            key,
            earned_at: now,
            points,
        });
    };

    // Patches first: they count toward the Patch Badges below.
    for r in rows {
        let Row::Roadmap(rm) = r else { continue };
        if rm.trashed_at.is_some() || rm.archived_at.is_some() {
            continue;
        }
        let phases: Vec<Uuid> = rows
            .iter()
            .filter_map(|p| match p {
                Row::Phase(p)
                    if p.roadmap_id == rm.id
                        && p.trashed_at.is_none()
                        && p.archived_at.is_none() =>
                {
                    Some(p.id)
                }
                _ => None,
            })
            .collect();
        if phases.len() >= 2
            && phases
                .iter()
                .all(|&p| roadmap::phase_progress(rows, p) >= 1.0)
        {
            let points = patch_points(phases.len());
            grant(&mut out, PATCH, format!("patch:{}", rm.id), points);
        }
    }

    let c = Counts::new(rows, utc_offset_min, day_start, today);
    for b in CATALOG.iter() {
        match b.metric {
            BusyDay => {
                for (day, n) in &c.per_day {
                    if *n as u64 >= b.need {
                        grant(&mut out, b.id, format!("{}@{day}", b.id), b.tier.points());
                    }
                }
            }
            InboxZero => {
                if let Some(cmd) = emptied {
                    let done = inbox_done(rows, b.id);
                    if done >= b.need {
                        grant(&mut out, b.id, format!("{}@{cmd}", b.id), b.tier.points());
                    }
                }
            }
            metric => {
                let have_n = match metric {
                    Patches => c.patches + out.iter().filter(|a| a.badge == PATCH).count() as u64,
                    Stardust => c.stardust + out.iter().map(|a| a.points as u64).sum::<u64>(),
                    m => c.of(m),
                };
                if have_n >= b.need {
                    grant(&mut out, b.id, b.id.to_string(), b.tier.points());
                }
            }
        }
    }
    out
}

impl Tier {
    /// Display name of this tier.
    pub fn name(self) -> &'static str {
        match self {
            Tier::Meteorite => "Meteorite",
            Tier::Comet => "Comet",
            Tier::Planet => "Planet",
            Tier::Star => "Star",
            Tier::BlackHole => "Black Hole",
        }
    }
}

/// One catalog Badge as the Profile shelf shows it. `count` is how many times it was earned.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ShelfItem {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: &'static str,
    pub points: u32,
    pub repeatable: bool,
    pub condition: &'static str,
    pub count: u32,
    /// Progress toward `need`, capped at it (today's Tasks for repeatable busy-day Badges).
    pub progress: u64,
    pub need: u64,
}

/// The Badge shelf over `rows`: every catalog Badge with its count, and the Patches earned.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Shelf {
    pub badges: Vec<ShelfItem>,
    pub patches: u32,
}

pub fn shelf(rows: &[Row], utc_offset_min: i32, day_start: &str, today: &str) -> Shelf {
    let c = Counts::new(rows, utc_offset_min, day_start, today);
    let badges = CATALOG
        .iter()
        .map(|b| ShelfItem {
            id: b.id,
            name: b.name,
            tier: b.tier.name(),
            points: b.tier.points(),
            repeatable: b.repeatable,
            condition: b.condition,
            count: rows
                .iter()
                .filter(|r| matches!(r, Row::Award(a) if a.badge == b.id))
                .count() as u32,
            progress: match b.metric {
                InboxZero => inbox_done(rows, b.id),
                m => c.of(m),
            }
            .min(b.need),
            need: b.need,
        })
        .collect();
    let patches = rows
        .iter()
        .filter(|r| matches!(r, Row::Award(a) if a.badge == PATCH))
        .count() as u32;
    Shelf { badges, patches }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_24_unique_badges_with_the_five_tiers() {
        assert_eq!(CATALOG.len(), 24);
        let ids: HashSet<&str> = CATALOG.iter().map(|b| b.id).collect();
        assert_eq!(ids.len(), 24);
        assert_eq!(Meteorite.points(), 50);
        assert_eq!(BlackHole.points(), 20000);
        assert_eq!(CATALOG.iter().filter(|b| b.repeatable).count(), 3);
    }

    #[test]
    fn shelf_counts_each_award_and_lists_every_badge() {
        let rows = vec![
            Row::Award(Award {
                id: award_id("busy-10@2026-11-20"),
                badge: "busy-10".into(),
                key: "busy-10@2026-11-20".into(),
                earned_at: 1,
                points: 50,
            }),
            Row::Award(Award {
                id: award_id("busy-10@2026-11-21"),
                badge: "busy-10".into(),
                key: "busy-10@2026-11-21".into(),
                earned_at: 2,
                points: 50,
            }),
        ];
        let shelf = shelf(&rows, 0, "00:00", "2026-11-24");
        assert_eq!(shelf.badges.len(), 24);
        assert_eq!(shelf.patches, 0);
        let busy = shelf.badges.iter().find(|b| b.id == "busy-10").unwrap();
        assert_eq!((busy.count, busy.points, busy.tier), (2, 50, "Meteorite"));
        assert_eq!(shelf.badges.iter().filter(|b| b.count > 0).count(), 1);
    }

    #[test]
    fn patch_is_500_plus_100_per_phase_capped_at_2000() {
        assert_eq!(patch_points(2), 700);
        assert_eq!(patch_points(5), 1000);
        assert_eq!(patch_points(20), 2000);
    }

    #[test]
    fn award_ids_are_stable_per_key() {
        assert_eq!(
            award_id("busy-10@2026-11-20"),
            award_id("busy-10@2026-11-20")
        );
        assert_ne!(
            award_id("busy-10@2026-11-20"),
            award_id("busy-10@2026-11-21")
        );
    }

    const AT: u64 = 1_700_000_000_000; // 2023-11-14 22:13 UTC
    const TODAY: &str = "2023-11-14";

    fn rows(v: serde_json::Value) -> Vec<Row> {
        serde_json::from_value(v).unwrap()
    }

    fn uid(n: u128) -> String {
        Uuid::from_u128(n).to_string()
    }

    fn progress(rows: &[Row], id: &str) -> (u64, u64) {
        let b = shelf(rows, 0, "00:00", TODAY);
        let b = b.badges.into_iter().find(|b| b.id == id).unwrap();
        (b.progress, b.need)
    }

    fn done_tasks(n: u128) -> Vec<Row> {
        rows(serde_json::Value::Array(
            (0..n)
                .map(|i| serde_json::json!({"kind":"completion","id":uid(100 + i),"item_id":uid(1000 + i),"at":AT,"points":15}))
                .collect(),
        ))
    }

    #[test]
    fn progress_tasks_and_busy_day_count_completions_and_cap_at_need() {
        assert_eq!(progress(&done_tasks(6), "tasks-10"), (6, 10));
        assert_eq!(progress(&done_tasks(6), "busy-10"), (6, 10));
        assert_eq!(progress(&done_tasks(12), "tasks-10"), (10, 10));
        // Busy day is today's tasks: completions on another day do not count.
        assert_eq!(progress(&done_tasks(6), "busy-10").0, 6);
        let b = shelf(&done_tasks(6), 0, "00:00", "2023-11-15");
        assert_eq!(
            b.badges
                .iter()
                .find(|b| b.id == "busy-10")
                .unwrap()
                .progress,
            0
        );
    }

    #[test]
    fn progress_checkins_and_streak() {
        let mut v = vec![
            serde_json::json!({"kind":"routine","id":uid(1),"parent_id":uid(2),"name":"R",
            "schedule":{"kind":"daily"},"start":"2023-11-01"}),
        ];
        for d in 12..=14 {
            v.push(
                serde_json::json!({"kind":"checkin","id":uid(10 + d),"routine_id":uid(1),
                "date":format!("2023-11-{d}"),"state":"done"}),
            );
        }
        let r = rows(serde_json::Value::Array(v));
        assert_eq!(progress(&r, "checkins-10"), (3, 10));
        assert_eq!(progress(&r, "streak-7"), (3, 7));
    }

    #[test]
    fn progress_milestones_goals_patches() {
        let r = rows(serde_json::json!([
            {"kind":"milestone","id":uid(1),"goal_id":uid(9),"title":"M","done":true},
            {"kind":"milestone","id":uid(2),"goal_id":uid(9),"title":"M","done":false},
            {"kind":"goal","id":uid(3),"parent_id":uid(8),"name":"G","progress":{"kind":"count"},"completed_at":AT},
            {"kind":"award","id":uid(4),"badge":"patch","key":"patch:x","earned_at":AT,"points":700},
        ]));
        assert_eq!(progress(&r, "milestones-10"), (1, 10));
        assert_eq!(progress(&r, "goals-5"), (1, 5));
        assert_eq!(progress(&r, "patches-5"), (1, 5));
        assert_eq!(progress(&[], "patches-5"), (0, 5));
    }

    #[test]
    fn progress_stardust_and_inbox_zero() {
        let (p, need) = progress(&done_tasks(2), "universe");
        assert_eq!((p, need), (30, 229400));
        assert_eq!(progress(&[], "inbox-zero"), (0, INBOX_MIN as u64));
    }
}
