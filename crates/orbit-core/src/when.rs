//! Natural-language "when" phrases for quick add: relative times (`in 5 minutes`,
//! `in 1h30m`, `in half an hour`), `at` times (`today at 10pm`, `at noon`), day parts
//! (`tomorrow morning`, `tonight`, `in the evening`) and richer dates (`next friday`,
//! `end of month`, `the 15th`, `27/1`, `dec 5th 2027`). Hand-rolled like the rest of the
//! parser so every phrase reports the words it used (the Composer shows them as chips).
//! Numeric dates read day/month; `3/12` is 3 December.
use crate::parse::{
    civil_from_days, date_phrase, day_number, days_from_civil, days_in_month, month_day,
    month_name, time_word, weekday, weekday_name,
};
use crate::recur::add_months;

pub(crate) struct When {
    pub day: i64,
    pub time: Option<String>,
    /// Words used from the start of the phrase.
    pub used: usize,
}

fn s(w: &[String], i: usize) -> &str {
    w.get(i).map(String::as_str).unwrap_or("")
}

/// The phrase at the start of `w` (lowercased words). A phrase with a time and no date means
/// the next time the clock shows it. `trailing_date` allows `9am friday`; reminders turn it off
/// so `!9am friday` stays a reminder plus a due date.
pub(crate) fn when(w: &[String], today: i64, now_min: u32, trailing_date: bool) -> Option<When> {
    relative(w, today, now_min)
        .or_else(|| dated(w, today))
        .or_else(|| timed(w, today, now_min, trailing_date))
}

#[derive(Clone, Copy, PartialEq)]
enum Unit {
    Min,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

fn unit(u: &str) -> Option<Unit> {
    Some(match u {
        "m" | "min" | "mins" | "minute" | "minutes" => Unit::Min,
        "h" | "hr" | "hrs" | "hour" | "hours" => Unit::Hour,
        "d" | "day" | "days" => Unit::Day,
        "w" | "wk" | "wks" | "week" | "weeks" => Unit::Week,
        "mo" | "month" | "months" => Unit::Month,
        "y" | "yr" | "yrs" | "year" | "years" => Unit::Year,
        _ => return None,
    })
}

#[derive(Default)]
struct Dur {
    months: i64,
    days: i64,
    mins: i64,
    timed: bool,
}

impl Dur {
    /// `tenths` is the quantity times ten, so `1.5 hours` works.
    fn add(&mut self, tenths: i64, u: Unit) -> Option<()> {
        let whole = tenths % 10 == 0;
        match u {
            Unit::Min if whole => self.mins += tenths / 10,
            Unit::Hour => self.mins += tenths * 6,
            Unit::Day if whole => self.days += tenths / 10,
            Unit::Week if whole => self.days += 7 * tenths / 10,
            Unit::Month if whole => self.months += tenths / 10,
            Unit::Year if whole => self.months += 12 * tenths / 10,
            _ => return None,
        }
        if matches!(u, Unit::Min | Unit::Hour) {
            self.timed = true;
        }
        (self.mins <= 1_000_000 && self.days <= 99_999 && self.months <= 99_999).then_some(())
    }
}

fn tenths(w: &str) -> Option<i64> {
    let (int, frac) = w.split_once('.').unwrap_or((w, "0"));
    let ok = |p: &str| !p.is_empty() && p.len() <= 5 && p.bytes().all(|c| c.is_ascii_digit());
    if !ok(int) || frac.len() != 1 || !ok(frac) {
        return None;
    }
    Some(int.parse::<i64>().ok()? * 10 + frac.parse::<i64>().ok()?)
}

/// `1h30m`, `90min`, `2h`: one word of number-unit pairs.
fn compact(w: &str) -> Option<Dur> {
    let mut d = Dur::default();
    let mut rest = w;
    let mut any = false;
    while !rest.is_empty() {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let letters = rest[digits..]
            .bytes()
            .take_while(u8::is_ascii_lowercase)
            .count();
        if digits == 0 || digits > 5 || letters == 0 {
            return None;
        }
        d.add(
            rest[..digits].parse::<i64>().ok()? * 10,
            unit(&rest[digits..digits + letters])?,
        )?;
        rest = &rest[digits + letters..];
        any = true;
    }
    any.then_some(d)
}

/// One quantity at `i`: `5 minutes`, `an hour`, `half an hour`, `1.5 hours`, `1h30m`.
fn component(w: &[String], i: usize, into: &mut Dur) -> Option<(Unit, usize)> {
    let first = s(w, i);
    match first {
        "half" => {
            let skip = usize::from(matches!(s(w, i + 1), "a" | "an"));
            if unit(s(w, i + 1 + skip))? != Unit::Hour {
                return None;
            }
            into.add(5, Unit::Hour)?;
            return Some((Unit::Hour, 2 + skip));
        }
        "a" | "an" => {
            let u = s(w, i + 1);
            let u = if u.len() > 1 { unit(u)? } else { return None };
            into.add(10, u)?;
            return Some((u, 2));
        }
        _ => {}
    }
    if let Some(q) = tenths(first) {
        let u = unit(s(w, i + 1))?;
        into.add(q, u)?;
        return Some((u, 2));
    }
    let c = compact(first)?;
    into.mins += c.mins;
    into.days += c.days;
    into.months += c.months;
    into.timed |= c.timed;
    Some((Unit::Min, 1))
}

fn relative(w: &[String], today: i64, now_min: u32) -> Option<When> {
    let (mut i, prefixed) = if s(w, 0) == "in" {
        (1, true)
    } else {
        (0, false)
    };
    let mut d = Dur::default();
    let mut parts = 0;
    while let Some((last, used)) = component(w, i, &mut d) {
        i += used;
        parts += 1;
        if last == Unit::Hour && (s(w, i), s(w, i + 1), s(w, i + 2)) == ("and", "a", "half") {
            d.mins += 30;
            i += 3;
            break;
        }
        let j = i + usize::from(s(w, i) == "and");
        if component(w, j, &mut Dur::default()).is_none() {
            break;
        }
        i = j;
    }
    if parts == 0 {
        return None;
    }
    if !prefixed {
        if s(w, i) == "from" && matches!(s(w, i + 1), "now" | "today") {
            i += 2;
        } else if s(w, i) == "later" {
            i += 1;
        } else {
            return None;
        }
    }
    let mut day = if d.months != 0 {
        add_months(today, d.months)
    } else {
        today
    };
    day += d.days;
    let mut time = None;
    if d.timed {
        let t = i64::from(now_min) + d.mins;
        day += t.div_euclid(1440);
        let m = t.rem_euclid(1440);
        time = Some(format!("{:02}:{:02}", m / 60, m % 60));
    }
    Some(When { day, time, used: i })
}

fn daypart(w: &str) -> Option<&'static str> {
    Some(match w {
        "morning" => "09:00",
        "afternoon" => "14:00",
        "evening" => "18:00",
        "night" => "21:00",
        _ => return None,
    })
}

/// A time of day at `i`: `6pm`, `at 18:00`, `6 pm`, `at noon`, `morning` (after a date) or
/// `in the evening`. Returns the time and the words used.
fn time_slot(w: &[String], i: usize, after_date: bool) -> Option<(String, usize)> {
    let at = s(w, i) == "at";
    let j = i + usize::from(at);
    let in_the = s(w, j) == "in" && s(w, j + 1) == "the";
    let k = j + if in_the { 2 } else { 0 };
    let word = s(w, k);
    if let Some(t) = daypart(word) {
        return (in_the || (after_date && !at)).then(|| (t.to_string(), k + 1 - i));
    }
    if in_the {
        return None;
    }
    if let Some(t) = time_word(word) {
        return Some((t, k + 1 - i));
    }
    if matches!(s(w, k + 1), "am" | "pm")
        && let Some(t) = time_word(&format!("{word}{}", s(w, k + 1)))
    {
        return Some((t, k + 2 - i));
    }
    if (at || after_date) && matches!(word, "noon" | "midnight") {
        let t = if word == "noon" { "12:00" } else { "00:00" };
        return Some((t.to_string(), k + 1 - i));
    }
    None
}

fn dated(w: &[String], today: i64) -> Option<When> {
    let on = s(w, 0) == "on";
    let (day, n, default) = date_ext(&w[usize::from(on)..], today, on)?;
    let i = usize::from(on) + n;
    Some(match time_slot(w, i, true) {
        Some((t, u)) => When {
            day,
            time: Some(t),
            used: i + u,
        },
        None => When {
            day,
            time: default.map(str::to_string),
            used: i,
        },
    })
}

fn timed(w: &[String], today: i64, now_min: u32, trailing_date: bool) -> Option<When> {
    let (t, u) = time_slot(w, 0, false)?;
    if trailing_date {
        let on = s(w, u) == "on";
        let at = u + usize::from(on);
        if let Some((day, n, _)) = date_ext(&w[at..], today, on) {
            return Some(When {
                day,
                time: Some(t),
                used: at + n,
            });
        }
    }
    let now = format!("{:02}:{:02}", now_min / 60, now_min % 60);
    let day = today + i64::from(t <= now);
    Some(When {
        day,
        time: Some(t),
        used: u,
    })
}

fn weekday_of(w: &str) -> Option<i64> {
    weekday_name(w)
}

fn exact(y: i64, m: i64, d: i64) -> Option<i64> {
    ((1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m)).then(|| days_from_civil(y, m, d))
}

fn year4(w: &str) -> Option<i64> {
    (w.len() == 4 && w.bytes().all(|c| c.is_ascii_digit()))
        .then(|| w.parse().ok())
        .flatten()
        .filter(|y| (1970..=2999).contains(y))
}

/// `m`/`d` with an optional year word after it at `i`; returns the day and words used there.
fn month_day_year(w: &[String], i: usize, today: i64, m: i64, d: i64) -> Option<(i64, usize)> {
    if let Some(y) = year4(s(w, i)) {
        return Some((exact(y, m, d)?, 1));
    }
    Some((month_day(today, m, d)?, 0))
}

/// The next day on or after `today` that is the `d`th of a month.
fn next_ordinal(today: i64, d: i64) -> Option<i64> {
    let (y, m, _) = civil_from_days(today);
    (0..14).find_map(|k| {
        let idx = y * 12 + (m - 1) + k;
        let (y, m) = (idx.div_euclid(12), idx.rem_euclid(12) + 1);
        exact(y, m, d).filter(|t| *t >= today)
    })
}

fn slash_date(w: &str, today: i64) -> Option<i64> {
    let parts: Vec<&str> = w.split('/').collect();
    if !(2..=3).contains(&parts.len())
        || parts
            .iter()
            .any(|p| p.is_empty() || p.len() > 4 || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let n: Vec<i64> = parts.iter().filter_map(|p| p.parse().ok()).collect();
    if parts[0].len() == 4 {
        return (n.len() == 3).then(|| exact(n[0], n[1], n[2])).flatten();
    }
    // Day/month; `1/27` can only be month/day, so it is read that way.
    let (d, m) = if n[1] > 12 && n[0] <= 12 {
        (n[1], n[0])
    } else {
        (n[0], n[1])
    };
    match n.get(2) {
        None => month_day(today, m, d),
        Some(&y) if parts[2].len() == 4 => exact(y, m, d),
        Some(&y) if parts[2].len() == 2 => exact(2000 + y, m, d),
        _ => None,
    }
}

/// A date at the start of `w`: the day, the words used and a default time some phrases carry
/// (`tonight`, `this morning`). `after_on` lets a bare ordinal (`27th`) count as a date.
fn date_ext(
    w: &[String],
    today: i64,
    after_on: bool,
) -> Option<(i64, usize, Option<&'static str>)> {
    if w.is_empty() {
        return None;
    }
    let wd = weekday(today);
    let first = s(w, 0);
    let second = s(w, 1);
    match first {
        "tmr" | "tmrw" => return Some((today + 1, 1, None)),
        "tonight" => return Some((today, 1, Some("20:00"))),
        "eom" => return Some((month_end(today), 1, None)),
        "end" if second == "of" => {
            let skip = usize::from(s(w, 2) == "the");
            if s(w, 2 + skip) == "month" {
                return Some((month_end(today), 3 + skip, None));
            }
        }
        "mid" => {
            if let Some(m) = month_name(second) {
                return Some((month_day(today, m, 15)?, 2, None));
            }
        }
        "next" => {
            if let Some(t) = weekday_of(second) {
                return Some((today + 7 - wd + t, 2, None));
            }
            match second {
                "month" => return Some((add_months(today, 1), 2, None)),
                "year" => {
                    let (y, _, _) = civil_from_days(today);
                    return Some((days_from_civil(y + 1, 1, 1), 2, None));
                }
                "weekend" => return Some((today + 7 - wd + 5, 2, None)),
                _ => {}
            }
        }
        "this" => {
            if let Some(t) = weekday_of(second) {
                return Some((today + (t - wd).rem_euclid(7), 2, None));
            }
            if second == "weekend" {
                return Some((today + if wd == 6 { 0 } else { 5 - wd }.max(0), 2, None));
            }
            if let Some(t) = daypart(second) {
                return Some((today, 2, Some(t)));
            }
        }
        "the" => {
            if let Some(d) = ordinal(second) {
                if s(w, 2) == "of"
                    && let Some(m) = month_name(s(w, 3))
                {
                    let (day, extra) = month_day_year(w, 4, today, m, d)?;
                    return Some((day, 4 + extra, None));
                }
                return Some((next_ordinal(today, d)?, 2, None));
            }
        }
        _ => {}
    }
    if let Some(day) = slash_date(first, today) {
        return Some((day, 1, None));
    }
    if let Some(m) = month_name(first)
        && let Some(d) = day_number(second)
    {
        let (day, extra) = month_day_year(w, 2, today, m, d)?;
        return Some((day, 2 + extra, None));
    }
    if let Some(d) = day_number(first)
        && let Some(m) = month_name(second)
    {
        let (day, extra) = month_day_year(w, 2, today, m, d)?;
        return Some((day, 2 + extra, None));
    }
    if after_on && let Some(d) = ordinal(first) {
        return Some((next_ordinal(today, d)?, 1, None));
    }
    date_phrase(w, today).map(|(d, n)| (d, n, None))
}

/// `27th`: a day number with its suffix, never a bare number.
fn ordinal(w: &str) -> Option<i64> {
    ["st", "nd", "rd", "th"]
        .iter()
        .any(|suffix| w.ends_with(suffix))
        .then(|| day_number(w))
        .flatten()
        .filter(|d| (1..=31).contains(d))
}

fn month_end(today: i64) -> i64 {
    let (y, m, _) = civil_from_days(today);
    days_from_civil(y, m, days_in_month(y, m))
}
