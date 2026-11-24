//! iCalendar feed parsing: VEVENTs (timed and all-day) with RRULE/EXDATE expansion into a window.
//!
//! Pure: the window comes in as epoch milliseconds, nothing reads the clock.
//! Limits: DURATION, RDATE, EXRULE and VTIMEZONE blocks are ignored; only IANA TZIDs are
//! understood (anything else is read as UTC, as are floating times).

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use serde::Serialize;

/// Most occurrences kept per recurring event.
pub const MAX_PER_EVENT: usize = 500;
/// Most recurrence steps examined per event, including ones before the window.
const MAX_STEPS: usize = 20_000;
/// Most events kept per feed.
pub const MAX_EVENTS: usize = 5_000;
const DAY_MS: i64 = 86_400_000;

/// One event or one occurrence of a recurring event. All-day events carry `start_date` and
/// `end_date` (exclusive), plain `YYYY-MM-DD` calendar dates with no zone; their `*_ms` are only
/// UTC-midnight sort keys and must not be used to pick a viewer's day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    pub uid: String,
    pub summary: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub all_day: bool,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// `YYYY-MM-DD` of a UTC-midnight all-day stamp.
pub fn date_of(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.date_naive().to_string())
        .unwrap_or_default()
}

struct Prop {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

impl Prop {
    fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, Copy)]
enum Zone {
    Utc,
    Named(chrono_tz::Tz),
}

struct Stamp {
    local: NaiveDateTime,
    zone: Zone,
    all_day: bool,
}

impl Stamp {
    fn ms(&self) -> i64 {
        match self.zone {
            Zone::Utc => self.local.and_utc().timestamp_millis(),
            Zone::Named(tz) => tz
                .from_local_datetime(&self.local)
                .earliest()
                .or_else(|| tz.from_local_datetime(&self.local).latest())
                .map_or_else(
                    || self.local.and_utc().timestamp_millis(),
                    |d| d.timestamp_millis(),
                ),
        }
    }

    /// `:<utc>` or `;TZID=<zone>:<local>`, the tail of a DTSTART/EXDATE line for rrule.
    fn ical(&self) -> String {
        let f = self.local.format("%Y%m%dT%H%M%S");
        match self.zone {
            Zone::Named(tz) if !self.all_day => format!(";TZID={}:{f}", tz.name()),
            _ => format!(":{f}Z"),
        }
    }
}

fn stamp(p: &Prop, default: Zone) -> Option<Stamp> {
    let v = p.value.trim();
    let date_only = p.param("VALUE") == Some("DATE") || v.len() == 8;
    if date_only {
        let d = NaiveDate::parse_from_str(v, "%Y%m%d").ok()?;
        return Some(Stamp {
            local: d.and_time(NaiveTime::MIN),
            zone: Zone::Utc,
            all_day: true,
        });
    }
    let (v, utc) = v.strip_suffix('Z').map_or((v, false), |s| (s, true));
    let local = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
    let zone = if utc {
        Zone::Utc
    } else {
        p.param("TZID")
            .map_or(default, |t| t.parse().map_or(Zone::Utc, Zone::Named))
    };
    Some(Stamp {
        local,
        zone,
        all_day: false,
    })
}

/// Unfolds continuation lines and splits `NAME;K=V:value` content lines.
fn lines(ics: &str) -> Vec<Prop> {
    let mut raw: Vec<String> = Vec::new();
    for l in ics.lines() {
        if let Some(rest) = l.strip_prefix([' ', '\t']) {
            if let Some(last) = raw.last_mut() {
                last.push_str(rest);
            }
        } else {
            raw.push(l.to_string());
        }
    }
    raw.iter().filter_map(|l| prop(l)).collect()
}

fn prop(line: &str) -> Option<Prop> {
    let mut quoted = false;
    let colon = line.char_indices().find_map(|(i, c)| {
        quoted ^= c == '"';
        (c == ':' && !quoted).then_some(i)
    })?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?.to_ascii_uppercase();
    let params = parts
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_ascii_uppercase(), v.trim_matches('"').to_string()))
        .collect();
    Some(Prop {
        name,
        params,
        value: value.to_string(),
    })
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(o) => out.push(o),
            None => {}
        }
    }
    out
}

struct Raw {
    props: Vec<Prop>,
}

impl Raw {
    fn get(&self, name: &str) -> Option<&Prop> {
        self.props.iter().find(|p| p.name == name)
    }
}

/// Parses `ics` and returns events overlapping `[from_ms, to_ms)`, sorted by start.
pub fn parse_feed(ics: &str, from_ms: i64, to_ms: i64) -> Vec<Event> {
    let mut events: Vec<Raw> = Vec::new();
    let mut cur: Option<Raw> = None;
    let mut alarm = false;
    for p in lines(ics) {
        match (p.name.as_str(), p.value.as_str()) {
            ("BEGIN", "VEVENT") => cur = Some(Raw { props: vec![] }),
            ("END", "VEVENT") => events.extend(cur.take()),
            ("BEGIN", "VALARM") => alarm = true,
            ("END", "VALARM") => alarm = false,
            _ if !alarm => {
                if let Some(c) = cur.as_mut() {
                    c.props.push(p);
                }
            }
            _ => {}
        }
    }
    // Moved or edited instances replace the master's occurrence at their RECURRENCE-ID.
    let replaced: Vec<(String, i64)> = events
        .iter()
        .filter_map(|e| {
            let id = stamp(e.get("RECURRENCE-ID")?, Zone::Utc)?.ms();
            Some((e.get("UID")?.value.clone(), id))
        })
        .collect();
    let mut out = Vec::new();
    for e in &events {
        if e.get("STATUS")
            .is_some_and(|s| s.value.eq_ignore_ascii_case("CANCELLED"))
        {
            continue;
        }
        expand(e, from_ms, to_ms, &replaced, &mut out);
        if out.len() >= MAX_EVENTS {
            break;
        }
    }
    out.sort_by_key(|e: &Event| (e.start_ms, e.uid.clone()));
    out.truncate(MAX_EVENTS);
    out
}

fn expand(e: &Raw, from: i64, to: i64, replaced: &[(String, i64)], out: &mut Vec<Event>) {
    let Some(start) = e.get("DTSTART").and_then(|p| stamp(p, Zone::Utc)) else {
        return;
    };
    let uid = e.get("UID").map_or(String::new(), |p| p.value.clone());
    let summary = e
        .get("SUMMARY")
        .map_or(String::new(), |p| unescape(&p.value));
    let start_ms = start.ms();
    let end_ms = e
        .get("DTEND")
        .and_then(|p| stamp(p, start.zone))
        .map(|s| s.ms())
        .filter(|&end| end >= start_ms)
        .unwrap_or(if start.all_day {
            start_ms + DAY_MS
        } else {
            start_ms
        });
    let len = end_ms - start_ms;
    let mut push = |s: i64| {
        let hit = s < to && s + len.max(1) > from;
        if hit {
            out.push(Event {
                uid: uid.clone(),
                summary: summary.clone(),
                start_ms: s,
                end_ms: s + len,
                all_day: start.all_day,
                start_date: start.all_day.then(|| date_of(s)),
                end_date: start.all_day.then(|| date_of(s + len)),
            });
        }
        hit
    };
    let rules: Vec<&Prop> = e.props.iter().filter(|p| p.name == "RRULE").collect();
    let set = (!rules.is_empty()).then(|| {
        let mut text = format!("DTSTART{}", start.ical());
        for r in &rules {
            text.push_str(&format!("\nRRULE:{}", fix_until(&r.value)));
        }
        for x in e.props.iter().filter(|p| p.name == "EXDATE") {
            for v in x.value.split(',') {
                let one = Prop {
                    name: x.name.clone(),
                    params: x.params.clone(),
                    value: v.into(),
                };
                if let Some(s) = stamp(&one, start.zone) {
                    text.push_str(&format!("\nEXDATE{}", s.ical()));
                }
            }
        }
        text.parse::<rrule::RRuleSet>()
    });
    match set {
        // A rule the crate rejects degrades to the master instance alone.
        None | Some(Err(_)) => {
            push(start_ms);
        }
        Some(Ok(set)) => {
            let mut kept = 0;
            for (n, dt) in set.limit().into_iter().enumerate() {
                let s = dt.timestamp_millis();
                if n >= MAX_STEPS || s >= to || kept >= MAX_PER_EVENT {
                    break;
                }
                if replaced.iter().any(|(u, id)| *u == uid && *id == s) {
                    continue;
                }
                kept += usize::from(push(s));
            }
        }
    }
}

/// A date-only or floating UNTIL gets a time and `Z` so it matches the DTSTART form handed to rrule.
fn fix_until(rule: &str) -> String {
    rule.split(';')
        .map(|part| match part.strip_prefix("UNTIL=") {
            Some(u) if u.len() == 8 => format!("UNTIL={u}T235959Z"),
            Some(u) if u.len() == 15 => format!("UNTIL={u}Z"),
            _ => part.to_string(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(y: i32, m: u32, d: u32, h: u32, mi: u32) -> i64 {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis()
    }

    fn feed(events: &str) -> String {
        format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{events}END:VCALENDAR\r\n")
    }

    fn all(ics: &str) -> Vec<Event> {
        parse_feed(ics, ms(2020, 1, 1, 0, 0), ms(2030, 1, 1, 0, 0))
    }

    fn starts(ev: &[Event]) -> Vec<i64> {
        ev.iter().map(|e| e.start_ms).collect()
    }

    #[test]
    fn all_day_event_spans_utc_midnights_and_unescapes_summary() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20240301\r\nDTEND;VALUE=DATE:20240303\r\nSUMMARY:Trip\\, with\r\n  fold\\nline\r\nEND:VEVENT\r\n",
        ));
        assert_eq!(ev.len(), 1);
        assert!(ev[0].all_day);
        assert_eq!(ev[0].start_ms, ms(2024, 3, 1, 0, 0));
        assert_eq!(ev[0].end_ms, ms(2024, 3, 3, 0, 0));
        assert_eq!(ev[0].summary, "Trip, with fold\nline");
        assert_eq!(ev[0].start_date.as_deref(), Some("2024-03-01"));
        assert_eq!(ev[0].end_date.as_deref(), Some("2024-03-03"));
    }

    #[test]
    fn all_day_without_dtend_lasts_one_day() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20240301\r\nEND:VEVENT\r\n",
        ));
        assert_eq!(ev[0].end_ms - ev[0].start_ms, DAY_MS);
    }

    #[test]
    fn tzid_and_utc_times_become_utc_millis() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:ny\r\nDTSTART;TZID=America/New_York:20240115T090000\r\nDTEND;TZID=America/New_York:20240115T100000\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:utc\r\nDTSTART:20240115T090000Z\r\nDTEND:20240115T093000Z\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:summer\r\nDTSTART;TZID=America/New_York:20240715T090000\r\nEND:VEVENT\r\n",
        ));
        let by = |u: &str| ev.iter().find(|e| e.uid == u).unwrap().clone();
        assert_eq!(by("ny").start_ms, ms(2024, 1, 15, 14, 0));
        assert_eq!(by("ny").end_ms, ms(2024, 1, 15, 15, 0));
        assert_eq!(by("utc").start_ms, ms(2024, 1, 15, 9, 0));
        assert_eq!(by("summer").start_ms, ms(2024, 7, 15, 13, 0), "DST applies");
        assert!(!by("ny").all_day);
    }

    #[test]
    fn rrule_count() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20240101T100000Z\r\nDTEND:20240101T110000Z\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEND:VEVENT\r\n",
        ));
        assert_eq!(
            starts(&ev),
            [
                ms(2024, 1, 1, 10, 0),
                ms(2024, 1, 2, 10, 0),
                ms(2024, 1, 3, 10, 0)
            ]
        );
        assert_eq!(ev[1].end_ms - ev[1].start_ms, 3_600_000);
    }

    #[test]
    fn rrule_interval_and_byday() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO;COUNT=3\r\nEND:VEVENT\r\n",
        ));
        assert_eq!(
            starts(&ev),
            [
                ms(2024, 1, 1, 10, 0),
                ms(2024, 1, 15, 10, 0),
                ms(2024, 1, 29, 10, 0)
            ]
        );
    }

    #[test]
    fn rrule_until_date_on_all_day_event() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20240101\r\nRRULE:FREQ=DAILY;UNTIL=20240103\r\nEND:VEVENT\r\n",
        ));
        assert_eq!(
            starts(&ev),
            [
                ms(2024, 1, 1, 0, 0),
                ms(2024, 1, 2, 0, 0),
                ms(2024, 1, 3, 0, 0)
            ]
        );
        assert!(ev.iter().all(|e| e.all_day));
    }

    #[test]
    fn rrule_until_utc_with_tz_dtstart_keeps_local_hour_across_dst() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;TZID=America/New_York:20240308T090000\r\nRRULE:FREQ=DAILY;UNTIL=20240311T200000Z\r\nEND:VEVENT\r\n",
        ));
        // DST starts 2024-03-10: 09:00 local is 14:00Z before, 13:00Z after.
        assert_eq!(
            starts(&ev),
            [
                ms(2024, 3, 8, 14, 0),
                ms(2024, 3, 9, 14, 0),
                ms(2024, 3, 10, 13, 0),
                ms(2024, 3, 11, 13, 0)
            ]
        );
    }

    #[test]
    fn exdate_removes_occurrences_all_day_and_timed() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:t\r\nDTSTART;TZID=Europe/London:20240101T090000\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEXDATE;TZID=Europe/London:20240102T090000\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:d\r\nDTSTART;VALUE=DATE:20240101\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEXDATE;VALUE=DATE:20240102,20240103\r\nEND:VEVENT\r\n",
        ));
        let t: Vec<_> = ev.iter().filter(|e| e.uid == "t").collect();
        assert_eq!(t.len(), 2);
        assert_eq!(t[1].start_ms, ms(2024, 1, 3, 9, 0));
        assert_eq!(ev.iter().filter(|e| e.uid == "d").count(), 1);
    }

    #[test]
    fn window_limits_expansion_of_open_ended_rules() {
        let ics = feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=DAILY\r\nEND:VEVENT\r\n",
        );
        let ev = parse_feed(&ics, ms(2024, 2, 1, 0, 0), ms(2024, 2, 4, 0, 0));
        assert_eq!(
            starts(&ev),
            [
                ms(2024, 2, 1, 10, 0),
                ms(2024, 2, 2, 10, 0),
                ms(2024, 2, 3, 10, 0)
            ]
        );
    }

    #[test]
    fn occurrences_per_event_are_capped() {
        let ics = feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=HOURLY\r\nEND:VEVENT\r\n",
        );
        let ev = parse_feed(&ics, ms(2024, 1, 1, 0, 0), ms(2029, 1, 1, 0, 0));
        assert_eq!(ev.len(), MAX_PER_EVENT);
    }

    #[test]
    fn cancelled_events_and_alarms_are_ignored_and_overrides_replace_the_instance() {
        let ev = all(&feed(
            "BEGIN:VEVENT\r\nUID:x\r\nDTSTART:20240101T100000Z\r\nSTATUS:CANCELLED\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:r\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=DAILY;COUNT=2\r\nSUMMARY:Standup\r\nBEGIN:VALARM\r\nSUMMARY:no\r\nEND:VALARM\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:r\r\nRECURRENCE-ID:20240102T100000Z\r\nDTSTART:20240102T150000Z\r\nSUMMARY:Moved\r\nEND:VEVENT\r\n",
        ));
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].summary, "Standup");
        assert_eq!(
            (ev[1].summary.as_str(), ev[1].start_ms),
            ("Moved", ms(2024, 1, 2, 15, 0))
        );
    }

    #[test]
    fn events_outside_the_window_are_dropped_and_bad_rrule_keeps_the_master() {
        let ics = feed(
            "BEGIN:VEVENT\r\nUID:old\r\nDTSTART:20150101T100000Z\r\nEND:VEVENT\r\n\
             BEGIN:VEVENT\r\nUID:bad\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=NOPE\r\nEND:VEVENT\r\n",
        );
        let ev = all(&ics);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].uid, "bad");
    }
}
