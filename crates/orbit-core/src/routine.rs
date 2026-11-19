//! Routines, their Check-ins and the pure streak maths.
//!
//! A Check-in stores only `done` or `skipped`; a miss is derived from a scheduled day that
//! passed without one. "Today" is always the caller's Day start date (see `local_day`).
use crate::parse::{fmt_date, parse_ymd};
use crate::{Command, ErrorCode, Op, Row, Uuid, is_date, is_time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How often a Routine is due.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Schedule {
    Daily,
    /// Monday to Friday.
    Weekdays,
    /// Every `n` days from the start.
    Every {
        n: u32,
    },
    /// `n` Check-ins in each Monday to Sunday week.
    PerWeek {
        n: u32,
    },
}

/// A span of paused days: `from` up to, not including, `to` (open while `to` is `None`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Pause {
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Routine {
    pub id: Uuid,
    /// An Area or a Project.
    pub parent_id: Uuid,
    pub name: String,
    pub schedule: Schedule,
    /// `YYYY-MM-DD`, the creator's Day start date.
    pub start: String,
    /// `None` is ongoing.
    #[serde(default)]
    pub length_days: Option<u32>,
    /// `HH:MM`; `None` is anytime.
    #[serde(default)]
    pub time: Option<String>,
    /// A Target Goal each done Check-in adds 1 to.
    #[serde(default)]
    pub goal_id: Option<Uuid>,
    #[serde(default)]
    pub pauses: Vec<Pause>,
    #[serde(default)]
    pub order: String,
    #[serde(default)]
    pub created_at: Option<u64>,
    /// Unix ms when it went to the Trash.
    #[serde(default)]
    pub trashed_at: Option<u64>,
    /// Unix ms when it was archived.
    #[serde(default)]
    pub archived_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckState {
    Done,
    Skipped,
}

/// One day's tick of a Routine. Its id is `checkin_id(routine_id, date)`, so two devices
/// ticking the same day write the same row. Clearing sets `cleared` rather than deleting, so the
/// clear syncs as an ordinary last-writer-wins edit and a later tick brings it back.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Checkin {
    pub id: Uuid,
    pub routine_id: Uuid,
    /// `YYYY-MM-DD`.
    pub date: String,
    pub state: CheckState,
    #[serde(default)]
    pub cleared: bool,
}

/// The same Routine and day always give the same Check-in id.
pub fn checkin_id(routine_id: Uuid, date: &str) -> Uuid {
    // FNV-1a over the date, spread across 128 bits, mixed into the routine id.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in date.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mix = ((h as u128) << 64) | h.rotate_left(32) as u128;
    let id = routine_id.as_u128() ^ mix;
    uuid::Builder::from_random_bytes(id.to_be_bytes()).into_uuid()
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DayState {
    Done,
    Skipped,
    Missed,
    Paused,
    /// Not scheduled, outside the Routine's span, or not yet due.
    Off,
    /// Due today, not ticked yet.
    Pending,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stats {
    pub current_streak: u32,
    pub best_streak: u32,
    /// Done over done plus missed, skips excluded; `None` before anything is resolved.
    pub completion_rate: Option<f64>,
    /// Every day from the start (or the first Check-in) to today.
    pub days: Vec<(String, DayState)>,
}

fn start_day(r: &Routine) -> i64 {
    parse_ymd(&r.start).unwrap_or(0)
}

/// The last day of a Routine with a length.
fn end_day(r: &Routine) -> Option<i64> {
    r.length_days.map(|n| start_day(r) + n.max(1) as i64 - 1)
}

/// Past its length: it moves to Archived by itself and misses stop.
pub fn is_ended(r: &Routine, today: &str) -> bool {
    match (end_day(r), parse_ymd(today)) {
        (Some(end), Some(t)) => t > end,
        _ => false,
    }
}

/// Whether `today` falls inside a pause.
pub fn is_paused(r: &Routine, today: &str) -> bool {
    let Some(t) = parse_ymd(today) else {
        return false;
    };
    paused_on(r, t)
}

fn paused_on(r: &Routine, day: i64) -> bool {
    r.pauses.iter().any(|p| {
        parse_ymd(&p.from).is_some_and(|f| f <= day)
            && p.to
                .as_deref()
                .is_none_or(|to| parse_ymd(to).is_some_and(|to| day < to))
    })
}

/// Monday is 0.
fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

fn scheduled(r: &Routine, day: i64) -> bool {
    let start = start_day(r);
    if day < start || end_day(r).is_some_and(|e| day > e) {
        return false;
    }
    match r.schedule {
        Schedule::Daily => true,
        Schedule::Weekdays => weekday(day) < 5,
        Schedule::Every { n } => (day - start) % n.max(1) as i64 == 0,
        Schedule::PerWeek { .. } => true,
    }
}

/// Streaks, completion rate and a state for every day up to `today` (`YYYY-MM-DD`).
pub fn stats(r: &Routine, checkins: &[Checkin], today: &str) -> Stats {
    let empty = Stats {
        current_streak: 0,
        best_streak: 0,
        completion_rate: None,
        days: vec![],
    };
    let Some(today) = parse_ymd(today) else {
        return empty;
    };
    let marks: BTreeMap<i64, CheckState> = checkins
        .iter()
        .filter(|c| c.routine_id == r.id && !c.cleared)
        .filter_map(|c| Some((parse_ymd(&c.date)?, c.state)))
        .collect();
    let start = start_day(r);
    let first = marks.keys().next().map_or(start, |m| (*m).min(start));
    let last = end_day(r).map_or(today, |e| e.min(today));
    let per_week = matches!(r.schedule, Schedule::PerWeek { .. });
    let mut days = Vec::new();
    for d in first..=last {
        let state = if let Some(mark) = marks.get(&d) {
            match mark {
                CheckState::Done => DayState::Done,
                CheckState::Skipped => DayState::Skipped,
            }
        } else if !scheduled(r, d) || d > today {
            DayState::Off
        } else if paused_on(r, d) {
            DayState::Paused
        } else if d == today {
            DayState::Pending
        } else if per_week {
            DayState::Off
        } else {
            DayState::Missed
        };
        days.push((fmt_date(d), state));
    }
    // Resolved units, oldest first: `Some(true)` kept the streak going, `None` left it as is.
    let (mut done, mut missed) = (0u32, 0u32);
    let mut units: Vec<Option<bool>> = Vec::new();
    if let Schedule::PerWeek { n } = r.schedule {
        let mut week = first - weekday(first);
        while week <= last {
            let span = week..week + 7;
            let (mut d, mut s, mut avail) = (0u32, 0u32, 0u32);
            for day in span.clone() {
                if day < start || day > last {
                    continue;
                }
                match marks.get(&day) {
                    Some(CheckState::Done) => d += 1,
                    Some(CheckState::Skipped) => s += 1,
                    None if !paused_on(r, day) => avail += 1,
                    None => {}
                }
            }
            let need = n.min(d + s + avail);
            let over = week + 6 < today;
            if need > 0 {
                if d >= need {
                    done += 1;
                    units.push(Some(true));
                } else if d + s >= need {
                    units.push(None);
                } else if over {
                    missed += 1;
                    units.push(Some(false));
                }
            }
            week += 7;
        }
    } else {
        for (_, state) in &days {
            match state {
                DayState::Done => {
                    done += 1;
                    units.push(Some(true));
                }
                DayState::Missed => {
                    missed += 1;
                    units.push(Some(false));
                }
                _ => {}
            }
        }
        // Skips are neutral, so they never need a unit of their own.
    }
    let (mut run, mut best) = (0u32, 0u32);
    for u in units {
        match u {
            Some(true) => {
                run += 1;
                best = best.max(run);
            }
            Some(false) => run = 0,
            None => {}
        }
    }
    let resolved = done + missed;
    Stats {
        current_streak: run,
        best_streak: best,
        completion_rate: (resolved > 0).then(|| done as f64 / resolved as f64),
        days,
    }
}

/// Done Check-ins of the Routines linked to `goal_id`: each adds 1 to a Target Goal.
pub fn done_checkins(goal_id: Uuid, rows: &[Row]) -> usize {
    let linked: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Routine(r) if r.goal_id == Some(goal_id) => Some(r.id),
            _ => None,
        })
        .collect();
    rows.iter()
        .filter(|r| {
            matches!(r, Row::Checkin(c)
                if !c.cleared && c.state == CheckState::Done && linked.contains(&c.routine_id))
        })
        .count()
}

/// The Day-start date at `now_ms`, given the zone's offset from UTC (minutes, east positive)
/// at that instant. Times before `day_start` (`HH:MM`) still belong to the previous day.
pub fn local_day(now_ms: u64, utc_offset_min: i32, day_start: &str) -> String {
    let shift = if is_time(day_start) {
        day_start[0..2].parse::<i64>().unwrap_or(0) * 60
            + day_start[3..5].parse::<i64>().unwrap_or(0)
    } else {
        0
    };
    let minutes = (now_ms / 60_000) as i64 + utc_offset_min as i64 - shift;
    fmt_date(minutes.div_euclid(1440))
}

fn check_fields(r: &Routine) -> Result<(), ErrorCode> {
    let bad_n = match r.schedule {
        Schedule::Every { n } => n == 0,
        Schedule::PerWeek { n } => !(1..=7).contains(&n),
        _ => false,
    };
    if bad_n
        || !is_date(&r.start)
        || parse_ymd(&r.start).is_none()
        || r.length_days == Some(0)
        || r.time.as_deref().is_some_and(|t| !is_time(t))
    {
        return Err(ErrorCode::BadField);
    }
    Ok(())
}

/// Applies the Routine and Check-in commands.
pub fn apply(existing: Option<Row>, cmd: &Command) -> Result<Row, ErrorCode> {
    match &cmd.op {
        Op::RoutineCreate {
            id,
            parent_id,
            name,
            schedule,
            start,
            length_days,
            time,
            goal_id,
            order,
        } => {
            if existing.is_some() {
                return Err(ErrorCode::Exists);
            }
            let name = name.trim();
            if name.is_empty() {
                return Err(ErrorCode::TitleRequired);
            }
            let r = Routine {
                id: *id,
                parent_id: *parent_id,
                name: name.into(),
                schedule: *schedule,
                start: start.clone(),
                length_days: *length_days,
                time: time.clone(),
                goal_id: *goal_id,
                pauses: vec![],
                order: order.clone(),
                created_at: Some(cmd.hlc.wall),
                trashed_at: None,
                archived_at: None,
            };
            check_fields(&r)?;
            Ok(Row::Routine(r))
        }
        Op::RoutineUpdate {
            name,
            schedule,
            length_days,
            time,
            goal_id,
            ..
        } => {
            let Some(Row::Routine(old)) = existing else {
                return Err(ErrorCode::NotFound);
            };
            let name = match name.as_deref().map(str::trim) {
                Some("") => return Err(ErrorCode::TitleRequired),
                Some(new) => new.into(),
                None => old.name.clone(),
            };
            let r = Routine {
                name,
                schedule: schedule.unwrap_or(old.schedule),
                length_days: length_days.unwrap_or(old.length_days),
                time: time.clone().unwrap_or(old.time.clone()),
                goal_id: goal_id.unwrap_or(old.goal_id),
                ..old
            };
            check_fields(&r)?;
            Ok(Row::Routine(r))
        }
        Op::RoutinePause { date, .. } => {
            let Some(Row::Routine(mut r)) = existing else {
                return Err(ErrorCode::NotFound);
            };
            if !is_date(date) || r.pauses.last().is_some_and(|p| p.to.is_none()) {
                return Err(ErrorCode::BadField);
            }
            r.pauses.push(Pause {
                from: date.clone(),
                to: None,
            });
            Ok(Row::Routine(r))
        }
        Op::RoutineResume { date, .. } => {
            let Some(Row::Routine(mut r)) = existing else {
                return Err(ErrorCode::NotFound);
            };
            if !is_date(date) {
                return Err(ErrorCode::BadField);
            }
            let Some(open) = r.pauses.last_mut().filter(|p| p.to.is_none()) else {
                return Err(ErrorCode::BadField);
            };
            if date.as_str() <= open.from.as_str() {
                r.pauses.pop();
            } else {
                open.to = Some(date.clone());
            }
            Ok(Row::Routine(r))
        }
        Op::CheckinSet {
            id,
            routine_id,
            date,
            state,
        } => {
            if !is_date(date) || parse_ymd(date).is_none() || *id != checkin_id(*routine_id, date) {
                return Err(ErrorCode::BadField);
            }
            match existing {
                None => Ok(Row::Checkin(Checkin {
                    id: *id,
                    routine_id: *routine_id,
                    date: date.clone(),
                    state: *state,
                    cleared: false,
                })),
                Some(Row::Checkin(c)) => Ok(Row::Checkin(Checkin {
                    state: *state,
                    cleared: false,
                    ..c
                })),
                Some(_) => Err(ErrorCode::BadField),
            }
        }
        Op::CheckinClear { .. } => match existing {
            Some(Row::Checkin(c)) => Ok(Row::Checkin(Checkin { cleared: true, ..c })),
            _ => Err(ErrorCode::NotFound),
        },
        _ => Err(ErrorCode::BadField),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Hlc;

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 + n)
    }

    fn routine(schedule: Schedule, start: &str) -> Routine {
        Routine {
            id: uid(1),
            parent_id: uid(100),
            name: "R".into(),
            schedule,
            start: start.into(),
            length_days: None,
            time: None,
            goal_id: None,
            pauses: vec![],
            order: String::new(),
            created_at: None,
            trashed_at: None,
            archived_at: None,
        }
    }

    fn mark(date: &str, state: CheckState) -> Checkin {
        Checkin {
            id: checkin_id(uid(1), date),
            routine_id: uid(1),
            date: date.into(),
            state,
            cleared: false,
        }
    }

    fn done(dates: &[&str]) -> Vec<Checkin> {
        dates.iter().map(|d| mark(d, CheckState::Done)).collect()
    }

    fn cmd(op: Op) -> Command {
        Command {
            uuid: uid(900),
            op,
            hlc: Hlc {
                wall: 5,
                counter: 0,
                device: Uuid::from_u128(1),
            },
        }
    }

    // 2026-03-02 is a Monday.
    #[test]
    fn a_daily_streak_counts_done_days_and_a_miss_breaks_it() {
        let r = routine(Schedule::Daily, "2026-03-02");
        let c = done(&["2026-03-02", "2026-03-03", "2026-03-05", "2026-03-06"]);
        let s = stats(&r, &c, "2026-03-06");
        assert_eq!((s.current_streak, s.best_streak), (2, 2));
        assert_eq!(s.completion_rate, Some(4.0 / 5.0));
        assert_eq!(s.days[2], ("2026-03-04".into(), DayState::Missed));
    }

    #[test]
    fn today_is_pending_not_missed_and_the_streak_survives_it() {
        let r = routine(Schedule::Daily, "2026-03-02");
        let s = stats(&r, &done(&["2026-03-02", "2026-03-03"]), "2026-03-04");
        assert_eq!(s.current_streak, 2);
        assert_eq!(s.days.last().unwrap().1, DayState::Pending);
    }

    #[test]
    fn a_skip_keeps_the_streak_and_is_left_out_of_the_rate() {
        let r = routine(Schedule::Daily, "2026-03-02");
        let mut c = done(&["2026-03-02", "2026-03-04"]);
        c.push(mark("2026-03-03", CheckState::Skipped));
        let s = stats(&r, &c, "2026-03-04");
        assert_eq!((s.current_streak, s.best_streak), (2, 2));
        assert_eq!(s.completion_rate, Some(1.0));
    }

    #[test]
    fn a_cleared_check_in_is_a_miss_again() {
        let r = routine(Schedule::Daily, "2026-03-02");
        let mut c = done(&["2026-03-02", "2026-03-03"]);
        c[0].cleared = true;
        let s = stats(&r, &c, "2026-03-04");
        assert_eq!(s.days[0].1, DayState::Missed);
        assert_eq!(s.best_streak, 1);
    }

    #[test]
    fn weekdays_and_every_n_days_only_miss_on_scheduled_days() {
        let w = routine(Schedule::Weekdays, "2026-03-02");
        let c = done(&["2026-03-02", "2026-03-03", "2026-03-04", "2026-03-05"]);
        // Fri done too, then the weekend is off and Monday is pending.
        let mut c = c;
        c.extend(done(&["2026-03-06"]));
        let s = stats(&w, &c, "2026-03-09");
        assert_eq!(s.current_streak, 5);
        assert_eq!(s.days[5].1, DayState::Off);
        let e = routine(Schedule::Every { n: 3 }, "2026-03-02");
        let s = stats(&e, &done(&["2026-03-02", "2026-03-05"]), "2026-03-07");
        assert_eq!(s.current_streak, 2);
        assert_eq!(s.days[1].1, DayState::Off);
        let s = stats(&e, &done(&["2026-03-02"]), "2026-03-07");
        assert_eq!((s.current_streak, s.best_streak), (0, 1));
    }

    #[test]
    fn per_week_counts_weeks_and_the_current_week_is_not_a_miss_yet() {
        let r = routine(Schedule::PerWeek { n: 2 }, "2026-03-02");
        let c = done(&[
            "2026-03-02",
            "2026-03-04",
            "2026-03-09",
            "2026-03-10",
            "2026-03-16",
        ]);
        let s = stats(&r, &c, "2026-03-18");
        assert_eq!((s.current_streak, s.best_streak), (2, 2));
        assert_eq!(s.completion_rate, Some(1.0));
        // Once that week ends short, it is a miss.
        let s = stats(&r, &c, "2026-03-23");
        assert_eq!((s.current_streak, s.best_streak), (0, 2));
        assert_eq!(s.completion_rate, Some(2.0 / 3.0));
    }

    #[test]
    fn a_pause_stops_misses_and_the_streak_carries_over_it() {
        let mut r = routine(Schedule::Daily, "2026-03-02");
        r.pauses = vec![Pause {
            from: "2026-03-03".into(),
            to: Some("2026-03-05".into()),
        }];
        let s = stats(&r, &done(&["2026-03-02", "2026-03-05"]), "2026-03-05");
        assert_eq!(s.days[1].1, DayState::Paused);
        assert_eq!(s.days[2].1, DayState::Paused);
        assert_eq!((s.current_streak, s.completion_rate), (2, Some(1.0)));
        assert!(is_paused(&r, "2026-03-04"));
        assert!(!is_paused(&r, "2026-03-05"));
    }

    #[test]
    fn nothing_is_missed_after_the_end() {
        let mut r = routine(Schedule::Daily, "2026-03-02");
        r.length_days = Some(3);
        let s = stats(
            &r,
            &done(&["2026-03-02", "2026-03-03", "2026-03-04"]),
            "2026-03-20",
        );
        assert_eq!((s.current_streak, s.completion_rate), (3, Some(1.0)));
        assert_eq!(s.days.len(), 3);
        assert!(!is_ended(&r, "2026-03-04"));
        assert!(is_ended(&r, "2026-03-05"));
    }

    #[test]
    fn day_start_pushes_the_early_hours_into_the_previous_day() {
        // 2026-03-10 02:00 UTC.
        let ms = parse_ymd("2026-03-10").unwrap() as u64 * 86_400_000 + 2 * 3_600_000;
        assert_eq!(local_day(ms, 0, "00:00"), "2026-03-10");
        assert_eq!(local_day(ms, 0, "04:00"), "2026-03-09");
        assert_eq!(local_day(ms, 0, "02:00"), "2026-03-10");
        assert_eq!(local_day(ms, 120, "04:00"), "2026-03-10");
        assert_eq!(local_day(ms, -180, "00:00"), "2026-03-09");
        assert_eq!(local_day(ms, 0, "bogus"), "2026-03-10");
    }

    /// Epoch ms of a local `date` at `minutes` past midnight, in a zone at `offset`.
    fn at(date: &str, minutes: i64, offset: i64) -> u64 {
        ((parse_ymd(date).unwrap() * 1440 + minutes - offset) * 60_000) as u64
    }

    #[test]
    fn a_dst_change_neither_adds_nor_drops_a_day() {
        // Clocks go forward on 2026-03-29: +60 before, +120 after.
        let at_1am = [
            ("2026-03-27", 60),
            ("2026-03-28", 60),
            ("2026-03-29", 120),
            ("2026-03-30", 120),
        ];
        let days: Vec<String> = at_1am
            .iter()
            .map(|(d, off)| local_day(at(d, 100, *off), *off as i32, "04:00"))
            .collect();
        assert_eq!(
            days,
            ["2026-03-26", "2026-03-27", "2026-03-28", "2026-03-29"]
        );
        // The same instants a day apart in UTC straddle the change by 23 hours.
        let before = at("2026-03-28", 23 * 60 + 30, 60);
        let after = before + 23 * 3_600_000;
        assert_eq!(local_day(before, 60, "00:00"), "2026-03-28");
        assert_eq!(local_day(after, 120, "00:00"), "2026-03-29");
    }

    #[test]
    fn a_streak_holds_across_the_day_start_and_a_dst_change() {
        let r = routine(Schedule::Daily, "2026-03-27");
        // Ticks at 01:30 local with a 04:00 Day start belong to the day before.
        let ticks = [
            (at("2026-03-28", 90, 60), 60),
            (at("2026-03-29", 90 + 60, 120), 120),
            (at("2026-03-30", 90, 120), 120),
            (at("2026-03-31", 90, 120), 120),
        ];
        let c: Vec<Checkin> = ticks
            .iter()
            .map(|(ms, off)| mark(&local_day(*ms, *off, "04:00"), CheckState::Done))
            .collect();
        let dates: Vec<&str> = c.iter().map(|c| c.date.as_str()).collect();
        assert_eq!(
            dates,
            ["2026-03-27", "2026-03-28", "2026-03-29", "2026-03-30"]
        );
        let today = local_day(at("2026-03-31", 90, 120), 120, "04:00");
        let s = stats(&r, &c, &today);
        assert_eq!((s.current_streak, s.best_streak), (4, 4));
        assert_eq!(s.days.len(), 4);
    }

    #[test]
    fn the_check_in_id_is_stable_and_per_routine_and_day() {
        let a = checkin_id(uid(1), "2026-03-02");
        assert_eq!(a, checkin_id(uid(1), "2026-03-02"));
        assert_ne!(a, checkin_id(uid(1), "2026-03-03"));
        assert_ne!(a, checkin_id(uid(2), "2026-03-02"));
    }

    fn create(schedule: Schedule, name: &str) -> Op {
        Op::RoutineCreate {
            id: uid(1),
            parent_id: uid(100),
            name: name.into(),
            schedule,
            start: "2026-03-02".into(),
            length_days: None,
            time: None,
            goal_id: None,
            order: String::new(),
        }
    }

    #[test]
    fn create_validates_and_update_clears() {
        assert_eq!(
            apply(None, &cmd(create(Schedule::Daily, "  "))),
            Err(ErrorCode::TitleRequired)
        );
        assert_eq!(
            apply(None, &cmd(create(Schedule::PerWeek { n: 8 }, "R"))),
            Err(ErrorCode::BadField)
        );
        assert_eq!(
            apply(None, &cmd(create(Schedule::Every { n: 0 }, "R"))),
            Err(ErrorCode::BadField)
        );
        let row = apply(None, &cmd(create(Schedule::Daily, "R"))).unwrap();
        assert_eq!(
            apply(Some(row.clone()), &cmd(create(Schedule::Daily, "R"))),
            Err(ErrorCode::Exists)
        );
        let set = apply(
            Some(row),
            &cmd(Op::RoutineUpdate {
                id: uid(1),
                name: None,
                schedule: None,
                length_days: Some(Some(30)),
                time: Some(Some("07:30".into())),
                goal_id: Some(Some(uid(5))),
            }),
        )
        .unwrap();
        let Row::Routine(r) = &set else { panic!() };
        assert_eq!(
            (r.length_days, r.time.as_deref(), r.goal_id),
            (Some(30), Some("07:30"), Some(uid(5)))
        );
        let cleared = apply(
            Some(set),
            &cmd(Op::RoutineUpdate {
                id: uid(1),
                name: None,
                schedule: None,
                length_days: Some(None),
                time: Some(None),
                goal_id: Some(None),
            }),
        )
        .unwrap();
        let Row::Routine(r) = &cleared else { panic!() };
        assert_eq!(
            (r.length_days, r.time.clone(), r.goal_id),
            (None, None, None)
        );
    }

    #[test]
    fn pause_and_resume_edit_the_pause_list() {
        let row = apply(None, &cmd(create(Schedule::Daily, "R"))).unwrap();
        let pause = |d: &str| {
            cmd(Op::RoutinePause {
                id: uid(1),
                date: d.into(),
            })
        };
        let resume = |d: &str| {
            cmd(Op::RoutineResume {
                id: uid(1),
                date: d.into(),
            })
        };
        assert_eq!(
            apply(Some(row.clone()), &resume("2026-03-05")),
            Err(ErrorCode::BadField)
        );
        let paused = apply(Some(row), &pause("2026-03-03")).unwrap();
        assert_eq!(
            apply(Some(paused.clone()), &pause("2026-03-04")),
            Err(ErrorCode::BadField)
        );
        let Row::Routine(r) = apply(Some(paused.clone()), &resume("2026-03-05")).unwrap() else {
            panic!()
        };
        assert_eq!(r.pauses[0].to.as_deref(), Some("2026-03-05"));
        let Row::Routine(r) = apply(Some(paused), &resume("2026-03-03")).unwrap() else {
            panic!()
        };
        assert!(r.pauses.is_empty());
    }

    #[test]
    fn check_in_ids_are_derived_and_a_clear_is_undone_by_a_tick() {
        let id = checkin_id(uid(1), "2026-03-02");
        let set = |state| {
            cmd(Op::CheckinSet {
                id,
                routine_id: uid(1),
                date: "2026-03-02".into(),
                state,
            })
        };
        let bad = cmd(Op::CheckinSet {
            id: uid(7),
            routine_id: uid(1),
            date: "2026-03-02".into(),
            state: CheckState::Done,
        });
        assert_eq!(apply(None, &bad), Err(ErrorCode::BadField));
        let done = apply(None, &set(CheckState::Done)).unwrap();
        let skipped = apply(Some(done.clone()), &set(CheckState::Skipped)).unwrap();
        assert!(matches!(&skipped, Row::Checkin(c) if c.state == CheckState::Skipped));
        let clear = cmd(Op::CheckinClear { id });
        let cleared = apply(Some(done), &clear).unwrap();
        assert!(matches!(&cleared, Row::Checkin(c) if c.cleared));
        let back = apply(Some(cleared), &set(CheckState::Done)).unwrap();
        assert!(matches!(&back, Row::Checkin(c) if !c.cleared));
        assert_eq!(apply(None, &clear), Err(ErrorCode::NotFound));
    }
}
