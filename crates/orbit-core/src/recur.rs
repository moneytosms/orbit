//! Recurrence rules: `every …` (from the due date) and `every! …` (from completion).
//! Dates are floating local, so a rule never touches the time of day and DST needs no handling.
use crate::Due;
use crate::parse::{
    civil_from_days, days_from_civil, days_in_month, fmt_date, parse_ymd, time_word, weekday,
    weekday_name,
};
use serde::{Deserialize, Serialize};

/// `rule` is normalised text, e.g. `day`, `3 weeks`, `mon,wed`, `15th`, `month end`.
///
/// `start` and `end` (`YYYY-MM-DD`, both optional, `end` inclusive) bound the series: completing
/// the occurrence after `end` finishes the Task instead of moving its due on.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Recurrence {
    pub rule: String,
    pub from_completion: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

impl Recurrence {
    pub fn new(rule: impl Into<String>, from_completion: bool) -> Self {
        Recurrence {
            rule: rule.into(),
            from_completion,
            ..Default::default()
        }
    }
}

#[derive(Debug, PartialEq)]
enum Rule {
    Days(i64),
    Weekdays,
    /// Every n-th working day (n >= 2).
    Workdays(i64),
    Weeks(i64),
    /// Every n-th week (n >= 2), on a weekday (0 = Monday).
    WeeksOn(i64, i64),
    /// Bit 0 = Monday.
    On(u8),
    Months(i64),
    DayOfMonth(i64),
    MonthEnd,
    /// The n-th (1..=5, or -1 for the last) weekday (0 = Monday) of the month.
    NthWeekday(i64, i64),
    /// The n-th (1..=23, or -1 for the last) working day of the month.
    NthWorkday(i64),
    Years(i64),
}

const SHORT: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

fn ordinal(n: i64) -> String {
    let s = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

fn nth_text(n: i64) -> String {
    if n < 0 { "last".into() } else { ordinal(n) }
}

/// `2nd`, `second` or `last` (-1).
fn nth_word(w: &str) -> Option<i64> {
    match w {
        "first" => Some(1),
        "second" => Some(2),
        "third" => Some(3),
        "fourth" => Some(4),
        "fifth" => Some(5),
        "last" => Some(-1),
        _ => ordinal_day(w),
    }
}

fn is_workday_word(w: &str) -> bool {
    matches!(w, "weekday" | "weekdays" | "workday" | "workdays")
}

fn every(n: i64, unit: &str) -> String {
    if n == 1 {
        unit.to_string()
    } else {
        format!("{n} {unit}s")
    }
}

impl Rule {
    fn text(&self) -> String {
        match self {
            Rule::Days(n) => every(*n, "day"),
            Rule::Weekdays => "weekday".into(),
            Rule::Workdays(n) => format!("{n} workdays"),
            Rule::Weeks(n) => every(*n, "week"),
            Rule::WeeksOn(n, wd) => format!("{n} weeks on {}", SHORT[*wd as usize]),
            Rule::NthWeekday(n, wd) => format!("{} {}", nth_text(*n), SHORT[*wd as usize]),
            Rule::NthWorkday(n) => format!("{} workday", nth_text(*n)),
            Rule::On(mask) => (0..7)
                .filter(|i| mask >> i & 1 == 1)
                .map(|i| SHORT[i])
                .collect::<Vec<_>>()
                .join(","),
            Rule::Months(n) => every(*n, "month"),
            Rule::DayOfMonth(d) => ordinal(*d),
            Rule::MonthEnd => "month end".into(),
            Rule::Years(n) => every(*n, "year"),
        }
    }
}

/// `mon`, `mon,tue` or `monday,` as a weekday bit mask.
fn day_list(w: &str) -> Option<u8> {
    let mut mask = 0;
    for part in w.trim_end_matches(',').split(',') {
        mask |= 1 << weekday_name(part)?;
    }
    Some(mask)
}

fn weekday_list(w: &[String]) -> Option<(u8, usize)> {
    let mut mask = 0;
    let mut used = 0;
    while let Some(m) = w.get(used).and_then(|x| day_list(x)) {
        mask |= m;
        used += 1;
        let and = w.get(used).is_some_and(|x| x == "and" || x == "&");
        if w[used - 1].ends_with(',') || and {
            let skip = and as usize;
            if w.get(used + skip).and_then(|x| day_list(x)).is_some() {
                used += skip;
                continue;
            }
        }
        break;
    }
    (used > 0).then_some((mask, used))
}

fn ordinal_day(w: &str) -> Option<i64> {
    let digits = w
        .strip_suffix("st")
        .or_else(|| w.strip_suffix("nd"))
        .or_else(|| w.strip_suffix("rd"))
        .or_else(|| w.strip_suffix("th"))?;
    if digits.is_empty() || digits.len() > 2 || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|d| (1..=31).contains(d))
}

/// The rule at the start of `w` (lowercased words after `every`): its words used, then an
/// optional trailing time. Returns the normalised rule, the time, and the total words used.
pub fn parse_rule(w: &[String]) -> Option<(String, Option<String>, usize)> {
    let (rule, time, used) = read_rule(w)?;
    Some((rule.text(), time, used))
}

fn read_rule(w: &[String]) -> Option<(Rule, Option<String>, usize)> {
    let first = w.first()?.as_str();
    let (rule, mut used) = match first {
        "day" | "daily" => (Rule::Days(1), 1),
        "weekday" | "weekdays" | "workday" | "workdays" => (Rule::Weekdays, 1),
        "week" | "weekly" => (Rule::Weeks(1), 1),
        "month" if w.get(1).map(String::as_str) == Some("end") => (Rule::MonthEnd, 2),
        "last" if w.get(1).map(String::as_str) == Some("day") => (Rule::MonthEnd, 2),
        "month" | "monthly" => (Rule::Months(1), 1),
        "year" | "yearly" => (Rule::Years(1), 1),
        "quarter" | "quarterly" => (Rule::Months(3), 1),
        "other" => {
            let unit = w.get(1)?.as_str();
            let rule = match unit {
                "day" => Rule::Days(2),
                "week" => Rule::Weeks(2),
                "month" => Rule::Months(2),
                "quarter" => Rule::Months(6),
                "year" => Rule::Years(2),
                u if is_workday_word(u) => Rule::Workdays(2),
                u => Rule::WeeksOn(2, weekday_name(u)?),
            };
            (rule, 2)
        }
        _ => {
            let nth = nth_word(first).zip(w.get(1).map(String::as_str));
            if let Some((n, next)) = nth
                && let Some(wd) = weekday_name(next).filter(|_| n == -1 || (1..=5).contains(&n))
            {
                (Rule::NthWeekday(n, wd), 2)
            } else if let Some((n, next)) = nth
                && is_workday_word(next)
                && (n == -1 || (1..=23).contains(&n))
            {
                (Rule::NthWorkday(n), 2)
            } else if let Some(d) = ordinal_day(first) {
                (Rule::DayOfMonth(d), 1)
            } else if let Some((mask, n)) = weekday_list(w) {
                (Rule::On(mask), n)
            } else {
                let n: i64 = first.parse().ok().filter(|n| (1..=999).contains(n))?;
                match w.get(1)?.as_str() {
                    "day" | "days" => (Rule::Days(n), 2),
                    "week" | "weeks" => match (w.get(2).map(String::as_str), w.get(3)) {
                        (Some("on"), Some(d)) if weekday_name(d).is_some() => {
                            let wd = weekday_name(d)?;
                            if n == 1 {
                                (Rule::On(1 << wd), 4)
                            } else {
                                (Rule::WeeksOn(n, wd), 4)
                            }
                        }
                        _ => (Rule::Weeks(n), 2),
                    },
                    "month" | "months" => (Rule::Months(n), 2),
                    "quarter" | "quarters" => (Rule::Months(3 * n), 2),
                    "year" | "years" => (Rule::Years(n), 2),
                    u if is_workday_word(u) => (
                        if n == 1 {
                            Rule::Weekdays
                        } else {
                            Rule::Workdays(n)
                        },
                        2,
                    ),
                    _ => return None,
                }
            }
        }
    };
    let at = usize::from(w.get(used).map(String::as_str) == Some("at"));
    let time = w.get(used + at).and_then(|t| time_word(t));
    if time.is_some() {
        used += 1 + at;
    }
    Some((rule, time, used))
}

fn stored(rule: &str) -> Option<Rule> {
    let words: Vec<String> = rule.split_whitespace().map(str::to_lowercase).collect();
    let (rule, _, used) = read_rule(&words)?;
    (used == words.len()).then_some(rule)
}

/// `days` moved `n` months on, clamped to the end of a shorter month.
pub(crate) fn add_months(days: i64, n: i64) -> i64 {
    let (y, m, d) = civil_from_days(days);
    let idx = y * 12 + (m - 1) + n;
    let (y, m) = (idx.div_euclid(12), idx.rem_euclid(12) + 1);
    days_from_civil(y, m, d.min(days_in_month(y, m)))
}

/// The `d`th of the month of `days` (clamped to its length), `months` months on.
fn day_in_month(days: i64, months: i64, d: i64) -> i64 {
    let (y, m, _) = civil_from_days(add_months(days - civil_from_days(days).2 + 1, months));
    days_from_civil(y, m, d.min(days_in_month(y, m)))
}

/// The (year, month) `n` months after that of `days`.
fn month_after(days: i64, n: i64) -> (i64, i64) {
    let (y, m, _) = civil_from_days(days);
    let idx = y * 12 + (m - 1) + n;
    (idx.div_euclid(12), idx.rem_euclid(12) + 1)
}

/// The `n`-th (or last, -1) weekday `wd` of a month, if it has one.
fn nth_weekday_of(y: i64, m: i64, n: i64, wd: i64) -> Option<i64> {
    let len = days_in_month(y, m);
    if n < 0 {
        let last = days_from_civil(y, m, len);
        return Some(last - (weekday(last) - wd).rem_euclid(7));
    }
    let first = days_from_civil(y, m, 1);
    let day = first + (wd - weekday(first)).rem_euclid(7) + 7 * (n - 1);
    (day < first + len).then_some(day)
}

/// The `n`-th (or last, -1) working day of a month.
fn nth_workday_of(y: i64, m: i64, n: i64) -> Option<i64> {
    let first = days_from_civil(y, m, 1);
    let len = days_in_month(y, m);
    if n < 0 {
        return (first..first + len).rev().find(|d| weekday(*d) < 5);
    }
    (first..first + len)
        .filter(|d| weekday(*d) < 5)
        .nth(n as usize - 1)
}

/// The first date strictly after `base` among the monthly picks.
fn monthly_after(base: i64, pick: impl Fn(i64, i64) -> Option<i64>) -> i64 {
    (0..48)
        .filter_map(|k| {
            let (y, m) = month_after(base, k);
            pick(y, m)
        })
        .find(|d| *d > base)
        .unwrap_or(base + 1)
}

/// The first date strictly after `base` that the rule lands on.
fn after(rule: &Rule, base: i64) -> i64 {
    match rule {
        Rule::Days(n) => base + n,
        Rule::Weeks(n) => base + 7 * n,
        Rule::WeeksOn(n, wd) => {
            let from = base + 7 * (n - 1);
            (from + 1..from + 8)
                .find(|d| weekday(*d) == *wd)
                .unwrap_or(from + 1)
        }
        Rule::NthWeekday(n, wd) => monthly_after(base, |y, m| nth_weekday_of(y, m, *n, *wd)),
        Rule::NthWorkday(n) => monthly_after(base, |y, m| nth_workday_of(y, m, *n)),
        Rule::Workdays(n) => {
            let mut d = base;
            for _ in 0..*n {
                d = (d + 1..).find(|d| weekday(*d) < 5).unwrap_or(d + 1);
            }
            d
        }
        Rule::Weekdays => (base + 1..).find(|d| weekday(*d) < 5).unwrap_or(base + 1),
        Rule::On(mask) => (base + 1..base + 8)
            .find(|d| mask >> weekday(*d) & 1 == 1)
            .unwrap_or(base + 1),
        Rule::Months(n) => add_months(base, *n),
        Rule::Years(n) => add_months(base, 12 * n),
        Rule::DayOfMonth(d) => {
            let this = day_in_month(base, 0, *d);
            if this > base {
                this
            } else {
                day_in_month(base, 1, *d)
            }
        }
        Rule::MonthEnd => {
            let this = day_in_month(base, 0, 31);
            if this > base {
                this
            } else {
                day_in_month(base, 1, 31)
            }
        }
    }
}

/// The first due date of a new recurring task: the first occurrence on or after `today`.
/// An interval rule ("every 3 days") starts today.
pub(crate) fn first_day(rule: &str, today: i64) -> Option<i64> {
    Some(match stored(rule)? {
        Rule::Days(_) | Rule::Weeks(_) | Rule::Months(_) | Rule::Years(_) => today,
        Rule::WeeksOn(_, wd) => (today..today + 7).find(|d| weekday(*d) == wd)?,
        Rule::Workdays(_) => (today..).find(|d| weekday(*d) < 5)?,
        r => after(&r, today - 1),
    })
}

/// The next due after completing a task, or `None` once the series has run past its `end`.
/// An unreadable rule or date leaves `due` unchanged.
pub fn advance(rec: &Recurrence, due: &Due, today: &str) -> Option<Due> {
    let base = if rec.from_completion {
        parse_ymd(today)
    } else {
        parse_ymd(&due.date).or_else(|| parse_ymd(today))
    };
    let (Some(rule), Some(base)) = (stored(&rec.rule), base) else {
        return Some(due.clone());
    };
    let day = after(&rule, base);
    if rec
        .end
        .as_deref()
        .and_then(parse_ymd)
        .is_some_and(|end| day > end)
    {
        return None;
    }
    Some(Due {
        date: fmt_date(day),
        time: due.time.clone(),
    })
}

/// The next due after completing a task: strictly after `due.date` for `every`, or after
/// `today` for `every!`. The time of day is kept as is. An unreadable rule or date, or a series
/// past its `end`, leaves `due` unchanged.
pub fn next(rec: &Recurrence, due: &Due, today: &str) -> Due {
    advance(rec, due, today).unwrap_or_else(|| due.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(rule: &str, from_completion: bool) -> Recurrence {
        Recurrence::new(rule, from_completion)
    }

    fn norm(text: &str) -> Option<String> {
        rule(text).map(|(r, _, _)| r)
    }

    /// Every rule's text re-reads as the same rule.
    fn round_trips(text: &str) -> String {
        let t = norm(text).unwrap_or_else(|| panic!("{text} should parse"));
        assert_eq!(norm(&t).as_deref(), Some(t.as_str()), "{text} -> {t}");
        assert_eq!(stored(&t).map(|r| r.text()).as_deref(), Some(t.as_str()));
        t
    }

    fn step(rule: &str, date: &str) -> String {
        let due = Due {
            date: date.into(),
            time: None,
        };
        next(&rec(rule, false), &due, "2000-01-01").date
    }

    fn rule(text: &str) -> Option<(String, Option<String>, usize)> {
        let w: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
        parse_rule(&w)
    }

    #[test]
    fn rules_normalise() {
        let r = |t: &str| rule(t).map(|(r, _, _)| r);
        assert_eq!(r("day"), Some("day".into()));
        assert_eq!(r("Daily"), Some("day".into()));
        assert_eq!(r("3 days"), Some("3 days".into()));
        assert_eq!(r("1 day"), Some("day".into()));
        assert_eq!(r("workday"), Some("weekday".into()));
        assert_eq!(r("weekly"), Some("week".into()));
        assert_eq!(r("2 weeks"), Some("2 weeks".into()));
        assert_eq!(r("Monday"), Some("mon".into()));
        assert_eq!(r("fri,mon"), Some("mon,fri".into()));
        assert_eq!(r("mon, wed and fri"), Some("mon,wed,fri".into()));
        assert_eq!(r("monthly"), Some("month".into()));
        assert_eq!(r("2 months"), Some("2 months".into()));
        assert_eq!(r("15th"), Some("15th".into()));
        assert_eq!(r("1st"), Some("1st".into()));
        assert_eq!(r("last day"), Some("month end".into()));
        assert_eq!(r("month end"), Some("month end".into()));
        assert_eq!(r("yearly"), Some("year".into()));
        assert_eq!(r("soon"), None);
        assert_eq!(r("3 apples"), None);
        assert_eq!(r("0 days"), None);
    }

    #[test]
    fn a_trailing_time_is_taken() {
        assert_eq!(
            rule("monday 9am p2"),
            Some(("mon".into(), Some("09:00".into()), 2))
        );
        assert_eq!(rule("day tomorrow"), Some(("day".into(), None, 1)));
        assert_eq!(
            rule("mon and tue 18:30"),
            Some(("mon,tue".into(), Some("18:30".into()), 4))
        );
        // A trailing connector with no day after it is left alone.
        assert_eq!(rule("mon and then"), Some(("mon".into(), None, 1)));
    }

    #[test]
    fn day_and_week_intervals() {
        assert_eq!(step("day", "2026-11-19"), "2026-11-20");
        assert_eq!(step("3 days", "2026-11-19"), "2026-11-22");
        assert_eq!(step("day", "2026-12-31"), "2027-01-01");
        assert_eq!(step("week", "2026-11-19"), "2026-11-26");
        assert_eq!(step("2 weeks", "2026-11-19"), "2026-12-03");
    }

    #[test]
    fn weekday_rule_skips_the_weekend() {
        // 2026-11-20 is a Friday.
        assert_eq!(step("weekday", "2026-11-19"), "2026-11-20");
        assert_eq!(step("weekday", "2026-11-20"), "2026-11-23");
        assert_eq!(step("weekday", "2026-11-21"), "2026-11-23");
        assert_eq!(step("weekday", "2026-11-22"), "2026-11-23");
    }

    #[test]
    fn named_weekdays() {
        assert_eq!(step("mon", "2026-11-19"), "2026-11-23");
        assert_eq!(step("mon", "2026-11-23"), "2026-11-30");
        assert_eq!(step("mon,wed,fri", "2026-11-20"), "2026-11-23");
        assert_eq!(step("mon,wed,fri", "2026-11-23"), "2026-11-25");
        assert_eq!(step("sat,sun", "2026-11-21"), "2026-11-22");
    }

    #[test]
    fn months_clamp_to_the_end() {
        assert_eq!(step("month", "2026-10-15"), "2026-11-15");
        assert_eq!(step("month", "2026-01-31"), "2026-02-28");
        assert_eq!(step("month", "2028-01-31"), "2028-02-29");
        assert_eq!(step("month", "2026-12-15"), "2027-01-15");
        assert_eq!(step("2 months", "2026-11-30"), "2027-01-30");
    }

    #[test]
    fn a_numbered_day_of_the_month() {
        assert_eq!(step("15th", "2026-11-15"), "2026-12-15");
        assert_eq!(step("15th", "2026-11-10"), "2026-11-15");
        assert_eq!(step("31st", "2026-11-15"), "2026-11-30");
        assert_eq!(step("31st", "2026-11-30"), "2026-12-31");
        assert_eq!(step("31st", "2026-01-31"), "2026-02-28");
        assert_eq!(step("30th", "2028-01-30"), "2028-02-29");
    }

    #[test]
    fn month_end() {
        assert_eq!(step("month end", "2026-01-31"), "2026-02-28");
        assert_eq!(step("month end", "2026-01-15"), "2026-01-31");
        assert_eq!(step("month end", "2028-02-15"), "2028-02-29");
        assert_eq!(step("month end", "2028-02-29"), "2028-03-31");
        assert_eq!(step("month end", "2026-12-31"), "2027-01-31");
    }

    #[test]
    fn years_and_leap_day() {
        assert_eq!(step("year", "2026-11-19"), "2027-11-19");
        assert_eq!(step("year", "2028-02-29"), "2029-02-28");
        assert_eq!(step("year", "2027-02-28"), "2028-02-28");
    }

    #[test]
    fn every_bang_counts_from_today_not_the_due_date() {
        let due = Due {
            date: "2026-11-01".into(),
            time: None,
        };
        assert_eq!(
            next(&rec("week", false), &due, "2026-11-19").date,
            "2026-11-08"
        );
        assert_eq!(
            next(&rec("week", true), &due, "2026-11-19").date,
            "2026-11-26"
        );
        assert_eq!(
            next(&rec("mon", true), &due, "2026-11-23").date,
            "2026-11-30"
        );
    }

    #[test]
    fn the_time_of_day_survives_dst_changes() {
        for (from, to) in [
            ("2026-03-07", "2026-03-08"),
            ("2026-03-08", "2026-03-09"),
            ("2026-10-31", "2026-11-01"),
            ("2026-11-01", "2026-11-02"),
        ] {
            let due = Due {
                date: from.into(),
                time: Some("09:00".into()),
            };
            let n = next(&rec("day", false), &due, from);
            assert_eq!((n.date.as_str(), n.time.as_deref()), (to, Some("09:00")));
        }
        let due = Due {
            date: "2026-03-01".into(),
            time: Some("09:00".into()),
        };
        let n = next(&rec("week", false), &due, "2026-03-01");
        assert_eq!(
            (n.date.as_str(), n.time.as_deref()),
            ("2026-03-08", Some("09:00"))
        );
    }

    #[test]
    fn a_new_task_starts_on_the_first_occurrence() {
        let today = parse_ymd("2026-11-19").unwrap();
        let first = |r| fmt_date(first_day(r, today).unwrap());
        assert_eq!(first("day"), "2026-11-19");
        assert_eq!(first("3 days"), "2026-11-19");
        assert_eq!(first("thu"), "2026-11-19");
        assert_eq!(first("mon"), "2026-11-23");
        assert_eq!(first("15th"), "2026-12-15");
        assert_eq!(first("month end"), "2026-11-30");
        assert_eq!(first_day("nonsense", today), None);
    }

    #[test]
    fn other_and_quarter_forms_normalise() {
        assert_eq!(round_trips("other day"), "2 days");
        assert_eq!(round_trips("other week"), "2 weeks");
        assert_eq!(round_trips("other month"), "2 months");
        assert_eq!(round_trips("other year"), "2 years");
        assert_eq!(round_trips("other quarter"), "6 months");
        assert_eq!(round_trips("other fri"), "2 weeks on fri");
        assert_eq!(round_trips("other Friday"), "2 weeks on fri");
        assert_eq!(round_trips("other workday"), "2 workdays");
        assert_eq!(round_trips("quarter"), "3 months");
        assert_eq!(round_trips("quarterly"), "3 months");
        assert_eq!(round_trips("2 quarters"), "6 months");
        assert_eq!(round_trips("3 weeks on mon"), "3 weeks on mon");
        assert_eq!(round_trips("1 week on mon"), "mon");
        assert_eq!(round_trips("3 workdays"), "3 workdays");
        assert_eq!(round_trips("1 weekday"), "weekday");
        assert_eq!(norm("other"), None);
        assert_eq!(norm("other soon"), None);
        assert_eq!(norm("2 weeks on soon"), Some("2 weeks".into()));
    }

    #[test]
    fn nth_forms_normalise() {
        assert_eq!(round_trips("2nd wed"), "2nd wed");
        assert_eq!(round_trips("second Wednesday"), "2nd wed");
        assert_eq!(round_trips("1st mon"), "1st mon");
        assert_eq!(round_trips("last fri"), "last fri");
        assert_eq!(round_trips("4th saturday"), "4th sat");
        assert_eq!(round_trips("first workday"), "1st workday");
        assert_eq!(round_trips("last workday"), "last workday");
        assert_eq!(round_trips("15th workday"), "15th workday");
        // Old forms are untouched.
        assert_eq!(round_trips("2nd"), "2nd");
        assert_eq!(round_trips("last day"), "month end");
        // Out of range falls back to the plain day of the month.
        assert_eq!(norm("6th wed"), Some("6th".into()));
        assert_eq!(norm("24th workday"), Some("24th".into()));
        assert_eq!(norm("last"), None);
    }

    #[test]
    fn other_intervals() {
        assert_eq!(step("2 days", "2026-11-19"), "2026-11-21");
        assert_eq!(step("2 weeks on fri", "2026-11-20"), "2026-12-04");
        assert_eq!(step("2 weeks on fri", "2026-11-19"), "2026-11-27");
        assert_eq!(step("3 weeks on mon", "2026-11-23"), "2026-12-14");
        assert_eq!(step("6 months", "2026-08-31"), "2027-02-28");
        assert_eq!(step("3 months", "2026-11-30"), "2027-02-28");
        assert_eq!(step("3 months", "2027-11-30"), "2028-02-29");
        assert_eq!(step("2 years", "2028-02-29"), "2030-02-28");
    }

    #[test]
    fn nth_weekday_of_the_month() {
        // 2026-11-11 is the 2nd Wednesday.
        assert_eq!(step("2nd wed", "2026-11-04"), "2026-11-11");
        assert_eq!(step("2nd wed", "2026-11-11"), "2026-12-09");
        assert_eq!(step("1st mon", "2026-11-23"), "2026-12-07");
        assert_eq!(step("1st mon", "2026-12-31"), "2027-01-04");
        assert_eq!(step("last fri", "2026-11-20"), "2026-11-27");
        assert_eq!(step("last fri", "2026-11-27"), "2026-12-25");
        assert_eq!(step("last fri", "2026-12-25"), "2027-01-29");
        assert_eq!(step("4th sat", "2026-11-01"), "2026-11-28");
        // A 5th weekday skips the months without one.
        assert_eq!(step("5th mon", "2026-11-30"), "2027-03-29");
        assert_eq!(step("last sun", "2028-02-01"), "2028-02-27");
        // Leap day itself.
        assert_eq!(step("last tue", "2028-02-15"), "2028-02-29");
        assert_eq!(step("5th tue", "2028-01-01"), "2028-02-29");
    }

    #[test]
    fn workday_of_the_month() {
        // Nov 2026: Sunday the 1st, so the 1st workday is Monday the 2nd; the last is Monday the 30th.
        assert_eq!(step("1st workday", "2026-10-15"), "2026-11-02");
        assert_eq!(step("1st workday", "2026-11-02"), "2026-12-01");
        assert_eq!(step("last workday", "2026-11-02"), "2026-11-30");
        assert_eq!(step("last workday", "2026-11-30"), "2026-12-31");
        // Jan 2027 ends Sunday-ish: Jan 31 2027 is a Sunday, so the last workday is Friday the 29th.
        assert_eq!(step("last workday", "2026-12-31"), "2027-01-29");
        assert_eq!(step("15th workday", "2026-11-02"), "2026-11-20");
        assert_eq!(step("15th workday", "2026-11-20"), "2026-12-21");
    }

    #[test]
    fn every_n_workdays() {
        // 2026-11-20 is a Friday.
        assert_eq!(step("3 workdays", "2026-11-19"), "2026-11-24");
        assert_eq!(step("2 workdays", "2026-11-20"), "2026-11-24");
        assert_eq!(step("2 workdays", "2026-11-21"), "2026-11-24");
        assert_eq!(step("5 workdays", "2026-11-23"), "2026-11-30");
    }

    #[test]
    fn a_new_task_starts_on_the_first_new_form() {
        let today = parse_ymd("2026-11-19").unwrap();
        let first = |r| fmt_date(first_day(r, today).unwrap());
        assert_eq!(first("2 weeks on fri"), "2026-11-20");
        assert_eq!(first("2 weeks on thu"), "2026-11-19");
        assert_eq!(first("2 weeks on wed"), "2026-11-25");
        assert_eq!(first("2nd wed"), "2026-12-09");
        assert_eq!(first("3rd thu"), "2026-11-19");
        assert_eq!(first("last fri"), "2026-11-27");
        assert_eq!(first("last workday"), "2026-11-30");
        let sat = parse_ymd("2026-11-21").unwrap();
        assert_eq!(
            fmt_date(first_day("3 workdays", sat).unwrap()),
            "2026-11-23"
        );
    }

    #[test]
    fn the_end_stops_the_series() {
        let due = Due {
            date: "2026-11-19".into(),
            time: Some("09:00".into()),
        };
        let mut r = rec("week", false);
        r.end = Some("2026-12-03".into());
        let n = advance(&r, &due, "2026-11-19").unwrap();
        assert_eq!(n.date, "2026-11-26");
        let n = advance(&r, &n, "2026-11-26").unwrap();
        assert_eq!(n.date, "2026-12-03");
        assert_eq!(advance(&r, &n, "2026-12-03"), None);
        // `next` leaves the due as it is.
        assert_eq!(next(&r, &n, "2026-12-03"), n);
        // From completion: counted from today.
        r.from_completion = true;
        assert_eq!(advance(&r, &due, "2026-11-25").unwrap().date, "2026-12-02");
        assert_eq!(advance(&r, &due, "2026-11-28"), None);
    }

    #[test]
    fn an_unreadable_rule_changes_nothing() {
        let due = Due {
            date: "2026-11-19".into(),
            time: None,
        };
        assert_eq!(next(&rec("nonsense", false), &due, "2026-11-19"), due);
    }
}
