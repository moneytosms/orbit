//! Quick-add parser. Date crate: none, hand-rolled. `interim` (chrono) does build for
//! wasm32, but it parses only whole strings (no phrase spans), rejects `in 3 days`, reads
//! `may` and a bare `5` as dates, and never rolls past dates forward, so it was dropped.
use crate::recur::{self, Recurrence};
use crate::reminder::{Anchor, Trigger};
use crate::when::{duration, range, when, when_date};
use serde::{Deserialize, Serialize};

pub const TITLE_MAX: usize = 120;

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
    Reminder,
    /// `{march 30}` or `by friday`.
    Deadline,
    /// `for 1h`, or a time range such as `3-4pm`.
    Duration,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: Kind,
    /// As typed, including the sigil.
    pub text: String,
    /// The name, "1".."4", `YYYY-MM-DD` (`YYYY-MM-DDTHH:MM` when a time was given; always a
    /// bare `YYYY-MM-DD` for a deadline) or, for a duration, the minutes ("90").
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
    #[serde(default)]
    pub description: String,
    pub tokens: Vec<Token>,
    pub priority: Option<u8>,
    pub due: Option<Due>,
    pub place: Option<String>,
    pub goal: Option<String>,
    pub page: Option<String>,
    pub labels: Vec<String>,
    pub recurrence: Option<Recurrence>,
    /// From `!` tokens; the app gives each an id.
    #[serde(default)]
    pub reminders: Vec<Trigger>,
    /// `YYYY-MM-DD`, from `{<date>}` or `by <date>`. Never sets the due date.
    #[serde(default)]
    pub deadline: Option<String>,
    /// Minutes (1 to 1440), from `for <duration>` or a time range.
    #[serde(default)]
    pub duration_min: Option<u32>,
}

/// Capture-only title cleanup. Existing stored task titles are never passed through this.
pub fn normalize_capture(input: &str, existing_description: &str) -> (String, String) {
    let raw = input.trim();
    let mut description = if existing_description.is_empty() {
        Vec::new()
    } else {
        vec![existing_description.to_string()]
    };
    let mut title = raw.to_string();

    if let Some(url) = bare_url(raw) {
        push_description(&mut description, url.to_string());
        title = readable_url(url);
    } else {
        let mut labels = Vec::new();
        let mut urls = Vec::new();
        let mut rest = raw;
        while let Some(open) = rest.find('[') {
            let after_open = &rest[open + 1..];
            let Some(close) = after_open.find("](") else {
                break;
            };
            let label = &after_open[..close];
            let after_label = &after_open[close + 2..];
            let Some(end) = after_label.find(')') else {
                break;
            };
            let url = &after_label[..end];
            if url.starts_with("http://") || url.starts_with("https://") {
                labels.push(label.to_string());
                urls.push(url.to_string());
                rest = &after_label[end + 1..];
            } else {
                rest = after_label;
            }
        }
        if !urls.is_empty() {
            title = labels.first().cloned().unwrap_or_default();
            if title.trim().is_empty() {
                title = readable_url(&urls[0]);
            }
            for url in urls {
                push_description(&mut description, url);
            }
        }
    }

    let original_title = title.clone();
    title = strip_block_marker(&title).to_string();
    title = title.replace("**", "").replace(['*', '`'], "");
    title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title != original_title.trim() && !description.iter().any(|line| line == raw) {
        description.push(raw.to_string());
    }
    if title.chars().count() > TITLE_MAX {
        if !description.iter().any(|d| d == raw) {
            description.push(raw.to_string());
        }
        title = cap_title(&title);
    }
    if title.trim().is_empty() {
        title = description
            .first()
            .and_then(|d| bare_url(d))
            .map(readable_url)
            .filter(|title| !title.is_empty())
            .or_else(|| {
                let fallback: String = raw.chars().take(60).collect();
                (!fallback.trim().is_empty()).then_some(fallback)
            })
            .unwrap_or_default();
    }
    (title, description.join("\n"))
}

fn bare_url(input: &str) -> Option<&str> {
    let rest = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    (!authority.is_empty() && !input.chars().any(char::is_whitespace)).then_some(input)
}

fn push_description(description: &mut Vec<String>, value: String) {
    if !description
        .iter()
        .any(|line| line.lines().any(|existing| existing == value))
    {
        description.push(value);
    }
}

fn readable_url(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..host_end]
        .strip_prefix("www.")
        .unwrap_or(&rest[..host_end]);
    let path = rest[host_end..]
        .strip_prefix('/')
        .unwrap_or(&rest[host_end..]);
    let segment = path.split(['/', '?', '#']).next().unwrap_or("");
    let mut title = if segment.is_empty() {
        host.to_string()
    } else {
        format!("{host}/{segment}")
    };
    if path.contains(['/', '?', '#']) {
        title.push('…');
    }
    cap_title(&title)
}

fn strip_block_marker(input: &str) -> &str {
    let mut rest = input.trim_start();
    loop {
        let mut stripped = false;
        for marker in ["###", "##", "#", "-", "*", ">", "1."] {
            if let Some(next) = rest.strip_prefix(marker)
                && next.chars().next().is_some_and(char::is_whitespace)
            {
                rest = next.trim_start();
                stripped = true;
                break;
            }
        }
        if !stripped {
            return rest;
        }
    }
}

fn only_leading_block_markers(input: &str) -> bool {
    let mut rest = input.trim_start();
    while !rest.is_empty() {
        let Some(next) = ["###", "##", "#", "-", "*", ">", "1."]
            .iter()
            .find_map(|marker| rest.strip_prefix(marker))
            .filter(|next| next.chars().next().is_some_and(char::is_whitespace))
        else {
            return false;
        };
        rest = next.trim_start();
    }
    true
}

fn cap_title(input: &str) -> String {
    if input.chars().count() <= TITLE_MAX {
        return input.to_string();
    }
    let limit = TITLE_MAX - 1;
    let prefix: String = input.chars().take(limit).collect();
    let boundary = prefix.rfind(char::is_whitespace).unwrap_or(prefix.len());
    format!("{}…", prefix[..boundary].trim_end())
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
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
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

pub(crate) fn days_in_month(y: i64, m: i64) -> i64 {
    days_from_civil(
        if m == 12 { y + 1 } else { y },
        if m == 12 { 1 } else { m + 1 },
        1,
    ) - days_from_civil(y, m, 1)
}

pub(crate) fn fmt_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Day number and minutes since midnight of a local `YYYY-MM-DDTHH:MM`.
fn parse_now(now: &str) -> Option<i64> {
    let (date, _) = now.split_once('T')?;
    parse_ymd(date)
}

pub(crate) fn parse_ymd(s: &str) -> Option<i64> {
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
pub(crate) fn weekday(days: i64) -> i64 {
    (days + 3).rem_euclid(7)
}

pub(crate) fn weekday_name(w: &str) -> Option<i64> {
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

pub(crate) fn month_name(w: &str) -> Option<i64> {
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

pub(crate) fn day_number(w: &str) -> Option<i64> {
    let w = w
        .strip_suffix("st")
        .or_else(|| w.strip_suffix("nd"))
        .or_else(|| w.strip_suffix("rd"))
        .or_else(|| w.strip_suffix("th"))
        .unwrap_or(w);
    if w.is_empty() || w.len() > 2 || !w.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    w.parse().ok()
}

/// Next occurrence of month/day on or after `today`.
pub(crate) fn month_day(today: i64, m: i64, d: i64) -> Option<i64> {
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
pub(crate) fn date_phrase(w: &[String], today: i64) -> Option<(i64, usize)> {
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

#[derive(Clone, Copy)]
enum SpanUnit {
    Days,
    Weeks,
    Months,
    Years,
}

/// The bounds that may follow a recurrence rule, in any order: `starting|from <date>`,
/// `until|ending|till|through <date>` and `for <n> days|weeks|months|years`. Sets them on `rec`
/// (or `span`, resolved once the first due is known) and returns the words used.
fn recurrence_bounds(
    w: &[String],
    today: i64,
    rec: &mut Recurrence,
    span: &mut Option<(i64, SpanUnit)>,
) -> usize {
    let mut used = 0;
    while let Some(word) = w.get(used) {
        let rest = &w[used + 1..];
        match word.as_str() {
            "starting" | "from" | "until" | "ending" | "till" | "through" if !rest.is_empty() => {
                let Some((day, n)) = date_phrase(rest, today) else {
                    break;
                };
                let date = fmt_date(day);
                if matches!(word.as_str(), "starting" | "from") {
                    rec.start = Some(date);
                } else {
                    rec.end = Some(date);
                }
                used += 1 + n;
            }
            "for" if rest.len() >= 2 => {
                let unit = match rest[1].as_str() {
                    "day" | "days" => SpanUnit::Days,
                    "week" | "weeks" => SpanUnit::Weeks,
                    "month" | "months" => SpanUnit::Months,
                    "year" | "years" => SpanUnit::Years,
                    _ => break,
                };
                let Some(n) = rest[0]
                    .parse::<i64>()
                    .ok()
                    .filter(|n| (1..=999).contains(n))
                else {
                    break;
                };
                *span = Some((n, unit));
                used += 3;
            }
            _ => break,
        }
    }
    used
}

/// `6pm`, `6:30pm`, `18:00` as `HH:MM`.
pub(crate) fn time_word(w: &str) -> Option<String> {
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

pub(crate) fn priority_word(w: &str) -> Option<u8> {
    match w.to_ascii_lowercase().as_str() {
        "p1" => Some(1),
        "p2" => Some(2),
        "p3" => Some(3),
        "p4" => Some(4),
        _ => None,
    }
}

/// A `!` reminder at the start of `w`: the trigger, the token value (minutes, or
/// `YYYY-MM-DDTHH:MM`) and the words used.
fn reminder_token(w: &[String], now: &str, today: Option<i64>) -> Option<(Trigger, String, usize)> {
    let rest = &w[0][1..];
    if let Some(n) = rest
        .strip_suffix(['m', 'h', 'd'])
        .filter(|n| !n.is_empty() && n.len() <= 5 && n.bytes().all(|c| c.is_ascii_digit()))
        .and_then(|n| n.parse::<u32>().ok())
    {
        let unit = match rest.as_bytes()[rest.len() - 1] {
            b'm' => 1,
            b'h' => 60,
            _ => 1440,
        };
        let minutes = n * unit;
        return Some((
            Trigger::Before {
                field: Anchor::Due,
                minutes,
            },
            minutes.to_string(),
            1,
        ));
    }
    let today = today?;
    let mut ph: Vec<String> = vec![rest.to_string()];
    ph.extend(w[1..].iter().cloned());
    let found = when(&ph, today, now_minutes(now), false)?;
    let at = format!("{}T{}", fmt_date(found.day), found.time?);
    Some((Trigger::At { at: at.clone() }, at, found.used))
}

/// Minutes since midnight of a local `YYYY-MM-DDTHH:MM`; 0 when there is no time.
fn now_minutes(now: &str) -> u32 {
    let h = now.get(11..13).and_then(|h| h.parse::<u32>().ok());
    let m = now.get(14..16).and_then(|m| m.parse::<u32>().ok());
    h.zip(m).map_or(0, |(h, m)| h.min(23) * 60 + m.min(59))
}

/// A `{<date>}` / `by <date>` deadline or a `for <duration>` / time range duration at word `i`:
/// the kind, token value and words used. Only the first of each is read, and `by`/`for` count
/// only when what follows really parses, so `written by Tom` and `for the team` stay title text.
/// A range also remembers its start in `range_start` for the due time.
fn deadline_or_duration(
    input: &str,
    ws: &[Word],
    lower: &[String],
    i: usize,
    (today, now_min): (Option<i64>, u32),
    (want_deadline, want_duration): (bool, bool),
    range_start: &mut Option<String>,
) -> Option<(Kind, String, usize)> {
    let day = |d: i64| fmt_date(d);
    let w = lower[i].as_str();
    if want_deadline && let Some(today) = today {
        if w.starts_with('{')
            && let Some(j) = (i..ws.len().min(i + 8)).find(|&j| ws[j].text.ends_with('}'))
        {
            let inner = input[ws[i].start + 1..ws[j].end - 1].to_lowercase();
            let words: Vec<String> = inner.split_whitespace().map(str::to_string).collect();
            if !words.is_empty()
                && let Some(f) = when_date(&words, today, now_min)
                && f.used == words.len()
            {
                return Some((Kind::Deadline, day(f.day), j + 1 - i));
            }
        }
        if w == "by"
            && i + 1 < lower.len()
            && let Some(f) = when_date(&lower[i + 1..], today, now_min)
        {
            return Some((Kind::Deadline, day(f.day), 1 + f.used));
        }
    }
    if want_duration {
        if w == "for"
            && i + 1 < lower.len()
            && let Some((mins, used)) = duration(&lower[i + 1..])
        {
            return Some((Kind::Duration, mins.to_string(), 1 + used));
        }
        if range_start.is_none()
            && let Some(r) = range(&lower[i..])
        {
            *range_start = Some(r.start);
            return Some((Kind::Duration, r.mins.to_string(), r.used));
        }
    }
    None
}

pub fn parse(input: &str, now: &str, known: &Known) -> Parsed {
    let ws = words(input);
    let lower: Vec<String> = ws.iter().map(|w| w.text.to_lowercase()).collect();
    let today = parse_now(now);
    let now_min = now_minutes(now);
    let mut out = Parsed {
        title: String::new(),
        description: String::new(),
        tokens: Vec::new(),
        priority: None,
        due: None,
        place: None,
        goal: None,
        page: None,
        labels: Vec::new(),
        recurrence: None,
        reminders: Vec::new(),
        deadline: None,
        duration_min: None,
    };
    let mut range_start: Option<String> = None;
    let mut rec_time: Option<String> = None;
    let mut rec_span: Option<(i64, SpanUnit)> = None;
    let mut title: Vec<String> = Vec::new();
    // Words after `every`/`every!` never become a due date.
    let mut recurring = false;
    let mut i = 0;
    while i < ws.len() {
        let w = &ws[i];
        let first = w.text.chars().next().unwrap_or(' ');
        if matches!(w.text, "##" | "###")
            && only_leading_block_markers(&input[..w.start])
            && input[w.end..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            title.push(w.text.to_string());
            i += 1;
            continue;
        }
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
        if first == '!'
            && w.text.len() > 1
            && let Some((trigger, value, used)) = reminder_token(&lower[i..], now, today)
        {
            let end = ws[i + used - 1].end;
            out.tokens.push(Token {
                kind: Kind::Reminder,
                text: input[w.start..end].to_string(),
                value,
                start: w.start,
                end,
                known: true,
            });
            out.reminders.push(trigger);
            i += used;
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
        if let Some((kind, value, used)) = deadline_or_duration(
            input,
            &ws,
            &lower,
            i,
            (today, now_min),
            (out.deadline.is_none(), out.duration_min.is_none()),
            &mut range_start,
        ) {
            let end = ws[i + used - 1].end;
            out.tokens.push(Token {
                kind,
                text: input[w.start..end].to_string(),
                value: value.clone(),
                start: w.start,
                end,
                known: true,
            });
            match kind {
                Kind::Deadline => out.deadline = Some(value),
                _ => out.duration_min = value.parse().ok(),
            }
            i += used;
            continue;
        }
        if matches!(lower[i].as_str(), "every" | "every!") {
            recurring = true;
            if out.recurrence.is_none()
                && let Some((rule, time, used)) = recur::parse_rule(&lower[i + 1..])
            {
                let mut rec = Recurrence::new(rule, lower[i] == "every!");
                i += 1 + used;
                if let Some(today) = today {
                    i += recurrence_bounds(&lower[i..], today, &mut rec, &mut rec_span);
                }
                out.recurrence = Some(rec);
                rec_time = time;
                continue;
            }
        }
        if out.due.is_none()
            && !recurring
            && let Some(today) = today
            && let Some(found) = when(&lower[i..], today, now_min, true)
        {
            {
                let date = fmt_date(found.day);
                let end = ws[i + found.used - 1].end;
                let value = match &found.time {
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
                out.due = Some(Due {
                    date,
                    time: found.time,
                });
                i += found.used;
                continue;
            }
        }
        title.push(w.text.to_string());
        i += 1;
    }
    let (normalized, description) = normalize_capture(&title.join(" "), "");
    out.title = normalized;
    out.description = description;
    if let (Some(rec), Some(today)) = (&mut out.recurrence, today) {
        // A series starting later first lands on or after its start.
        let from = rec
            .start
            .as_deref()
            .and_then(parse_ymd)
            .map_or(today, |s| s.max(today));
        match &mut out.due {
            Some(due) => due.time = due.time.take().or(rec_time),
            None => {
                if let Some(day) = recur::first_day(&rec.rule, from) {
                    out.due = Some(Due {
                        date: fmt_date(day),
                        time: rec_time,
                    });
                }
            }
        }
        // `for 3 weeks` counts from the first occurrence.
        if let Some((n, unit)) = rec_span {
            let first = out
                .due
                .as_ref()
                .and_then(|d| parse_ymd(&d.date))
                .unwrap_or(from);
            let end = match unit {
                SpanUnit::Days => first + n,
                SpanUnit::Weeks => first + 7 * n,
                SpanUnit::Months => recur::add_months(first, n),
                SpanUnit::Years => recur::add_months(first, 12 * n),
            } - 1;
            rec.end.get_or_insert_with(|| fmt_date(end));
        }
    }
    if let (Some(start), Some(today)) = (range_start, today) {
        match &mut out.due {
            Some(due) => {
                due.time.get_or_insert(start);
            }
            None => {
                let at = format!("{:02}:{:02}", now_min / 60, now_min % 60);
                out.due = Some(Due {
                    date: fmt_date(today + i64::from(start <= at)),
                    time: Some(start),
                });
            }
        }
    }
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
    fn relative_times_count_from_now() {
        assert_eq!(due("x in 5 minutes"), d("2026-11-19", Some("10:05")));
        assert_eq!(due("x in 5 min"), d("2026-11-19", Some("10:05")));
        assert_eq!(due("x in 5m"), d("2026-11-19", Some("10:05")));
        assert_eq!(due("x in 2 hours"), d("2026-11-19", Some("12:00")));
        assert_eq!(due("x in an hour"), d("2026-11-19", Some("11:00")));
        assert_eq!(due("x in half an hour"), d("2026-11-19", Some("10:30")));
        assert_eq!(
            due("x in an hour and a half"),
            d("2026-11-19", Some("11:30"))
        );
        assert_eq!(due("x in 1.5 hours"), d("2026-11-19", Some("11:30")));
        assert_eq!(due("x in 1h30m"), d("2026-11-19", Some("11:30")));
        assert_eq!(
            due("x in 1 hour 15 minutes"),
            d("2026-11-19", Some("11:15"))
        );
        assert_eq!(
            due("x in 2 hours and 5 minutes"),
            d("2026-11-19", Some("12:05"))
        );
        assert_eq!(due("x in 20 hours"), d("2026-11-20", Some("06:00")));
        assert_eq!(due("x 90 minutes from now"), d("2026-11-19", Some("11:30")));
        assert_eq!(due("x 10 minutes later"), d("2026-11-19", Some("10:10")));
        assert_eq!(due("x in 2 months"), d("2027-01-19", None));
        assert_eq!(due("x in a week"), d("2026-11-26", None));
        assert_eq!(due("x in 5"), None);
        assert_eq!(due("x 5 minutes"), None);
        let r = p("call mom in 5 minutes p1");
        assert_eq!(r.title, "call mom");
        assert_eq!(r.tokens[0].text, "in 5 minutes");
        assert_eq!(r.tokens[0].value, "2026-11-19T10:05");
    }

    #[test]
    fn at_and_day_parts() {
        assert_eq!(due("x today at 10pm"), d("2026-11-19", Some("22:00")));
        assert_eq!(due("x tomorrow at 9am"), d("2026-11-20", Some("09:00")));
        assert_eq!(due("x today at 9am"), d("2026-11-19", Some("09:00")));
        assert_eq!(due("x at 6pm"), d("2026-11-19", Some("18:00")));
        assert_eq!(due("x at noon"), d("2026-11-19", Some("12:00")));
        assert_eq!(due("x tomorrow at noon"), d("2026-11-20", Some("12:00")));
        assert_eq!(due("x fri at midnight"), d("2026-11-20", Some("00:00")));
        assert_eq!(due("x fri 7 pm"), d("2026-11-20", Some("19:00")));
        assert_eq!(due("x 9am fri"), d("2026-11-20", Some("09:00")));
        assert_eq!(due("x tonight"), d("2026-11-19", Some("20:00")));
        assert_eq!(due("x tonight 9pm"), d("2026-11-19", Some("21:00")));
        assert_eq!(due("x tomorrow morning"), d("2026-11-20", Some("09:00")));
        assert_eq!(due("x tmrw evening"), d("2026-11-20", Some("18:00")));
        assert_eq!(due("x this afternoon"), d("2026-11-19", Some("14:00")));
        assert_eq!(due("x in the evening"), d("2026-11-19", Some("18:00")));
        assert_eq!(due("x in the morning"), d("2026-11-20", Some("09:00")));
        assert_eq!(p("meet at the cafe").due, None);
        assert_eq!(p("meet at the cafe").title, "meet at the cafe");
        assert_eq!(p("x today at 10pm").title, "x");
        assert_eq!(due("x on friday"), d("2026-11-20", None));
        assert_eq!(p("pay rent on friday").title, "pay rent");
    }

    #[test]
    fn richer_dates() {
        assert_eq!(due("x next friday"), d("2026-11-27", None));
        assert_eq!(due("x next tuesday"), d("2026-11-24", None));
        assert_eq!(due("x this friday"), d("2026-11-20", None));
        assert_eq!(due("x this thursday"), d("2026-11-19", None));
        assert_eq!(due("x next month"), d("2026-12-19", None));
        assert_eq!(due("x next year"), d("2027-01-01", None));
        assert_eq!(due("x end of month"), d("2026-11-30", None));
        assert_eq!(due("x eom"), d("2026-11-30", None));
        assert_eq!(due("x mid january"), d("2027-01-15", None));
        assert_eq!(due("x this weekend"), d("2026-11-21", None));
        assert_eq!(due("x next weekend"), d("2026-11-28", None));
        assert_eq!(due("x on the 27th"), d("2026-11-27", None));
        assert_eq!(due("x the 5th"), d("2026-12-05", None));
        assert_eq!(due("x dec 5th"), d("2026-12-05", None));
        assert_eq!(due("x 5th dec"), d("2026-12-05", None));
        assert_eq!(due("x dec 5 2027"), d("2027-12-05", None));
        assert_eq!(due("x the 5th of dec"), d("2026-12-05", None));
        assert_eq!(due("x 27/1"), d("2027-01-27", None));
        assert_eq!(due("x 3/12"), d("2026-12-03", None));
        assert_eq!(due("x 1/27"), d("2027-01-27", None));
        assert_eq!(due("x 3/12/2027"), d("2027-12-03", None));
        assert_eq!(due("x 2027/01/05"), d("2027-01-05", None));
        assert_eq!(due("x tmr"), d("2026-11-20", None));
        assert_eq!(due("x tomorrow 6pm"), d("2026-11-20", Some("18:00")));
        // A bare ordinal or number is not a date.
        assert_eq!(due("x 27th"), None);
        assert_eq!(due("x the 32nd"), None);
        assert_eq!(due("x end of"), None);
    }

    #[test]
    fn reminders_take_the_same_phrases() {
        let at = |s: &str| match &p(s).reminders[..] {
            [Trigger::At { at }] => at.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(at("x !in 10 minutes"), "2026-11-19T10:10");
        assert_eq!(at("x !tomorrow morning"), "2026-11-20T09:00");
        assert_eq!(at("x !tonight"), "2026-11-19T20:00");
        // A date after a reminder time stays the due date.
        let r = p("x !9am friday");
        assert_eq!(r.due.map(|d| d.date), Some("2026-11-20".into()));
        assert_eq!(r.title, "x");
    }

    #[test]
    fn every_accepts_at_before_the_time() {
        let r = p("standup every weekday at 9am");
        assert_eq!(r.title, "standup");
        assert_eq!(
            r.due.as_ref().and_then(|d| d.time.clone()),
            Some("09:00".into())
        );
        let r = p("x every mon,wed at 18:30");
        assert_eq!(r.title, "x");
    }

    #[test]
    fn bang_tokens_parse_offsets_and_absolutes() {
        let r = p("call mom !30m !1h !1d");
        assert_eq!(r.title, "call mom");
        let mins: Vec<_> = r
            .reminders
            .iter()
            .map(|t| match t {
                Trigger::Before { minutes, .. } => *minutes,
                _ => 0,
            })
            .collect();
        assert_eq!(mins, [30, 60, 1440]);
        let at = |s: &str| match &p(s).reminders[..] {
            [Trigger::At { at }] => at.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(at("x !14:00"), "2026-11-19T14:00");
        assert_eq!(at("x !9am"), "2026-11-20T09:00");
        assert_eq!(at("x !fri 9am"), "2026-11-20T09:00");
        assert_eq!(at("x !tomorrow 9am"), "2026-11-20T09:00");
        let r = p("x !fri 9am");
        assert_eq!(
            (r.title.as_str(), r.tokens[0].text.as_str()),
            ("x", "!fri 9am")
        );
        assert_eq!(r.tokens[0].kind, Kind::Reminder);
    }

    #[test]
    fn unresolved_bang_stays_in_the_title() {
        let r = p("wow !nope !fri");
        assert_eq!(r.title, "wow !nope !fri");
        assert!(r.reminders.is_empty() && r.tokens.is_empty());
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
    fn richer_recurrence_rules() {
        let due_of = |r: &Parsed| r.due.clone().map(|x| (x.date, x.time));
        let rule = |s: &str| p(s).recurrence.map(|r| r.rule);
        let r = p("water every other fri");
        assert_eq!(r.title, "water");
        assert_eq!(
            r.recurrence.clone().map(|r| r.rule),
            Some("2 weeks on fri".into())
        );
        assert_eq!(due_of(&r), d("2026-11-20", None));
        assert_eq!(rule("x every other day"), Some("2 days".into()));
        assert_eq!(rule("x every quarter"), Some("3 months".into()));
        assert_eq!(rule("x quarterly"), None);
        assert_eq!(rule("x every 3 workdays"), Some("3 workdays".into()));
        let r = p("x every 2nd wed");
        assert_eq!(r.recurrence.clone().map(|r| r.rule), Some("2nd wed".into()));
        assert_eq!(due_of(&r), d("2026-12-09", None));
        assert_eq!(r.title, "x");
        assert_eq!(due_of(&p("x every last fri")), d("2026-11-27", None));
        assert_eq!(due_of(&p("x every first workday")), d("2026-12-01", None));
        let r = p("x every last workday at 9am");
        assert_eq!(
            r.recurrence.clone().map(|r| r.rule),
            Some("last workday".into())
        );
        assert_eq!(due_of(&r), d("2026-11-30", Some("09:00")));
        // A plain ordinal is still the day of the month.
        assert_eq!(rule("x every 2nd"), Some("2nd".into()));
    }

    #[test]
    fn recurrence_start_and_end() {
        let due_of = |r: &Parsed| r.due.clone().map(|x| (x.date, x.time));
        let r = p("x every day until dec 25");
        assert_eq!(r.title, "x");
        let rec = r.recurrence.clone().unwrap();
        assert_eq!(rec.end.as_deref(), Some("2026-12-25"));
        assert_eq!(rec.start, None);
        assert_eq!(due_of(&r), d("2026-11-19", None));

        let r = p("x every week starting dec 1");
        assert_eq!(
            r.recurrence.clone().unwrap().start.as_deref(),
            Some("2026-12-01")
        );
        assert_eq!(due_of(&r), d("2026-12-01", None));

        let r = p("x every mon from dec 1 ending dec 31 p1");
        let rec = r.recurrence.clone().unwrap();
        assert_eq!(
            (rec.start.as_deref(), rec.end.as_deref()),
            (Some("2026-12-01"), Some("2026-12-31"))
        );
        assert_eq!(due_of(&r), d("2026-12-07", None));
        assert_eq!(r.title, "x");
        assert_eq!(r.priority, Some(1));

        let r = p("x every day for 3 weeks");
        assert_eq!(
            r.recurrence.clone().unwrap().end.as_deref(),
            Some("2026-12-09")
        );
        assert_eq!(r.title, "x");
        let r = p("x every week for 2 months");
        assert_eq!(
            r.recurrence.clone().unwrap().end.as_deref(),
            Some("2027-01-18")
        );
        // Month-end clamping: Jan 31 + 1 month is Feb 28, so the last day is Feb 27.
        let r = p("x every day starting 2027-01-31 for 1 month");
        assert_eq!(
            r.recurrence.clone().unwrap().end.as_deref(),
            Some("2027-02-27")
        );
        let r = p("x every year for 1 year");
        assert_eq!(
            r.recurrence.clone().unwrap().end.as_deref(),
            Some("2027-11-18")
        );

        // Words that are not a bound stay in the title; `every` never makes a due date.
        let r = p("x every day from home");
        assert_eq!(r.title, "x from home");
        assert_eq!(r.recurrence.clone().unwrap().end, None);
        let r = p("x every day for fun");
        assert_eq!(r.title, "x for fun");
        let r = p("x every day until");
        assert_eq!(r.title, "x until");
        // Without the keyword after a rule nothing is read as a bound.
        assert_eq!(p("x until dec 25").recurrence, None);
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
        // A bare time that has already passed today means tomorrow.
        assert_eq!(due("x 12am"), d("2026-11-20", Some("00:00")));
        assert_eq!(due("x 10am"), d("2026-11-20", Some("10:00")));
        assert_eq!(due("x 10:01am"), d("2026-11-19", Some("10:01")));
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
    fn recurrence_with_a_reminder_token() {
        let r = p("water plants every day !9am");
        assert_eq!(r.title, "water plants");
        assert_eq!(r.reminders.len(), 1);
        assert_eq!(
            r.due,
            Some(Due {
                date: "2026-11-19".into(),
                time: None
            })
        );
        let rec = |rule: &str, from_completion| Some(Recurrence::new(rule, from_completion));
        assert_eq!(r.recurrence, rec("day", false));
        let r = p("x every! day");
        assert_eq!(r.title, "x");
        assert_eq!(r.recurrence, rec("day", true));
        let r = p("standup every monday 9am p2");
        assert_eq!(r.title, "standup");
        assert_eq!(r.recurrence, rec("mon", false));
        assert_eq!(
            (r.due, r.priority),
            (
                Some(Due {
                    date: "2026-11-23".into(),
                    time: Some("09:00".into())
                }),
                Some(2)
            )
        );
        // Words after the rule are not a due date.
        let r = p("x every week tomorrow");
        assert_eq!(r.title, "x tomorrow");
        // A date before `every` still counts.
        assert_eq!(due("x tomorrow every week"), d("2026-11-20", None));
        assert_eq!(
            due("x tomorrow every day 9am"),
            d("2026-11-20", Some("09:00"))
        );
        // Not a rule: stays in the title, and still no due date from what follows.
        let r = p("go every which way tomorrow");
        assert_eq!(
            (r.title.as_str(), r.recurrence, r.due),
            ("go every which way tomorrow", None, None)
        );
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

    #[test]
    fn capture_normalization_strips_markdown_and_preserves_source() {
        let r = p("[Notion – all-in-one](https://notion.so/workspace?ref=abc)");
        assert_eq!(r.title, "Notion – all-in-one");
        assert_eq!(r.description, "https://notion.so/workspace?ref=abc");
        assert_eq!(
            normalize_capture(&r.title, &r.description),
            (r.title, r.description)
        );
    }

    #[test]
    fn capture_normalization_keeps_first_label_and_all_link_urls() {
        let (title, description) = normalize_capture(
            "[First](https://one.test/a) then [Second](https://two.test/b)",
            "",
        );
        assert_eq!(title, "First");
        assert_eq!(description, "https://one.test/a\nhttps://two.test/b");
    }

    #[test]
    fn capture_links_without_labels_remain_readable_and_nonempty() {
        for input in [
            "[](https://notion.so/8-Weeks-to-Build/project?ref=abc)",
            "https://youtube.com/playlist?list=abc",
        ] {
            let r = p(input);
            assert!(!r.title.is_empty());
            assert!(
                r.title.starts_with("notion.so/8-Weeks-to-Build")
                    || r.title == "youtube.com/playlist…"
            );
            assert_eq!(r.description, input.trim_matches(['[', ']', '(', ')']));
            assert_eq!(
                normalize_capture(&r.title, &r.description),
                (r.title, r.description)
            );
        }
    }

    #[test]
    fn capture_normalization_only_shortens_url_only_input() {
        let input = "https://example.test/task and remember the context";
        let r = p(input);
        assert_eq!(r.title, input);
        assert!(r.description.is_empty());
    }

    #[test]
    fn capture_normalization_strips_nested_block_markers_idempotently() {
        for input in ["> # title", "- > ## title", "### - `title`"] {
            let r = p(input);
            assert_eq!(r.title, "title");
            assert_eq!(r.description, input);
            assert_eq!(
                normalize_capture(&r.title, &r.description),
                (r.title, r.description)
            );
        }
    }

    #[test]
    fn empty_markdown_label_uses_its_url_and_preserves_existing_description() {
        let url = "https://links.test/first/path?ref=abc";
        let (title, description) = normalize_capture(&format!("[]({url})"), "existing notes");
        assert_eq!(title, "links.test/first…");
        assert_eq!(description, format!("existing notes\n{url}"));

        let (_, description) = normalize_capture(&format!("[]({url})"), url);
        assert_eq!(description, url);
    }

    #[test]
    fn capture_normalization_strips_block_and_inline_markers() {
        let r = p("## **Scope discipline** `keep` this");
        assert_eq!(r.title, "Scope discipline keep this");
        assert_eq!(r.description, "## **Scope discipline** `keep` this");
        assert_eq!(p("### heading").title, "heading");
    }

    #[test]
    fn capture_titles_cap_at_word_boundary_and_preserve_full_text() {
        let input = format!("{} final", "word ".repeat(TITLE_MAX));
        let r = p(&input);
        assert!(r.title.chars().count() <= TITLE_MAX);
        assert!(r.title.ends_with('…'));
        assert_eq!(
            r.description,
            input.split_whitespace().collect::<Vec<_>>().join(" ")
        );
    }

    #[test]
    fn capture_normalization_keeps_plain_text_and_has_an_empty_fallback() {
        assert_eq!(p("buy milk").title, "buy milk");
        assert_eq!(p("   ").title, "");
        let (title, _) = normalize_capture("[]()", "");
        assert_eq!(title, "[]()");
        assert!(!normalize_capture("https://", "").0.is_empty());
    }

    fn dl(input: &str) -> Option<String> {
        p(input).deadline
    }

    fn dur(input: &str) -> Option<u32> {
        p(input).duration_min
    }

    #[test]
    fn braces_set_a_deadline_and_never_the_due_date() {
        let r = p("file taxes {march 30} p2");
        assert_eq!(r.title, "file taxes");
        assert_eq!(r.deadline.as_deref(), Some("2027-03-30"));
        assert_eq!(r.due, None);
        assert_eq!(r.tokens[0].kind, Kind::Deadline);
        assert_eq!(r.tokens[0].text, "{march 30}");
        assert_eq!(r.tokens[0].value, "2027-03-30");
        assert_eq!(dl("x {next friday}").as_deref(), Some("2026-11-27"));
        assert_eq!(dl("x {2026-12-24}").as_deref(), Some("2026-12-24"));
        assert_eq!(dl("x {tomorrow}").as_deref(), Some("2026-11-20"));
        assert_eq!(dl("x {in 3 days}").as_deref(), Some("2026-11-22"));
        assert_eq!(dl("x {end of month}").as_deref(), Some("2026-11-30"));
        // A time is dropped: Task.deadline is a bare date.
        assert_eq!(dl("x {friday 5pm}").as_deref(), Some("2026-11-20"));
        // Unparsable braces stay in the title.
        let r = p("use {foo} here");
        assert_eq!(r.title, "use {foo} here");
        assert_eq!(r.deadline, None);
        assert_eq!(p("x {}").title, "x {}");
        assert_eq!(p("x {5pm}").deadline, None);
        assert_eq!(p("x {friday").title, "x {friday");
        // Braces and a due date together.
        let r = p("report tomorrow {dec 5}");
        assert_eq!(r.due.map(|d| d.date).as_deref(), Some("2026-11-20"));
        assert_eq!(r.deadline.as_deref(), Some("2026-12-05"));
        assert_eq!(r.title, "report");
        // Only the first deadline counts.
        assert_eq!(dl("x {dec 5} {dec 6}").as_deref(), Some("2026-12-05"));
    }

    #[test]
    fn by_sets_a_deadline_only_when_a_date_follows() {
        let r = p("send report by friday");
        assert_eq!(r.title, "send report");
        assert_eq!(r.deadline.as_deref(), Some("2026-11-20"));
        assert_eq!(r.due, None);
        assert_eq!(r.tokens[0].text, "by friday");
        assert_eq!(dl("x by dec 5").as_deref(), Some("2026-12-05"));
        assert_eq!(dl("x by end of month").as_deref(), Some("2026-11-30"));
        assert_eq!(dl("x by next friday").as_deref(), Some("2026-11-27"));
        assert_eq!(dl("x by tomorrow").as_deref(), Some("2026-11-20"));
        assert_eq!(dl("x by the 15th").as_deref(), Some("2026-12-15"));
        assert_eq!(dl("x by 2026-12-01").as_deref(), Some("2026-12-01"));
        assert_eq!(dl("x by in 2 days").as_deref(), Some("2026-11-21"));
        for text in [
            "book written by Tom",
            "stand by me",
            "drop by",
            "pay by card",
            "x by 5pm",
            "x by may smith",
        ] {
            let r = p(text);
            assert_eq!(r.deadline, None, "{text}");
        }
        assert_eq!(p("book written by Tom").title, "book written by Tom");
        assert_eq!(p("stand by me").title, "stand by me");
        // `by` plus a date, then a due date elsewhere.
        let r = p("report by friday today");
        assert_eq!(r.deadline.as_deref(), Some("2026-11-20"));
        assert_eq!(r.due.map(|d| d.date).as_deref(), Some("2026-11-19"));
        // Only the first deadline counts; the second date becomes the due date.
        let r = p("x by friday by monday");
        assert_eq!(r.deadline.as_deref(), Some("2026-11-20"));
        assert_eq!(r.title, "x by");
        assert_eq!(r.due.map(|d| d.date).as_deref(), Some("2026-11-23"));
    }

    #[test]
    fn for_sets_a_duration_only_when_a_length_follows() {
        let r = p("write essay for 1h tomorrow");
        assert_eq!(r.title, "write essay");
        assert_eq!(r.duration_min, Some(60));
        assert_eq!(r.tokens[0].kind, Kind::Duration);
        assert_eq!(r.tokens[0].text, "for 1h");
        assert_eq!(r.tokens[0].value, "60");
        assert_eq!(dur("x for 30 min"), Some(30));
        assert_eq!(dur("x for 30m"), Some(30));
        assert_eq!(dur("x for 1h15m"), Some(75));
        assert_eq!(dur("x for 1 hour 15 minutes"), Some(75));
        assert_eq!(dur("x for 1 hour and 15 minutes"), Some(75));
        assert_eq!(dur("x for an hour"), Some(60));
        assert_eq!(dur("x for half an hour"), Some(30));
        assert_eq!(dur("x for an hour and a half"), Some(90));
        assert_eq!(dur("x for 90 minutes"), Some(90));
        assert_eq!(dur("x for 1.5 hours"), Some(90));
        assert_eq!(dur("x for 24h"), Some(1440));
        assert_eq!(dur("x for 1 minute"), Some(1));
        for text in [
            "x for 25h",
            "x for 0m",
            "x for 3 days",
            "x for 2 weeks",
            "x for 1h and 2 days",
            "x for the team",
            "x for 5",
            "x for",
            "gift for Anna",
        ] {
            assert_eq!(dur(text), None, "{text}");
        }
        assert_eq!(p("gift for Anna").title, "gift for Anna");
        assert_eq!(p("x for 25h").title, "x for 25h");
        // A duration does not touch the due date.
        let r = p("x 3pm for 45 min");
        assert_eq!(r.due.map(|d| d.time), Some(Some("15:00".into())));
        assert_eq!(r.duration_min, Some(45));
        let r = p("x in 1 hour for 30 min");
        assert_eq!(r.due.map(|d| d.time), Some(Some("11:00".into())));
        assert_eq!(r.duration_min, Some(30));
        // Only the first counts.
        assert_eq!(dur("x for 1h for 2h"), Some(60));
    }

    #[test]
    fn time_ranges_set_the_due_time_and_duration() {
        let r = p("standup tomorrow 3-4pm");
        assert_eq!(r.title, "standup");
        assert_eq!(r.duration_min, Some(60));
        let due = r.due.unwrap();
        assert_eq!(
            (due.date.as_str(), due.time.as_deref()),
            ("2026-11-20", Some("15:00"))
        );
        assert_eq!(r.tokens.last().unwrap().kind, Kind::Duration);
        assert_eq!(r.tokens.last().unwrap().text, "3-4pm");
        let range = |t: &str| {
            let r = p(t);
            (r.due.and_then(|d| d.time), r.duration_min)
        };
        let some = |t: &str, m| (Some(t.to_string()), Some(m));
        assert_eq!(range("x 9:30-10:00"), some("09:30", 30));
        assert_eq!(range("x 9am-10:30am"), some("09:00", 90));
        assert_eq!(range("x 11-1pm"), some("11:00", 120));
        assert_eq!(range("x 9-10pm"), some("21:00", 60));
        assert_eq!(range("x 3pm-4"), some("15:00", 60));
        assert_eq!(range("x from 9 to 10:30"), some("09:00", 90));
        assert_eq!(range("x from 9am to 5pm"), some("09:00", 480));
        assert_eq!(range("x from 9 am to 10 am"), some("09:00", 60));
        assert_eq!(range("x from 14:00 until 15:15"), some("14:00", 75));
        // Date after the range, and an explicit due time wins.
        let r = p("x 3-4pm friday");
        assert_eq!(r.due.as_ref().map(|d| d.date.as_str()), Some("2026-11-20"));
        assert_eq!(r.due.and_then(|d| d.time).as_deref(), Some("15:00"));
        let r = p("x tomorrow 9am 3-4pm");
        assert_eq!(r.due.and_then(|d| d.time).as_deref(), Some("09:00"));
        assert_eq!(r.duration_min, Some(60));
        // Today when still ahead, else tomorrow.
        let r = p("x 3-4pm");
        assert_eq!(r.due.map(|d| d.date).as_deref(), Some("2026-11-19"));
        let r = p("x 8-9am");
        assert_eq!(r.due.map(|d| d.date).as_deref(), Some("2026-11-20"));
        // Not ranges: bare numbers, backwards, across midnight, dates.
        for text in [
            "x 9-10",
            "x 11pm-1am",
            "x 2026-11-19",
            "x pages 3-4",
            "x from 9 to",
        ] {
            let r = p(text);
            assert_eq!(r.duration_min, None, "{text}");
        }
        assert_eq!(p("x pages 3-4").title, "x pages 3-4");
        // `for` wins over a later range; the range then stays text.
        assert_eq!(dur("x for 1h 3-4pm"), Some(60));
    }

    #[test]
    fn deadline_and_duration_work_with_other_sigils_and_recurrence() {
        let r = p("review @Work p1 by friday for 1h #errand tomorrow");
        assert_eq!(r.title, "review");
        assert_eq!(r.place.as_deref(), Some("Work"));
        assert_eq!(r.priority, Some(1));
        assert_eq!(r.deadline.as_deref(), Some("2026-11-20"));
        assert_eq!(r.duration_min, Some(60));
        assert_eq!(r.labels, vec!["errand".to_string()]);
        assert_eq!(r.due.map(|d| d.date).as_deref(), Some("2026-11-20"));
        // Words after `every` stay recurrence-only, but `for` still reads.
        let r = p("water plants every monday for 15 min");
        assert_eq!(r.title, "water plants");
        assert!(r.recurrence.is_some());
        assert_eq!(r.duration_min, Some(15));
        let r = p("standup every weekday 9:00-9:15");
        assert!(r.recurrence.is_some());
        assert_eq!(r.duration_min, Some(15));
        assert_eq!(r.due.and_then(|d| d.time).as_deref(), Some("09:00"));
        let r = p("pay rent every month by the 1st");
        assert!(r.recurrence.is_some());
        assert_eq!(r.deadline.as_deref(), Some("2026-12-01"));
        // A reminder keeps its own words.
        let r = p("call !9am friday by monday");
        assert_eq!(r.reminders.len(), 1);
        assert_eq!(r.deadline.as_deref(), Some("2026-11-23"));
    }

    #[test]
    fn old_parsed_json_loads_without_deadline_or_duration() {
        let r: Parsed = serde_json::from_str(
            r#"{"title":"x","tokens":[],"priority":null,"due":null,"place":null,"goal":null,"page":null,"labels":[],"recurrence":null}"#,
        )
        .unwrap();
        assert_eq!((r.deadline, r.duration_min), (None, None));
        let kinds = serde_json::to_string(&[Kind::Deadline, Kind::Duration]).unwrap();
        assert_eq!(kinds, r#"["deadline","duration"]"#);
    }
}
