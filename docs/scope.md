# Orbit scope (draft)

Status: **draft**. Decisions here feed the [v1 map](https://github.com/moneytosms/orbit/issues/1).
Sources: planning chat decisions, the v1 map, `docs/adr/0001-stack.md`, and a creator's "personal dashboard built in Claude Code" video, filtered to cheap, high-value ideas only.

## Principles

1. **Capture friction is the product.** Every input path is one hotkey or two taps, then organise later.
2. **One hierarchy.** Projects, Goals and Routines all hang off one tree, so nothing floats loose.
3. **Self-hostable on 1 GB RAM / 2 vCPU.** No local AI models, no extra heavy services.
4. **Planner, not a notes app.** Pages support planning; they are not the centre.
5. **You choose what you see.** Every view and the overall appearance are configurable.

## Hierarchy

```text
Area (Home, College, Career, ...)      ← top level
├── Project                             ← optional
│   └── Task
└── Task                                ← tasks may sit straight in an Area
Inbox                                   ← built-in Area for unsorted captures; can't be deleted, never needs filing
```

Project-first and Area-first people both fit: use Projects, or put everything straight into Areas. Goals and Routines also belong to an Area (Inbox by default).


## Three kinds of "repeat" (kept separate in model and UI)

| | **Recurring task** | **Routine** | **Goal** |
|---|---|---|---|
| What it is | One piece of work that comes back | A habit you check in on | An outcome you work toward |
| Example | Pay rent monthly | Take medicine daily for 2 weeks; gym Mon/Wed/Fri | Read 5 books; get an internship |
| Missed it | Stays overdue until done | That day is missed; no pile-up | Progress just doesn't move |
| Tracks | Next due date | Daily check-ins, streak, completion rate | % progress toward a target |
| Lives in UI | Task lists, Dashboard | Dashboard Routines section, Routines page | Horizon view, Goal page |

A Routine can optionally **count toward** a Goal (gym → "get fit"), but stays its own thing.

## Routines

- **Setup in one sheet:** name, schedule, length, time of day, reminder. Defaults: daily, ongoing, anytime, no reminder.
  - **Schedule:** daily, specific weekdays, every N days, or N times per week (gym 3x, any days). All in v1.
  - **Length:** ongoing (gym), or time-boxed: N days from a start date (medicine for 14 days).
  - **Time of day:** morning, afternoon, evening, anytime, or an exact time.
  - **Reminder:** optional, on-device (see Reminders).
- **Quick capture too:** `routine take medicine daily 9am for 2 weeks` via the same quick-add parser.
- **Daily tracking:** the Dashboard shows due Routines grouped by time of day; one tap checks in, long-press skips.
- **Streaks:** current streak, best streak, completion rate. A skipped day doesn't break a streak; a missed one does.
- **Ending:** a time-boxed Routine archives itself at its end date with its final completion rate. An ongoing one can be paused or archived.
- **Stardust:** check-ins earn Stardust.

## Kept from the video (cheap, high value)

| Feature | Phase | Why keep |
|---|---|---|
| Routines with streaks | 1 | Separates habits from tasks so the Dashboard isn't a wall of reminders |
| **Slipping** view | 1 | One last-touched timestamp and a query; stops projects silently dying |
| Recently captured feed | 1 | A query on creation time; confirms a quick capture landed where it should |
| Voice capture via the keyboard mic | 1 | Free: Android's keyboard dictation already types into quick-add, and the parser routes it |
| Command palette with fuzzy full-text search | 1 | One Ctrl+K box finds anything and runs any action, offline, in `orbit-core` |

## Phase 2 (deferred, not built in v1)

- **Server push:** daily summary and missed-routine alerts via self-hosted ntfy.
- **Google Calendar:** read-only events on the Dashboard and the Calendar view, with integration health in Settings.

## Removed

People, content pipeline, books and Kindle import, inventory, the Library (notes, quotes, journal) and everything built on it (resurfacing, review flags, photo-journal AI), chat over your data, AI voice routing, time logging, retainers, checklist templates, and watch capture. No code exists yet, so this removal is in the plan only.

## v1 (Phase 1) feature list

- Areas, Projects, Inbox; tasks with sections, subtasks, priorities, labels, due dates, recurring tasks.
- Goals with milestones and progress.
- Routines (above).
- Views: Inbox, Dashboard (What's next, Routines, Goal progress, streaks, stats), Upcoming, Calendar, Area/Project list/board, Horizon, Slipping, Recently captured, plus saved custom views.
- Quick capture: Android quick-add widget, share target, desktop hotkey, web quick-add, CLI; Todoist-style syntax.
- Android Home-screen widgets: Quick-add, List (pick a source, complete Tasks), Routines (tick today's Check-ins) and Goal (one Goal or a list, view only). See "Android widgets: kinds, sources and actions" (#28); data feed open in "List widget data feed with the app closed" (#29).
- Offline queue with background sync.
- Reminders on phone and desktop (see Reminders).
- Stardust. Todoist import (active items). Basic Pages with attachments.

## Reminders (v1)

Scheduled on each device from synced data; no server involved.
- **Android:** exact alarms (`setAlarmClock`, `USE_EXACT_ALARM`), re-registered after reboot. Fire offline, app closed.
- **Desktop (Linux):** system notifications via Tauri's notification plugin while Orbit is open or in the tray. Cheap: the app already holds the synced schedule.
- **Web:** browser notifications while a tab is open.
- Timed tasks get a reminder by default; per-task and per-Routine override.

## Calendar view (native, v1)

- Month, week and day layouts.
- Shows tasks by due date/time and deadline, Routine days (with check-in state), Goal target dates and milestones.
- Drag a task to reschedule; click an empty slot to quick-add with that date prefilled.
- Uses only Orbit's own data. External calendars arrive in Phase 2.

## Editable views and appearance (v1)

- **Every view is a saved configuration:** which Areas/Projects/labels it includes, filters (due, priority, done), grouping, sort, layout (list, board, calendar, timeline, table) and which fields show on each task.
- **The Dashboard is assembled from sections** you toggle and reorder: What's next, Routines, Goal progress, streaks, stats, Slipping, Recently captured.
- **Custom views:** save any configuration as a named view (Todoist-filter style) and pin it to the sidebar.
- **Sidebar:** reorder or hide views, Areas and Projects; choose the start view.
- **Appearance:** light/dark/system theme, accent colour, density (compact/comfortable), font size.
- View and appearance settings sync across devices like any other data.

## Clients and install

| Client | Install as | Built with |
|---|---|---|
| Web | Any browser; installable as a PWA | Svelte 5, served by the Orbit server |
| Linux desktop | `.AppImage`, `.deb`, `.rpm` | Tauri 2 (same Svelte UI) + tray, hotkey, notifications |
| Android | Signed `.apk` you sideload | Tauri 2 (same Svelte UI) + Kotlin module for widget, share, alarms, background sync |
| Windows, macOS | `.msi`/`.exe`, `.dmg` | Same Tauri build, later phase |

**Tauri is locked** for desktop and Android: one Svelte UI serves web, Linux and Android, and the Rust shell shares `orbit-core` with the server, so it is the least-work route to all three installable clients. A native GPUI desktop client stays an idea for later (research ticket "GPUI desktop client maturity", answered in issue #10: not viable now; revisit when the conditions listed there hold).

## UI layer

Svelte 5, with headless behaviour from a Svelte-native library (Bits UI or similar) for menus, dialogs and popovers. Vercel's UI SDK (public beta, Nov 2026) is a candidate for drag and drop and keyboard navigation once its Svelte adapter is proven; it is not a v1 dependency. See the comparison in the planning chat.
