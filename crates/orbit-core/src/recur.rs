//! Recurrence rules: `every …` (from the due date) and `every! …` (from completion).
//! Dates are floating local, so a rule never touches the time of day and DST needs no handling.
use crate::Due;
use crate::parse::{
    civil_from_days, days_from_civil, days_in_month, fmt_date, parse_ymd, time_word, weekday,
    weekday_name,
};
use serde::{Deserialize, Serialize};

/// `rule` is normalised text, e.g. `day`, `3 weeks`, `mon,wed`, `15th`, `month end`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Recurrence {
    pub rule: String,
    pub from_completion: bool,
}

#[derive(Debug, PartialEq)]
enum Rule {
    Days(i64),
    Weekdays,
    Weeks(i64),
    /// Bit 0 = Monday.
    On(u8),
    Months(i64),
    DayOfMonth(i64),
    MonthEnd,
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
            Rule::Weeks(n) => every(*n, "week"),
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
        _ => {
            if let Some(d) = ordinal_day(first) {
                (Rule::DayOfMonth(d), 1)
            } else if let Some((mask, n)) = weekday_list(w) {
                (Rule::On(mask), n)
            } else {
                let n: i64 = first.parse().ok().filter(|n| (1..=999).contains(n))?;
                match w.get(1)?.as_str() {
                    "day" | "days" => (Rule::Days(n), 2),
                    "week" | "weeks" => (Rule::Weeks(n), 2),
                    "month" | "months" => (Rule::Months(n), 2),
                    "year" | "years" => (Rule::Years(n), 2),
                    _ => return None,
                }
            }
        }
    };
    let time = w.get(used).and_then(|t| time_word(t));
    if time.is_some() {
        used += 1;
    }
    Some((rule, time, used))
}

fn stored(rule: &str) -> Option<Rule> {
    let words: Vec<String> = rule.split_whitespace().map(str::to_lowercase).collect();
    let (rule, _, used) = read_rule(&words)?;
    (used == words.len()).then_some(rule)
}

/// `days` moved `n` months on, clamped to the end of a shorter month.
fn add_months(days: i64, n: i64) -> i64 {
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

/// The first date strictly after `base` that the rule lands on.
fn after(rule: &Rule, base: i64) -> i64 {
    match rule {
        Rule::Days(n) => base + n,
        Rule::Weeks(n) => base + 7 * n,
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
        r => after(&r, today - 1),
    })
}

/// The next due after completing a task: strictly after `due.date` for `every`, or after
/// `today` for `every!`. The time of day is kept as is. An unreadable rule or date leaves `due` unchanged.
pub fn next(rec: &Recurrence, due: &Due, today: &str) -> Due {
    let base = if rec.from_completion {
        parse_ymd(today)
    } else {
        parse_ymd(&due.date).or_else(|| parse_ymd(today))
    };
    match (stored(&rec.rule), base) {
        (Some(rule), Some(base)) => Due {
            date: fmt_date(after(&rule, base)),
            time: due.time.clone(),
        },
        _ => due.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(rule: &str, from_completion: bool) -> Recurrence {
        Recurrence {
            rule: rule.into(),
            from_completion,
        }
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
    fn an_unreadable_rule_changes_nothing() {
        let due = Due {
            date: "2026-11-19".into(),
            time: None,
        };
        assert_eq!(next(&rec("nonsense", false), &due, "2026-11-19"), due);
    }
}
