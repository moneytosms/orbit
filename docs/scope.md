# Orbit scope (draft)

Status: **draft**. Open questions at the bottom feed the [v1 map](https://github.com/moneytosms/orbit/issues/1).
Sources: planning chat decisions, the v1 map, `docs/adr/0001-stack.md`, and a creator's "personal dashboard built in Claude Code" video, filtered to cheap, high-value ideas only.

## Principles

1. **Capture friction is the product.** Every input path is one hotkey or two taps, then organise later.
2. **One hierarchy.** Projects, Goals and Routines all hang off one tree, so nothing floats loose.
3. **Self-hostable on 1 GB RAM / 2 vCPU.** No local AI models, no extra heavy services.
4. **Planner, not a notes app.** Pages support planning; they are not the centre.

## Three kinds of "repeat" (kept separate in model and UI)

| | **Recurring task** | **Routine** | **Goal** |
|---|---|---|---|
| What it is | One piece of work that comes back | A habit you check in on | An outcome you work toward |
| Example | Pay rent monthly | Take medicine daily for 2 weeks; gym Mon/Wed/Fri | Read 5 books; get an internship |
| Missed it | Stays overdue until done | That day is missed; no pile-up | Progress just doesn't move |
| Tracks | Next due date | Daily check-ins, streak, completion rate | % progress toward a target |
| Lives in UI | Task lists, Today | Today's Routines strip, Routines page | Horizon view, Goal page |

A Routine can optionally **count toward** a Goal (gym → "get fit"), but stays its own thing.

## Routines

- **Setup in one sheet:** name, schedule, length, time of day, reminder. Defaults: daily, ongoing, anytime, no reminder.
  - **Schedule:** daily, specific weekdays, every N days, or N times per week.
  - **Length:** ongoing (gym), or time-boxed: N days from a start date (medicine for 14 days).
  - **Time of day:** morning, afternoon, evening, anytime, or an exact time.
  - **Reminder:** on-device alarm, works offline.
- **Quick capture too:** `routine take medicine daily 9am for 2 weeks` via the same quick-add parser.
- **Daily tracking:** Today shows due Routines grouped by time of day; one tap checks in, long-press skips.
- **Streaks:** current streak, best streak, completion rate. A skipped day doesn't break a streak; a missed one does.
- **Ending:** a time-boxed Routine archives itself at its end date with its final completion rate. An ongoing one can be paused or archived.
- **Stardust:** check-ins earn Stardust.

## Kept from the video (cheap, high value)

| Feature | Phase | Why keep |
|---|---|---|
| Routines with streaks | 1 | Separates habits from tasks so Today isn't a wall of reminders |
| Today **Top 3** (star to pin) | 1 | One field; forces focus on what matters today |
| **Slipping** view | 1 | One last-touched timestamp and a query; stops projects silently dying |
| Recently captured feed | 1 | A query on creation time; confirms a quick capture landed where it should |
| Default reminder on timed tasks | 1 | One setting; a timed task without an alert is usually a mistake |
| Voice capture via the keyboard mic | 1 | Free: Android's keyboard dictation already types into quick-add, and the parser routes it |
| Global search (SQLite FTS5) | 2 | Built into SQLite; finds anything across tasks, goals and pages |
| Daily summary + missed-routine push | 2 | One HTTP call per message to a self-hosted ntfy server |
| Google Calendar, read-only, in Today | 2 | Moderate cost (OAuth); kept because you asked for it earlier |
| Integration health in Settings | 2 | Only matters once Calendar and push exist |

## Removed

People, content pipeline, books and Kindle import, inventory, the Library (notes, quotes, journal) and everything built on it (resurfacing, review flags, photo-journal AI), chat over your data, AI voice routing, time logging, retainers, checklist templates, and watch capture. No code exists yet, so this removal is in the plan only.

## v1 (Phase 1) feature list

- Project tree with tasks, sections, subtasks, priorities, labels, due dates, recurring tasks.
- Goals with milestones and progress.
- Routines (above).
- Views: Inbox, Today (Top 3, Routines strip, tasks), Upcoming, Project list/board, Horizon, Slipping, Recently captured.
- Quick capture: Android widget, share target, desktop hotkey, web quick-add, CLI; Todoist-style syntax.
- Offline queue with background sync; on-device reminders.
- Stardust. Todoist import (active items). Basic Pages with attachments.

## Clients and install

| Client | Install as | Built with |
|---|---|---|
| Web | Any browser; installable as a PWA | Svelte 5, served by the Orbit server |
| Linux desktop | `.AppImage`, `.deb`, `.rpm` | Tauri 2 (same Svelte UI) + tray, hotkey, notifications |
| Android | Signed `.apk` you sideload | Tauri 2 (same Svelte UI) + Kotlin module for widget, share, alarms, background sync |
| Windows, macOS | `.msi`/`.exe`, `.dmg` | Same Tauri build, later phase |

A native GPUI desktop client stays an idea for later (research ticket "GPUI desktop client maturity", not run).

## Open questions

1. **Desktop:** Tauri now with GPUI as a later idea, or GPUI from day one (two separate UIs)?
2. **Top level:** Is the top of the tree called **Area** (Home, College, Career)? Can a task sit in an Area with no Project?
3. **Routine → Goal link:** Can a Routine count toward a Goal, as proposed above?
4. **Routine schedules:** Is "N times per week" (gym 3x, any days) needed in v1, or are daily, weekdays and every-N-days enough?
5. **Push service:** ntfy (self-hosted, free) or Pushover (hosted, one-time fee)?
6. **Google Calendar:** Keep it in Phase 2 despite the OAuth setup every self-hoster must do, or drop it?
