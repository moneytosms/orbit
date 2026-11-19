//! Quick-add parser. Date crate: none, hand-rolled. `interim` (chrono) does build for
//! wasm32, but it parses only whole strings (no phrase spans), rejects `in 3 days`, reads
//! `may` and a bare `5` as dates, and never rolls past dates forward, so it was dropped.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Known {
    /// Areas and Projects.
    #[serde(default)]
    pub areas: Vec<String>,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Place,
    Goal,
    Page,
    Label,
    Priority,
    Date,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: Kind,
    /// As typed, including the sigil.
    pub text: String,
    /// The name, "1".."4", or `YYYY-MM-DD` (`YYYY-MM-DDTHH:MM` when a time was given).
    pub value: String,
    /// Byte offsets in the input.
    pub start: usize,
    pub end: usize,
    pub known: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Due {
    pub date: String,
    pub time: Option<String>,
}

/// Parsed fields are set only from resolved tokens; unresolved ones appear in `tokens` only.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Parsed {
    pub title: String,
    pub tokens: Vec<Token>,
    pub priority: Option<u8>,
    pub due: Option<Due>,
    pub place: Option<String>,
    pub goal: Option<String>,
    pub page: Option<String>,
    pub labels: Vec<String>,
}

struct Word<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

fn words(input: &str) -> Vec<Word<'_>> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in input.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push(Word {
                    text: &input[s..i],
                    start: s,
                    end: i,
                });
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push(Word {
            text: &input[s..],
            start: s,
            end: input.len(),
        });
    }
    out
}

// Civil date arithmetic (days since 1970-01-01).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    days_from_civil(
        if m == 12 { y + 1 } else { y },
        if m == 12 { 1 } else { m + 1 },
        1,
    ) - days_from_civil(y, m, 1)
}

fn fmt_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Day number and minutes since midnight of a local `YYYY-MM-DDTHH:MM`.
fn parse_now(now: &str) -> Option<i64> {
    let (date, _) = now.split_once('T')?;
    parse_ymd(date)
}

fn parse_ymd(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: i64 = s[5..7].parse().ok()?;
    let d: i64 = s[8..10].parse().ok()?;
    if !s
        .bytes()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return None;
    }
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// 0 = Monday.
fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7)
}

fn weekday_name(w: &str) -> Option<i64> {
    Some(match w {
        "mon" | "monday" => 0,
        "tue" | "tues" | "tuesday" => 1,
        "wed" | "weds" | "wednesday" => 2,
        "thu" | "thur" | "thurs" | "thursday" => 3,
        "fri" | "friday" => 4,
        "sat" | "saturday" => 5,
        "sun" | "sunday" => 6,
        _ => return None,
    })
}

fn month_name(w: &str) -> Option<i64> {
    Some(match w {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    })
}

fn day_number(w: &str) -> Option<i64> {
    if w.is_empty() || w.len() > 2 || !w.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    w.parse().ok()
}

/// Next occurrence of month/day on or after `today`.
fn month_day(today: i64, m: i64, d: i64) -> Option<i64> {
    let (y, _, _) = civil_from_days(today);
    for y in [y, y + 1] {
        if d >= 1 && d <= days_in_month(y, m) {
            let t = days_from_civil(y, m, d);
            if t >= today {
                return Some(t);
            }
        }
    }
    None
}

/// A date phrase at the start of `w` (lowercased words): (day number, words used).
fn date_phrase(w: &[String], today: i64) -> Option<(i64, usize)> {
    let first = w[0].as_str();
    match first {
        "today" => return Some((today, 1)),
        "tomorrow" => return Some((today + 1, 1)),
        "next" if w.get(1).map(String::as_str) == Some("week") => {
            return Some((today + 7 - weekday(today), 2));
        }
        "in" => {
            let n: i64 = w.get(1)?.parse().ok().filter(|n| (1..=9999).contains(n))?;
            let unit = match w.get(2)?.as_str() {
                "day" | "days" => 1,
                "week" | "weeks" => 7,
                _ => return None,
            };
            return Some((today + n * unit, 3));
        }
        _ => {}
    }
    if let Some(wd) = weekday_name(first) {
        let ahead = (wd - weekday(today)).rem_euclid(7);
        return Some((today + if ahead == 0 { 7 } else { ahead }, 1));
    }
    if let Some(t) = parse_ymd(first) {
        return Some((t, 1));
    }
    if let Some(m) = month_name(first) {
        let d = day_number(w.get(1)?)?;
        return Some((month_day(today, m, d)?, 2));
    }
    if let Some(d) = day_number(first) {
        let m = month_name(w.get(1)?)?;
        return Some((month_day(today, m, d)?, 2));
    }
    None
}

/// `6pm`, `6:30pm`, `18:00` as `HH:MM`.
fn time_word(w: &str) -> Option<String> {
    let (body, meridiem) = if let Some(b) = w.strip_suffix("am") {
        (b, Some(false))
    } else if let Some(b) = w.strip_suffix("pm") {
        (b, Some(true))
    } else {
        (w, None)
    };
    let (h, m) = match body.split_once(':') {
        Some((h, m)) => {
            if m.len() != 2 {
                return None;
            }
            (h, m)
        }
        None if meridiem.is_some() => (body, "00"),
        None => return None,
    };
    if h.is_empty() || h.len() > 2 || !h.bytes().chain(m.bytes()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if m > 59 {
        return None;
    }
    match meridiem {
        Some(pm) => {
            if !(1..=12).contains(&h) {
                return None;
            }
            h = h % 12 + if pm { 12 } else { 0 };
        }
        None if h > 23 => return None,
        None => {}
    }
    Some(format!("{h:02}:{m:02}"))
}

fn is_sigil(c: char) -> bool {
    matches!(c, '@' | '$' | '%' | '#')
}

fn priority_word(w: &str) -> Option<u8> {
    match w.to_ascii_lowercase().as_str() {
        "p1" => Some(1),
        "p2" => Some(2),
        "p3" => Some(3),
        "p4" => Some(4),
        _ => None,
    }
}

pub fn parse(input: &str, now: &str, known: &Known) -> Parsed {
    let ws = words(input);
    let lower: Vec<String> = ws.iter().map(|w| w.text.to_lowercase()).collect();
    let today = parse_now(now);
    let mut out = Parsed {
        title: String::new(),
        tokens: Vec::new(),
        priority: None,
        due: None,
        place: None,
        goal: None,
        page: None,
        labels: Vec::new(),
    };
    let mut title: Vec<String> = Vec::new();
    let mut i = 0;
    while i < ws.len() {
        let w = &ws[i];
        let first = w.text.chars().next().unwrap_or(' ');
        // Escaped sigil stays literal.
        if first == '\\' && w.text[1..].chars().next().is_some_and(is_sigil) {
            title.push(w.text[1..].to_string());
            i += 1;
            continue;
        }
        if is_sigil(first) && w.text.len() > 1 {
            let (kind, list) = match first {
                '@' => (Kind::Place, &known.areas),
                '$' => (Kind::Goal, &known.goals),
                '%' => (Kind::Page, &known.pages),
                _ => (Kind::Label, &known.labels),
            };
            let name_start = w.start + 1;
            let mut best: Option<(usize, &String)> = None;
            for (j, wj) in ws.iter().enumerate().skip(i) {
                let slice = input[name_start..wj.end].to_lowercase();
                if let Some(n) = list.iter().find(|n| n.to_lowercase() == slice) {
                    best = Some((j, n));
                }
            }
            let (last, value, is_known) = match best {
                Some((j, n)) => (j, n.clone(), true),
                None => (i, w.text[1..].to_string(), false),
            };
            let end = ws[last].end;
            out.tokens.push(Token {
                kind,
                text: input[w.start..end].to_string(),
                value: value.clone(),
                start: w.start,
                end,
                known: is_known,
            });
            if is_known {
                match kind {
                    Kind::Place if out.place.is_none() => out.place = Some(value),
                    Kind::Goal if out.goal.is_none() => out.goal = Some(value),
                    Kind::Page if out.page.is_none() => out.page = Some(value),
                    Kind::Label if !out.labels.contains(&value) => out.labels.push(value),
                    _ => {}
                }
            }
            i = last + 1;
            continue;
        }
        if let Some(p) = priority_word(w.text) {
            out.tokens.push(Token {
                kind: Kind::Priority,
                text: w.text.to_string(),
                value: p.to_string(),
                start: w.start,
                end: w.end,
                known: true,
            });
            out.priority = Some(p);
            i += 1;
            continue;
        }
        if out.due.is_none()
            && let Some(today) = today
        {
            {
                let mut found = date_phrase(&lower[i..], today).map(|(d, n)| (Some(d), n));
                if found.is_none() && time_word(&lower[i]).is_some() {
                    found = Some((None, 0));
                }
                if let Some((day, n)) = found {
                    let mut used = n;
                    let time = lower.get(i + used).and_then(|t| time_word(t));
                    if time.is_some() {
                        used += 1;
                    }
                    let (date, time) = match day {
                        Some(d) => (fmt_date(d), time),
                        None => (fmt_date(today), time_word(&lower[i])),
                    };
                    if day.is_none() {
                        used = 1;
                    }
                    let end = ws[i + used - 1].end;
                    let value = match &time {
                        Some(t) => format!("{date}T{t}"),
                        None => date.clone(),
                    };
                    out.tokens.push(Token {
                        kind: Kind::Date,
                        text: input[w.start..end].to_string(),
                        value,
                        start: w.start,
                        end,
                        known: true,
                    });
                    out.due = Some(Due { date, time });
                    i += used;
                    continue;
                }
            }
        }
        title.push(w.text.to_string());
        i += 1;
    }
    out.title = title.join(" ");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-19 is a Thursday.
    const NOW: &str = "2026-11-19T10:00";

    fn known() -> Known {
        Known {
            areas: vec!["Home".into(), "Side Project".into(), "Work".into()],
            goals: vec!["Get Fit".into()],
            pages: vec!["Trip Notes".into()],
            labels: vec!["errand".into(), "deep work".into()],
        }
    }

    fn p(input: &str) -> Parsed {
        parse(input, NOW, &known())
    }

    fn due(input: &str) -> Option<(String, Option<String>)> {
        p(input).due.map(|d| (d.date, d.time))
    }

    fn d(date: &str, time: Option<&str>) -> Option<(String, Option<String>)> {
        Some((date.into(), time.map(Into::into)))
    }

    #[test]
    fn plain_text_is_just_a_title() {
        let r = p("  buy   milk ");
        assert_eq!(r.title, "buy milk");
        assert!(r.tokens.is_empty() && r.due.is_none() && r.priority.is_none());
    }

    #[test]
    fn place_token_resolves_known_area_with_its_casing() {
        let r = p("fix tap @home");
        assert_eq!(r.title, "fix tap");
        assert_eq!(r.place.as_deref(), Some("Home"));
        let t = &r.tokens[0];
        assert_eq!(
            (t.kind, t.text.as_str(), t.value.as_str()),
            (Kind::Place, "@home", "Home")
        );
        assert_eq!((t.start, t.end, t.known), (8, 13, true));
    }

    #[test]
    fn multi_word_names_match_longest() {
        let r = p("ship it @side project now");
        assert_eq!(r.place.as_deref(), Some("Side Project"));
        assert_eq!(r.title, "ship it now");
        let r = p("#deep work today");
        assert_eq!(r.labels, vec!["deep work"]);
        assert_eq!(r.due.unwrap().date, "2026-11-19");
    }

    #[test]
    fn goal_and_page_and_label_tokens() {
        let r = p("run $get fit %trip notes #errand");
        assert_eq!(r.title, "run");
        assert_eq!(r.goal.as_deref(), Some("Get Fit"));
        assert_eq!(r.page.as_deref(), Some("Trip Notes"));
        assert_eq!(r.labels, vec!["errand"]);
        let kinds: Vec<_> = r.tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![Kind::Goal, Kind::Page, Kind::Label]);
    }

    #[test]
    fn unknown_names_are_unresolved_single_word_tokens() {
        let r = p("call @nobody now");
        assert_eq!(r.title, "call now");
        assert_eq!(r.place, None);
        let t = &r.tokens[0];
        assert_eq!((t.value.as_str(), t.known), ("nobody", false));
        for s in ["$x", "%x", "#x"] {
            assert!(!p(s).tokens[0].known);
        }
    }

    #[test]
    fn known_name_must_end_at_a_word_boundary() {
        let r = p("@homework");
        assert!(!r.tokens[0].known);
        assert_eq!(r.tokens[0].value, "homework");
    }

    #[test]
    fn priority_last_wins_and_p5_stays_text() {
        let r = p("a p1 b p3");
        assert_eq!(r.priority, Some(3));
        assert_eq!(r.title, "a b");
        let r = p("task P2");
        assert_eq!(r.priority, Some(2));
        let r = p("task p5 p0 p12");
        assert_eq!(r.priority, None);
        assert_eq!(r.title, "task p5 p0 p12");
    }

    #[test]
    fn relative_days() {
        assert_eq!(due("x today"), d("2026-11-19", None));
        assert_eq!(due("x tomorrow"), d("2026-11-20", None));
        assert_eq!(due("x in 3 days"), d("2026-11-22", None));
        assert_eq!(due("x in 1 day"), d("2026-11-20", None));
        assert_eq!(due("x in 2 weeks"), d("2026-12-03", None));
        assert_eq!(due("x in many days"), None);
    }

    #[test]
    fn next_week_is_next_monday() {
        assert_eq!(due("x next week"), d("2026-11-23", None));
        assert_eq!(p("x next week").title, "x");
        assert_eq!(due("x next"), None);
    }

    #[test]
    fn weekdays_mean_the_next_occurrence_and_abbreviate() {
        assert_eq!(due("x fri"), d("2026-11-20", None));
        assert_eq!(due("x friday"), d("2026-11-20", None));
        assert_eq!(due("x Mon"), d("2026-11-23", None));
        // Today is Thursday: the same weekday means a week out.
        assert_eq!(due("x thu"), d("2026-11-26", None));
        assert_eq!(due("x thursday"), d("2026-11-26", None));
    }

    #[test]
    fn month_names_need_a_day_to_be_a_date() {
        let r = p("may the force be with you");
        assert_eq!(r.due, None);
        assert_eq!(r.title, "may the force be with you");
        assert_eq!(due("x may 5"), d("2027-05-05", None));
        assert_eq!(due("x 5 may"), d("2027-05-05", None));
        assert_eq!(due("x Dec 25"), d("2026-12-25", None));
        assert_eq!(due("x 19 nov"), d("2026-11-19", None));
        assert_eq!(due("x feb 30"), None);
        assert_eq!(p("x may 5").title, "x");
    }

    #[test]
    fn bare_numbers_are_not_dates() {
        let r = p("buy 5 apples");
        assert_eq!(r.due, None);
        assert_eq!(r.title, "buy 5 apples");
        assert_eq!(due("x 12"), None);
        assert_eq!(due("x 5 apples"), None);
    }

    #[test]
    fn iso_dates() {
        assert_eq!(due("x 2026-12-01"), d("2026-12-01", None));
        assert_eq!(due("x 2026-13-01"), None);
        assert_eq!(due("x 2026-02-29"), None);
        assert_eq!(due("x 2028-02-29"), d("2028-02-29", None));
    }

    #[test]
    fn times_attach_to_a_date_or_mean_today() {
        assert_eq!(due("x 6pm"), d("2026-11-19", Some("18:00")));
        assert_eq!(due("x 6:30pm"), d("2026-11-19", Some("18:30")));
        assert_eq!(due("x 18:00"), d("2026-11-19", Some("18:00")));
        assert_eq!(due("x 12am"), d("2026-11-19", Some("00:00")));
        assert_eq!(due("x 12pm"), d("2026-11-19", Some("12:00")));
        assert_eq!(due("x tomorrow 6pm"), d("2026-11-20", Some("18:00")));
        assert_eq!(due("x fri 18:00"), d("2026-11-20", Some("18:00")));
        assert_eq!(due("x dec 1 9:15am"), d("2026-12-01", Some("09:15")));
        assert_eq!(due("x 25:00"), None);
        assert_eq!(due("x 13pm"), None);
        assert_eq!(due("x 6:75pm"), None);
        let r = p("x tomorrow 6pm");
        assert_eq!(r.title, "x");
        assert_eq!(r.tokens[0].value, "2026-11-20T18:00");
    }

    #[test]
    fn first_date_wins_and_later_ones_stay_in_title() {
        let r = p("x tomorrow friday");
        assert_eq!(r.due.unwrap().date, "2026-11-20");
        assert_eq!(r.title, "x friday");
        assert_eq!(r.tokens.len(), 1);
    }

    #[test]
    fn combinations_report_byte_offsets() {
        let input = "pay rent @work #errand p2 tomorrow 9am";
        let r = p(input);
        assert_eq!(r.title, "pay rent");
        assert_eq!(r.place.as_deref(), Some("Work"));
        assert_eq!(r.labels, vec!["errand"]);
        assert_eq!(r.priority, Some(2));
        assert_eq!(
            r.due,
            Some(Due {
                date: "2026-11-20".into(),
                time: Some("09:00".into())
            })
        );
        for t in &r.tokens {
            assert_eq!(&input[t.start..t.end], t.text);
        }
        assert_eq!(r.tokens.len(), 4);
    }

    #[test]
    fn backslash_keeps_a_sigil_literal() {
        let r = p("issue \\#1 and \\@home");
        assert_eq!(r.title, "issue #1 and @home");
        assert!(r.tokens.is_empty() && r.place.is_none());
        assert_eq!(p("\\#1").title, "#1");
    }

    #[test]
    fn reminders_and_recurrence_stay_in_title() {
        let r = p("water plants every day !9am");
        assert_eq!(r.title, "water plants every day !9am");
        assert!(r.tokens.is_empty());
        let r = p("x every! day");
        assert_eq!(r.title, "x every! day");
    }

    #[test]
    fn lone_sigils_stay_text() {
        assert_eq!(p("a # b @ c").title, "a # b @ c");
    }

    #[test]
    fn bad_now_disables_dates_only() {
        let r = parse("x tomorrow p1", "nonsense", &known());
        assert_eq!(r.due, None);
        assert_eq!(r.title, "x tomorrow");
        assert_eq!(r.priority, Some(1));
    }

    #[test]
    fn year_rolls_over_for_relative_dates() {
        let r = parse("x in 2 weeks", "2026-12-25T08:00", &known());
        assert_eq!(r.due.unwrap().date, "2027-01-08");
    }

    #[test]
    fn parsed_serializes_with_lowercase_kinds() {
        let s = serde_json::to_string(&p("#errand")).unwrap();
        assert!(s.contains("\"kind\":\"label\""));
    }
}
