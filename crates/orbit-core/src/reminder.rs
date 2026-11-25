//! Task reminders and Routine daily reminders, and the pure fire-time maths.
//!
//! Times are floating local `YYYY-MM-DDTHH:MM`; the caller passes the device's local "now".
//! Offsets follow the Task's current due or deadline, so they re-apply to each occurrence of a
//! recurring Task. Absolute reminders are spent once fired.
use crate::parse::{fmt_date, parse_ymd};
use crate::routine::{self, Checkin};
use crate::trash;
use crate::{Command, ErrorCode, Op, Row, Uuid, is_date, is_time};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fires this many minutes late or more are dropped.
pub const LATE_LIMIT_MIN: i64 = 60;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    Due,
    Deadline,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    /// `minutes` before the Task's due time (timed due only) or deadline (midnight).
    Before { field: Anchor, minutes: u32 },
    /// `YYYY-MM-DDTHH:MM`.
    At { at: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Reminder {
    pub id: Uuid,
    #[serde(flatten)]
    pub trigger: Trigger,
    /// An absolute reminder that has fired.
    #[serde(default)]
    pub spent: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Legacy on/off: `true` means the "at due time" default, used only when
    /// `default_reminders` is absent.
    #[serde(default = "yes")]
    pub default_reminder: bool,
    /// Minutes before a timed due to fire a default reminder; `None` falls back to
    /// `default_reminder`.
    #[serde(default)]
    pub default_reminders: Option<Vec<u32>>,
}

fn yes() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_reminder: true,
            default_reminders: None,
        }
    }
}

impl Config {
    /// The config for the Settings row in `rows`, or the defaults when there is none.
    pub fn from_rows(rows: &[Row]) -> Config {
        rows.iter()
            .find_map(|r| match r {
                Row::Settings(s) => Some(Config {
                    default_reminder: s.default_reminder.unwrap_or(true),
                    default_reminders: s.default_reminders.clone(),
                }),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// The effective default offsets in minutes, sorted and deduplicated.
    pub fn offsets(&self) -> Vec<u32> {
        let mut v = match &self.default_reminders {
            Some(v) => v.clone(),
            None if self.default_reminder => vec![0],
            None => vec![],
        };
        v.sort_unstable();
        v.dedup();
        v
    }
}

/// One reminder due to be shown.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Fire {
    #[serde(default)]
    pub task_id: Option<Uuid>,
    #[serde(default)]
    pub routine_id: Option<Uuid>,
    /// `None` for the implicit "at due time" and for Routine reminders.
    #[serde(default)]
    pub reminder_id: Option<Uuid>,
    /// `YYYY-MM-DDTHH:MM`.
    pub at: String,
    /// Already past; otherwise still to come.
    pub late: bool,
    pub title: String,
}

/// `YYYY-MM-DDTHH:MM` as minutes since the epoch (floating, no zone).
fn minutes(s: &str) -> Option<i64> {
    let (d, t) = s.split_once('T')?;
    if !is_date(d) || !is_time(t) {
        return None;
    }
    Some(parse_ymd(d)? * 1440 + minutes_of_day(t)?)
}

fn minutes_of_day(t: &str) -> Option<i64> {
    let (h, m) = t.split_once(':')?;
    let (h, m): (i64, i64) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn fmt(min: i64) -> String {
    let day = min.div_euclid(1440);
    let r = min.rem_euclid(1440);
    format!("{}T{:02}:{:02}", fmt_date(day), r / 60, r % 60)
}

/// The reminders to show at `now_local` (`YYYY-MM-DDTHH:MM`), soonest first. Those not yet due
/// are included (`late` false) so the caller can set a timer; those under an hour late are
/// shown and the rest dropped.
pub fn schedule(rows: &[Row], now_local: &str, cfg: &Config) -> Vec<Fire> {
    let Some(now) = minutes(now_local) else {
        return vec![];
    };
    let by_id: HashMap<Uuid, Row> = rows.iter().map(|r| (r.id(), r.clone())).collect();
    let lookup = |id: Uuid| by_id.get(&id).cloned();
    let mut out = vec![];
    let mut push =
        |task: Option<Uuid>, routine: Option<Uuid>, rid: Option<Uuid>, at: i64, title: &str| {
            if now - at < LATE_LIMIT_MIN {
                out.push(Fire {
                    task_id: task,
                    routine_id: routine,
                    reminder_id: rid,
                    at: fmt(at),
                    late: at < now,
                    title: title.to_string(),
                });
            }
        };
    for row in rows {
        match row {
            Row::Task(t) => {
                if t.completed_at.is_some() || trash::hidden(row, &lookup) {
                    continue;
                }
                let due = t.due.as_ref().and_then(|d| {
                    let time = d.time.as_deref()?;
                    Some(parse_ymd(&d.date)? * 1440 + minutes_of_day(time)?)
                });
                let deadline = t.deadline.as_deref().and_then(parse_ymd).map(|d| d * 1440);
                if let Some(due) = due {
                    // The legacy bool only defaults a Task with no reminders of its own; the
                    // offset list adds one fire per offset, skipping any an explicit reminder covers.
                    let legacy_gated = cfg.default_reminders.is_none() && !t.reminders.is_empty();
                    if !legacy_gated {
                        let explicit: Vec<u32> = t
                            .reminders
                            .iter()
                            .filter_map(|r| match r.trigger {
                                Trigger::Before {
                                    field: Anchor::Due,
                                    minutes,
                                } => Some(minutes),
                                _ => None,
                            })
                            .collect();
                        for m in cfg.offsets() {
                            if !explicit.contains(&m) {
                                push(Some(t.id), None, None, due - m as i64, &t.title);
                            }
                        }
                    }
                }
                for r in &t.reminders {
                    let at = match &r.trigger {
                        Trigger::Before { field, minutes } => {
                            let base = match field {
                                Anchor::Due => due,
                                Anchor::Deadline => deadline,
                            };
                            base.map(|b| b - *minutes as i64)
                        }
                        Trigger::At { at } if !r.spent => minutes(at),
                        Trigger::At { .. } => None,
                    };
                    if let Some(at) = at {
                        push(Some(t.id), None, Some(r.id), at, &t.title);
                    }
                }
            }
            Row::Routine(r) => {
                let Some(time) = r.reminder.as_deref().and_then(minutes_of_day) else {
                    continue;
                };
                if trash::hidden(row, &lookup) {
                    continue;
                }
                let today = now.div_euclid(1440);
                for day in today - 1..=today + 1 {
                    if !routine::due_on(r, day) {
                        continue;
                    }
                    let date = fmt_date(day);
                    let checked = rows.iter().any(|c| {
                        matches!(c, Row::Checkin(Checkin { routine_id, date: d, cleared: false, .. })
                            if *routine_id == r.id && *d == date)
                    });
                    if !checked {
                        let title = if r.icon.is_empty() {
                            r.name.clone()
                        } else {
                            format!("{} {}", r.icon, r.name)
                        };
                        push(None, Some(r.id), None, day * 1440 + time, &title);
                    }
                }
            }
            _ => {}
        }
    }
    out.sort_by(|a, b| a.at.cmp(&b.at));
    out
}

fn valid(r: &Reminder) -> bool {
    match &r.trigger {
        Trigger::Before { minutes, .. } => *minutes <= 60 * 24 * 365,
        Trigger::At { at } => minutes(at).is_some(),
    }
}

pub(crate) fn apply(existing: Option<Row>, cmd: &Command) -> Result<Row, ErrorCode> {
    let Some(Row::Task(mut t)) = existing else {
        return Err(ErrorCode::NotFound);
    };
    match &cmd.op {
        Op::ReminderAdd { reminder, .. } => {
            if !valid(reminder) {
                return Err(ErrorCode::BadField);
            }
            if t.reminders.iter().any(|r| r.id == reminder.id) {
                return Err(ErrorCode::Exists);
            }
            t.reminders.push(reminder.clone());
        }
        Op::ReminderSnooze { at, .. } => {
            let reminder = Reminder {
                id: cmd.uuid,
                trigger: Trigger::At { at: at.clone() },
                spent: false,
            };
            if !valid(&reminder) {
                return Err(ErrorCode::BadField);
            }
            if t.reminders.iter().any(|r| r.id == reminder.id) {
                return Err(ErrorCode::Exists);
            }
            t.reminders.push(reminder);
        }
        Op::ReminderRemove { id, .. } => {
            if !t.reminders.iter().any(|r| r.id == *id) {
                return Err(ErrorCode::NotFound);
            }
            t.reminders.retain(|r| r.id != *id);
        }
        Op::ReminderSpend { id, .. } => {
            let Some(r) = t.reminders.iter_mut().find(|r| r.id == *id) else {
                return Err(ErrorCode::NotFound);
            };
            r.spent = true;
        }
        _ => return Err(ErrorCode::BadField),
    }
    Ok(Row::Task(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routine::{CheckState, Routine, Schedule, checkin_id};
    use crate::{Due, Hlc, Task};

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(0x0190_0000_0000_7000_8000_0000_0000_0000 + n)
    }

    fn task(due: Option<(&str, Option<&str>)>, reminders: Vec<Reminder>) -> Task {
        Task {
            id: uid(1),
            parent_id: uid(100),
            title: "T".into(),
            completed_at: None,
            order: String::new(),
            description: String::new(),
            due: due.map(|(d, t)| Due {
                date: d.into(),
                time: t.map(Into::into),
            }),
            deadline: None,
            duration_min: None,
            priority: 4,
            label_ids: vec![],
            recurrence: None,
            goal_id: None,
            created_at: None,
            trashed_at: None,
            reminders,
            links: vec![],
            blobs: vec![],
            pinned_on: None,
            not_today_on: None,
        }
    }

    fn before(n: u128, minutes: u32) -> Reminder {
        Reminder {
            id: uid(n),
            trigger: Trigger::Before {
                field: Anchor::Due,
                minutes,
            },
            spent: false,
        }
    }

    fn at(n: u128, at: &str, spent: bool) -> Reminder {
        Reminder {
            id: uid(n),
            trigger: Trigger::At { at: at.into() },
            spent,
        }
    }

    fn times(rows: &[Row], now: &str, cfg: &Config) -> Vec<String> {
        schedule(rows, now, cfg).into_iter().map(|f| f.at).collect()
    }

    const ON: Config = Config {
        default_reminder: true,
        default_reminders: None,
    };

    fn offsets(list: Option<Vec<u32>>) -> Config {
        Config {
            default_reminder: true,
            default_reminders: list,
        }
    }

    #[test]
    fn offset_follows_the_current_due() {
        let mut t = task(Some(("2026-03-02", Some("09:00"))), vec![before(2, 30)]);
        let now = "2026-03-02T08:00";
        assert_eq!(
            times(&[Row::Task(t.clone())], now, &ON),
            ["2026-03-02T08:30"]
        );
        // The recurring Task moved on: the same offset re-applies to the next occurrence.
        t.due = Some(Due {
            date: "2026-03-09".into(),
            time: Some("09:00".into()),
        });
        assert_eq!(times(&[Row::Task(t)], now, &ON), ["2026-03-09T08:30"]);
    }

    #[test]
    fn date_only_due_gets_no_offset_or_default() {
        let t = task(Some(("2026-03-02", None)), vec![before(2, 30)]);
        assert!(times(&[Row::Task(t)], "2026-03-02T08:00", &ON).is_empty());
        let t = task(Some(("2026-03-02", None)), vec![]);
        assert!(times(&[Row::Task(t)], "2026-03-02T08:00", &ON).is_empty());
    }

    #[test]
    fn deadline_offset_counts_from_midnight() {
        let mut t = task(None, vec![]);
        t.deadline = Some("2026-03-03".into());
        t.reminders.push(Reminder {
            id: uid(2),
            trigger: Trigger::Before {
                field: Anchor::Deadline,
                minutes: 60,
            },
            spent: false,
        });
        assert_eq!(
            times(&[Row::Task(t)], "2026-03-02T08:00", &ON),
            ["2026-03-02T23:00"]
        );
    }

    #[test]
    fn spent_absolute_is_gone_and_snooze_is_one_off() {
        let live = task(None, vec![at(2, "2026-03-02T09:00", false)]);
        assert_eq!(times(&[Row::Task(live)], "2026-03-02T09:00", &ON).len(), 1);
        let spent = task(None, vec![at(2, "2026-03-02T09:00", true)]);
        assert!(times(&[Row::Task(spent)], "2026-03-02T09:00", &ON).is_empty());
    }

    #[test]
    fn late_fires_under_an_hour_show_and_later_drop() {
        let t = task(None, vec![at(2, "2026-03-02T09:00", false)]);
        let rows = [Row::Task(t)];
        let f = schedule(&rows, "2026-03-02T09:59", &ON);
        assert_eq!((f.len(), f[0].late), (1, true));
        assert!(schedule(&rows, "2026-03-02T10:00", &ON).is_empty());
        assert!(schedule(&rows, "2026-03-02T10:01", &ON).is_empty());
        let f = schedule(&rows, "2026-03-02T08:00", &ON);
        assert_eq!((f.len(), f[0].late), (1, false));
    }

    #[test]
    fn default_reminder_setting() {
        let t = task(Some(("2026-03-02", Some("09:00"))), vec![]);
        let rows = [Row::Task(t)];
        assert_eq!(times(&rows, "2026-03-02T08:00", &ON), ["2026-03-02T09:00"]);
        let off = Config {
            default_reminder: false,
            default_reminders: None,
        };
        assert!(times(&rows, "2026-03-02T08:00", &off).is_empty());
    }

    #[test]
    fn legacy_bool_defaults_only_a_task_without_reminders() {
        let bare = [Row::Task(task(Some(("2026-03-02", Some("09:00"))), vec![]))];
        let with_explicit = [Row::Task(task(
            Some(("2026-03-02", Some("09:00"))),
            vec![before(2, 30)],
        ))];
        let now = "2026-03-02T08:00";
        assert_eq!(offsets(None).offsets(), [0]);
        assert_eq!(times(&bare, now, &offsets(None)), ["2026-03-02T09:00"]);
        assert_eq!(
            times(&with_explicit, now, &offsets(None)),
            ["2026-03-02T08:30"]
        );
        let off = Config {
            default_reminder: false,
            default_reminders: None,
        };
        assert!(off.offsets().is_empty());
    }

    #[test]
    fn each_default_offset_yields_one_fire() {
        let rows = [Row::Task(task(Some(("2026-03-02", Some("09:00"))), vec![]))];
        let cfg = offsets(Some(vec![30, 0, 1440, 30]));
        assert_eq!(cfg.offsets(), [0, 30, 1440]);
        let fires = schedule(&rows, "2026-03-01T08:00", &cfg);
        assert_eq!(
            times(&rows, "2026-03-01T08:00", &cfg),
            ["2026-03-01T09:00", "2026-03-02T08:30", "2026-03-02T09:00"]
        );
        assert!(
            fires
                .iter()
                .all(|f| f.reminder_id.is_none() && f.task_id.is_some())
        );
    }

    #[test]
    fn default_offsets_skip_those_an_explicit_reminder_covers() {
        let rows = [Row::Task(task(
            Some(("2026-03-02", Some("09:00"))),
            vec![before(2, 30)],
        ))];
        let cfg = offsets(Some(vec![0, 30]));
        assert_eq!(
            times(&rows, "2026-03-02T08:00", &cfg),
            ["2026-03-02T08:30", "2026-03-02T09:00"]
        );
        let fires = schedule(&rows, "2026-03-02T08:00", &cfg);
        let explicit = fires.iter().filter(|f| f.reminder_id.is_some()).count();
        assert_eq!(explicit, 1);
    }

    #[test]
    fn empty_offset_list_means_no_defaults() {
        let rows = [Row::Task(task(Some(("2026-03-02", Some("09:00"))), vec![]))];
        // An explicit empty list wins over the legacy `true`, which is still set.
        assert!(offsets(Some(vec![])).default_reminder);
        assert!(times(&rows, "2026-03-02T08:00", &offsets(Some(vec![]))).is_empty());
    }

    #[test]
    fn done_tasks_do_not_fire() {
        let mut t = task(None, vec![at(2, "2026-03-02T09:00", false)]);
        t.completed_at = Some(1);
        assert!(times(&[Row::Task(t)], "2026-03-02T09:00", &ON).is_empty());
    }

    fn routine(reminder: Option<&str>) -> Routine {
        Routine {
            id: uid(7),
            parent_id: uid(100),
            name: "Stretch".into(),
            icon: String::new(),
            schedule: Schedule::Weekdays,
            start: "2026-03-02".into(),
            length_days: None,
            time: None,
            goal_id: None,
            reminder: reminder.map(Into::into),
            pauses: vec![],
            order: String::new(),
            created_at: None,
            trashed_at: None,
            archived_at: None,
        }
    }

    #[test]
    fn routine_reminder_on_scheduled_days_until_checked_in() {
        // 2026-03-02 is a Monday; Saturday is not scheduled.
        let r = routine(Some("07:30"));
        let rows = vec![Row::Routine(r.clone())];
        assert_eq!(
            times(&rows, "2026-03-02T07:00", &ON),
            ["2026-03-02T07:30", "2026-03-03T07:30"]
        );
        assert!(times(&rows, "2026-03-07T07:00", &ON).is_empty());
        let mut rows = rows;
        rows.push(Row::Checkin(Checkin {
            id: checkin_id(r.id, "2026-03-02"),
            routine_id: r.id,
            date: "2026-03-02".into(),
            state: CheckState::Done,
            cleared: false,
        }));
        assert_eq!(times(&rows, "2026-03-02T07:00", &ON), ["2026-03-03T07:30"]);
        let Some(Row::Checkin(c)) = rows.last_mut() else {
            panic!()
        };
        c.cleared = true;
        assert_eq!(times(&rows, "2026-03-02T07:00", &ON).len(), 2);
        assert!(times(&[Row::Routine(routine(None))], "2026-03-02T07:00", &ON).is_empty());
    }

    #[test]
    fn routine_fire_title_includes_explicit_icon_and_preserves_legacy_title() {
        let mut iconed = routine(Some("07:30"));
        iconed.id = uid(8);
        iconed.icon = "🏃".into();
        let rows = [Row::Routine(iconed), Row::Routine(routine(Some("07:30")))];

        let fires = schedule(&rows, "2026-03-02T07:00", &ON);
        let explicit = fires.iter().find(|f| f.routine_id == Some(uid(8))).unwrap();
        assert_eq!(explicit.title, "🏃 Stretch");
        assert_eq!(explicit.at, "2026-03-02T07:30");
        let legacy = fires.iter().find(|f| f.routine_id == Some(uid(7))).unwrap();
        assert_eq!(legacy.title, "Stretch");
        assert_eq!(legacy.at, "2026-03-02T07:30");
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

    #[test]
    fn ops_add_snooze_spend_remove() {
        let row = Some(Row::Task(task(None, vec![])));
        let added = apply(
            row.clone(),
            &cmd(Op::ReminderAdd {
                task_id: uid(1),
                reminder: before(2, 10),
            }),
        )
        .unwrap();
        assert_eq!(
            apply(
                Some(added.clone()),
                &cmd(Op::ReminderAdd {
                    task_id: uid(1),
                    reminder: before(2, 10)
                })
            ),
            Err(ErrorCode::Exists)
        );
        assert_eq!(
            apply(
                row.clone(),
                &cmd(Op::ReminderSnooze {
                    task_id: uid(1),
                    at: "bad".into()
                })
            ),
            Err(ErrorCode::BadField)
        );
        let snoozed = apply(
            Some(added),
            &cmd(Op::ReminderSnooze {
                task_id: uid(1),
                at: "2026-03-02T09:10".into(),
            }),
        )
        .unwrap();
        let spent = apply(
            Some(snoozed),
            &cmd(Op::ReminderSpend {
                task_id: uid(1),
                id: uid(900),
            }),
        )
        .unwrap();
        let Row::Task(t) = &spent else { panic!() };
        assert_eq!(t.reminders.len(), 2);
        assert!(t.reminders[1].spent);
        let removed = apply(
            Some(spent),
            &cmd(Op::ReminderRemove {
                task_id: uid(1),
                id: uid(2),
            }),
        )
        .unwrap();
        let Row::Task(t) = &removed else { panic!() };
        assert_eq!(t.reminders.len(), 1);
        assert_eq!(
            apply(
                None,
                &cmd(Op::ReminderSpend {
                    task_id: uid(1),
                    id: uid(2)
                })
            ),
            Err(ErrorCode::NotFound)
        );
    }
}
