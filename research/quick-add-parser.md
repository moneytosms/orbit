# Quick-add parser prior art

Issue: moneytosms/orbit#9. Date: 2026-11-19.
Scope: Todoist quick-add syntax, plus Rust crates for natural-language date and recurrence parsing, checked for wasm32 support.

Legend: **V** = read directly from primary source (crates.io API, crate tarball, GitHub API, SDK source). **S** = official Todoist page read through a summarizing fetch; quotes not hand-checked. **U** = unverified (third-party, or primary page not reached).

## Summary

- Todoist parses quick add on the server (`POST /api/v1/tasks/quick`, V for SDK signature, path U). No official client-side parser to reuse.
- No Rust crate parses Todoist grammar (`#`, `/`, `@`/`%`, `+`, `pN`, `!`). Orbit needs its own token layer; crates only cover the date part.
- `every` vs `every!` completion semantics have no crate equivalent. Build custom.
- wasm32 was NOT compiled (no cargo/rustup on this machine). All wasm notes come from reading crate source.

## Todoist syntax

| Element | Form | Example | Source | Status |
|---|---|---|---|---|
| Project | `#Name` | `#Work` | [Quick Add](https://todoist.com/help/articles/use-task-quick-add-in-todoist-va4Lhpzz) | S |
| Section | `/Name`, after a project | `#Work /Admin` | same | S |
| Label | `%name` per article; `@name` still works, retirement planned by end of 2026 | `%email` | same | S (conflicts with third-party docs that say `@`) |
| Assignee | `+name`, shared projects only | `+Lucile` | same | S |
| Priority | `p1`, `p2`, `p3` | `p1` | same; [Priorities](https://todoist.com:443/help/articles/introduction-to-priorities-Wy82Jp) | S. Quick-add list has no `p4`; UI has 4 levels |
| Due date/time | natural language in task text | `tomorrow at 4 PM` | same | S |
| Recurring due | recurrence phrase in task text | `every other Tuesday starting March 3` | same | S |
| Reminder | `!` + time | `!3pm`, `!30m before`, `!2h`, `!tmr 3pm`, `!every sat 9am` | [Reminders](https://www.todoist.com/help/articles/introduction-to-reminders-9PezfU) | S. Pro/Business only |
| Urgent reminder | `!...!` around phrase | `!every day 9am!` | same | S. Paid plans and Android |
| Project with space | escaped space? | `#My\ Project` | third-party only | U |
| Deadline, duration | syntax not confirmed | - | none reached | U |

## Recurrence

Source: [Set a recurring due date](https://www.todoist.com/help/articles/set-a-recurring-due-date-YUYVJJAV) (S).

- `every X`: next date counts from the task's date, not completion day.
- `every! X`: next date counts from completion day. Example: `every! week` done Tuesday moves to following Tuesday.
- Phrases listed: `every day` / `daily`, `every weekday` / `every workday`, `every week`, `every month`, `every year`, `every quarter`, `every 3 months`, `every mon, fri at 20:00`, `every 12 hours starting at 9pm`, `every last workday at 3pm`, `every 2, 15, 27`, `every 3 workday`, `every other day|week|month|year|fri`, `every! 3 hours`, `after 10 days`.
- Overdue: Todoist only schedules future dates. Weekday rules move to next matching day. Postponing an overdue task moves it to tomorrow.

## Server-side parsing (V unless noted)

- SDK `quickAddTask(args)` takes `QuickAddTaskArgs = { text, note?, reminder?, autoReminder?, meta?, isUncompletable? }`, linked to operation `quick_add_api_v1_tasks_quick_post` ([requests.ts](https://github.com/Doist/todoist-api-typescript/blob/main/src/types/tasks/requests.ts), v21.1.0, 2026-11-17).
- Path `POST /api/v1/tasks/quick`: inferred from operationId, U.
- `Doist/todoist-quickadd` (v1.1.6, 2025-10-23, MIT, TypeScript) is a UI widget, not a parser ([repo](https://github.com/todoist/todoist-quickadd)).

## Crate shortlist

Versions and dates from [crates.io API](https://crates.io/api/v1/crates/) (V). Repo push dates from GitHub API (V). Clock/wasm notes from crate tarballs at `static.crates.io/crates/<name>/<name>-<ver>.crate` (V source reading, not compiled).

| Crate | Latest, released | License, MSRV | Repo last push | Parses | Clock / wasm (source) |
|---|---|---|---|---|---|
| [interim](https://github.com/conradludgate/interim) 0.2.2 | 2026-11-05 | MIT, 1.81.0 | 2026-11-17 | English dates ("next friday 8pm"), durations; fork of chrono-english per its description | `parse_date_string(s, now, dialect)` takes `now`. Optional chrono/jiff/time backends. Best candidate on maturity and recency |
| [chrono-english](https://github.com/stevedonovan/chrono-english) 0.2.1 | 2026-08-26 | MIT | 2026-08-26 | Same as interim, inspired by Linux `date` | `parse_date_string(s, now, dialect)`. chrono with `alloc` only (no `clock`/`now`), so no system clock in lib. 4.3M downloads |
| [human-date-parser](https://github.com/technologicalMayhem/human-date-parser) 0.3.1 | 2025-03-27 | MIT | 2025-09-24 | Human date phrases, pest grammar | `from_human_time(str, now)` takes `now`. chrono default features. Compile U |
| [fuzzydate](https://github.com/DevinVS/fuzzydate) 0.4.1 | 2026-04-05 | MIT | 2026-04-05 | Fuzzy NL dates | `parse_relative_to(input, now)` takes `now`. chrono `clock` enabled. No-arg `parse()` not inspected, U |
| [dateparser](https://github.com/waltzofpearls/dateparser) 0.3.1 | 2026-03-25 | MIT | 2026-03-25 | Absolute date formats (6.4M downloads) | Has `wasm` feature (js-sys, wasm-bindgen), but does not enable chrono `wasmbind`. Partial-date paths call `Utc::now()` (`src/datetime.rs` lines 313, 361). Risk of panic on wasm32-unknown-unknown |
| [two_timer](https://github.com/dfhoughton/two-timer) 2.2.5 | 2023-10-10 | MIT | 2023-10-10 | English time expressions and ranges | `Config::new()` calls `Local::now()` (`src/lib.rs` line 805). Stale (~3 years) |
| [english-to-cron](https://github.com/kaplanelad/english-to-cron) 0.1.7 | 2025-11-18 | Apache-2.0 | 2025-11-18 | "every day at 4:00 pm" to 7-field Quartz cron. No `every!`, no "every other" | Only `regex` dep, no clock. Pure function `str_cron_syntax(&str)`. Good wasm candidate |
| [rrule](https://github.com/fmeringdal/rust-rrule) 0.14.0 | 2025-04-20 | MIT/Apache-2.0, 1.74.0 | 2025-04-20 | Not NL. RFC 5545 RRULE | chrono 0.4.39 + chrono-tz. No `now` calls in lib. Possible target format for recurrence |
| [croner](https://github.com/Hexagon/croner-rust) 4.0.3 | 2026-11-04 | MIT | 2026-11-08 | Not NL. Cron evaluation | chrono optional (in default features). Next-occurrence takes a time argument. Not NL |
| [parse_datetime](https://github.com/uutils/parse_datetime) 0.17.0 | 2026-10-11 | MIT, 1.71.1 | 2026-11-18 | GNU `date -d` syntax, not Todoist | jiff (default-features=false). jiff docs: on wasm32-unknown-unknown, reading now panics without `js` feature |

Not checked for wasm: crates not listed above. Not checked for Todoist fidelity: all crates (none claim Todoist syntax).

## wasm32 notes (source reading only)

- chrono 0.4.44: `now` and `clock` read system time. Browser time on wasm32 needs `wasmbind` (`src/lib.rs` line 33; `src/offset/utc.rs` gating). Any crate enabling chrono `clock` and calling `Local::now()` / `Utc::now()` is at risk.
- jiff 0.2.15: `js` feature needed for now and time zone on wasm32-unknown-unknown (`src/lib.rs` ~line 621).
- Mitigation that avoids this: host passes `now` into the parser. interim, chrono-english, human-date-parser and fuzzydate's `parse_relative_to` already do this.
- Compile test: not run. Next step below.

## Unverified / gaps

- Todoist Dates & Times article: unreachable (TLS error). Full natural-language date grammar beyond examples above is U.
- Todoist help article 115001745265: 403. Not read.
- Deadline and duration syntax: U.
- `p4` in quick add: U (official list says p1-p3).
- `@` vs `%` for labels: official article per summary (S); re-check manually before relying on it.
- Full Reminders article and recurrence article: S only.
- Third-party syntax source: [usecarly](https://www.usecarly.com/blog/how-to-use-natural-language-in-todoist/) (search snippet only, U).

## Next step

Spike: `rustup target add wasm32-unknown-unknown`, then in a scratch crate `cargo check --target wasm32-unknown-unknown` with `interim` and `english-to-cron` (both default features off). Confirm MSRV 1.81 fits the workspace toolchain before choosing.

## Sources

- https://todoist.com/help/articles/use-task-quick-add-in-todoist-va4Lhpzz
- https://www.todoist.com/help/articles/set-a-recurring-due-date-YUYVJJAV
- https://www.todoist.com/help/articles/introduction-to-reminders-9PezfU
- https://todoist.com:443/help/articles/introduction-to-priorities-Wy82Jp
- https://developer.todoist.com/api/v1/
- https://github.com/Doist/todoist-api-typescript
- https://github.com/todoist/todoist-quickadd
- https://crates.io/api/v1/crates/{interim,chrono-english,human-date-parser,fuzzydate,dateparser,two_timer,english-to-cron,rrule,croner,parse_datetime}
- https://static.crates.io/crates/<name>/<name>-<ver>.crate
