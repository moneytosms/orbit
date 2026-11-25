# Quick add: cheat sheet

A one-page list of everything quick add understands, with an example for each. The same parser runs in the web app, the desktop capture window, the Android widget and `orbit add`. In the app, Settings has the same list with a box to try phrases live.

For the step-by-step version, see [Quick add and tasks](tasks.md).

## Symbols

| Type this | It sets | Example |
| --- | --- | --- |
| `@Name` | Area or Project | `read chapter @College` |
| `$Name` | Goal | `run 5k $Get fit` |
| `%Name` | Page | `book flights %Trip notes` |
| `#name` | Label | `buy milk #errand` |
| `p1` to `p4` | Priority | `fix bug p1` |
| `!30m`, `!fri 9am`, `!later` | Reminder | `dentist fri 3pm !1h` |
| `every …`, `every! …` | Repeat (`every!` counts from completion) | `water plants every 3 days` |
| `{date}`, `by date` | Deadline | `report by friday` |
| `for duration` | Duration | `write essay for 2h` |
| `\#`, `\@` | A literal symbol | `issue \#1` |

## Dates and times

| Phrase | Result (today is Thursday 19 Nov) |
| --- | --- |
| `today`, `tomorrow`, `tmrw`, `fri`, `on friday` | That day. A bare weekday is its next occurrence |
| `next fri` | Friday of next week (27 Nov) |
| `this fri` | This week's Friday (20 Nov) |
| `next week`, `next month`, `next year` | Next Monday, same date next month, 1 January |
| `this weekend`, `next weekend` | The coming Saturday, the Saturday after |
| `end of month`, `mid jan` | 30 Nov, 15 Jan |
| `the 27th`, `on the 27th` | The next 27th |
| `dec 5`, `dec 5th`, `5 dec 2027` | That date (a month name needs a day) |
| `27/1`, `3/12` | Numbers read day/month: 27 Jan, 3 Dec |
| `6pm`, `6:30pm`, `18:00`, `6 pm` | A time. With no date it is the next time the clock shows it |
| `today at 10pm`, `fri at 7pm`, `9am fri` | Date plus time |
| `at noon`, `at midnight` | 12:00, 00:00 |
| `tomorrow morning`, `this evening`, `in the evening` | Morning 09:00, afternoon 14:00, evening 18:00, night 21:00 |
| `tonight` | 20:00 |

## Relative times

Counted from right now, to the minute: `in 5 minutes`, `in 20 min`, `in half an hour`, `in an hour and a half`, `in 1h30m`, `in 2 hours`, `90 minutes from now`, `in 3 days`, `in 2 weeks`, `in 2 months`.

## Deadline and duration

- `by friday`, `by dec 5`, `by end of month` or `{march 30}` set the **deadline**, not the due date. `written by Tom` stays in the title because no date follows.
- `for 2h`, `for 30 min`, `for 1h15m`, `for half an hour` set the **duration** (up to 24 hours).
- A time range sets both: `3-4pm`, `9:30-10:00`, `from 9 to 10:30`.

## Repeating

| Phrase | Meaning |
| --- | --- |
| `every day`, `every week`, `every month`, `every quarter`, `every year` | Simple repeats |
| `every 3 days`, `every other week` | Every N units |
| `every weekday`, `every mon,wed,fri` | On those days |
| `every other fri`, `every 2nd wed`, `every last fri` | Nth weekday |
| `every first workday`, `every 15th workday` | Nth working day of the month |
| `every day at 9am` | A time for every occurrence |
| `starting dec 1`, `until dec 31`, `for 3 months` | Start and stop dates |
| `every! 3 weeks` | Repeats three weeks after you complete it |

## Reminders

- Before the due time: `!10m`, `!30m`, `!1h`, `!1d`.
- At a fixed time: `!9am`, `!fri 9am`, `!tomorrow morning`, `!in 10 minutes`.
- Shortcuts: `!later` (3 hours from now), `!morning`, `!afternoon`, `!evening`, `!tonight`, `!night`.

## Several tasks at once

Paste several lines into the task box to add one task each. List markers (`-`, `*`, `1.`, `[ ]`) are removed and indented lines become subtasks. From a terminal, `orbit add -` reads one task per line from standard input.

## Settings that change it

Under **Settings, Quick add** you can turn date recognition off, and choose what `next week` and `this weekend` mean.
