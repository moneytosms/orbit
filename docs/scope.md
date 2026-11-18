# Orbit scope (draft)

Status: **draft**. Open questions at the bottom must be answered before Phase 1 is locked in the [v1 map](https://github.com/moneytosms/orbit/issues/1).
Sources: planning chat decisions, the v1 map, `docs/adr/0001-stack.md`, and a creator's "personal dashboard built in Claude Code" video used as inspiration.

## Principles

1. **Capture friction is the product.** Every input path is one hotkey or two taps, then organise later. A feature that adds friction to capture does not ship.
2. **One hierarchy.** Everything hangs off an Area, so nothing floats loose.
3. **Self-hostable on 1 GB RAM / 2 vCPU.** No local AI models, no extra services. Integrations and AI are optional, bring-your-own keys.
4. **Planner first, not a notes app.** Pages and the Library support planning; they are not the centre.

## Domain hierarchy (proposed, see Q2)

```
Area (Home, College, Career, Health, ...)
├── Project        kind: finite (end date) | ongoing; optional monthly reset (retainer-style)
│   ├── Section, Task (subtasks, recurrence, priority, reminders)
│   ├── Milestone  (% complete)
│   └── Checklist  (from templates, Phase 3)
├── Goal           outcome + target date; progress from linked tasks/milestones or manual log
├── Routine        time of day, streak, fixed N days or ongoing
└── Records        Phase 3+: People, Content, Books, Inventory (see Q5)
Page               standalone markdown, links to anything
```

A Task lives in one Project (or straight in an Area) and can count toward one Goal.

## Features by phase

| Feature | Phase | Notes |
|---|---|---|
| Areas → Projects → Tasks, sections, subtasks | 1 | Core hierarchy |
| Recurring tasks, priorities, labels, due dates | 1 | |
| Goals with milestones and progress | 1 | Already decided |
| Quick capture: Android widget, share target, desktop hotkey, web quick-add, CLI | 1 | Todoist syntax via `orbit-core` parser |
| Offline queue + background sync | 1 | Already decided |
| On-device reminders (alarms) | 1 | Works offline |
| Today view with **Top 3** (star to pin) | 1 | Cheap, high value |
| Inbox, Upcoming, list/board per project | 1 | |
| **Routines + streaks** (morning/afternoon/evening/anytime, N-day or ongoing) | 1 | Separate from tasks so Today isn't cluttered (Q3) |
| **Slipping** view (projects/tasks untouched for N days) | 1 | Needs only a last-touched timestamp |
| Stardust | 1 | Already decided |
| Todoist import (active items) | 1 | Already decided |
| Basic Pages (markdown + attachments) | 1 | Already decided |
| Global search (SQLite FTS5) | 2 | |
| Google Calendar, read-only, shown in Today | 2 | Server background job, OAuth |
| Server push notifications: daily summary, missed routine | 2 | ntfy (self-hostable) or Pushover (Q6) |
| Voice capture: on-device speech-to-text → AI cleanup + routing | 2 | BYO Anthropic key; falls back to the parser (Q7) |
| Recently-captured feed ("did it land in the right place?") | 2 | |
| Settings: integration health, last sync, force sync | 2 | |
| Project checklists + templates | 3 | |
| Time logging per project | 3 | Q8 |
| Retainer-style monthly reset of recurring items | 3 | |
| People (facts, interaction log) | 3 | Record type |
| Library: notes, quotes with thoughts, journal with photos | 3 | Q4 |
| Kindle highlights import, book records | 3 | |
| Resurfacing: daily quote, items flagged for review | 3 | |
| Content pipeline (status, channel, outline, board) | 4 | Record type |
| Inventory | 4 | Record type |
| Chat over your data (Anthropic API) | 4 | |
| Photo of handwritten journal → AI extraction | 4 | |
| Native GPUI desktop client | see below | Q1 |
| Watch capture | 4 | Wear OS only; no iOS (Q9) |
| Calendar write-back, sharing, multi-account, E2E encryption, iOS | later / never | |

## Capture paths

| Path | Phase |
|---|---|
| Android home-screen widget, share sheet | 1 |
| Desktop global hotkey (quick-capture window) | 1 |
| Web quick-add, CLI `orbit add` | 1 |
| Voice in app (on-device speech-to-text) | 2 |
| AI routing ("task for home tomorrow 2pm" → Area + due + reminder) | 2 |
| Android quick-settings tile | 2 |
| Wear OS tile or voice | 4 |

## Desktop client

**Recommended: Tauri for Phase 1, GPUI as a candidate later.**
Tauri reuses the one Svelte UI, so the Linux app costs close to nothing beyond tray, hotkey and notifications. A GPUI app is a second, separate UI written in Rust: every screen is built twice, and GPUI (Zed's framework) is desktop-only, still changing, and thinly documented (unverified, see the "GPUI desktop client maturity" research ticket). Revisit GPUI if the Tauri desktop app feels slow or non-native once v1 is in daily use. Pending Q1.

## Notifications

Two kinds:
- **Reminders and alarms (Phase 1):** scheduled on the device from synced data, so they fire offline.
- **Server push (Phase 2):** daily summary, missed routine, slipping alerts. Recommended: **ntfy**. It is open source, self-hostable next to Orbit, and has an Android app with no Google dependency. Pushover is an optional paid alternative.

## Integrations and AI

- **Google Calendar (Phase 2):** read-only pull every few minutes, plus push notifications from Google where available. Events appear in Today. Health shown in Settings.
- **AI (Phase 2+):** optional, bring-your-own key. Speech-to-text runs on the device (Android's built-in recogniser); only the text goes to the model, which cleans it up and routes it. Nothing runs locally on the server, so the 1 GB limit holds.

## Open questions

1. **Desktop:** You said you'd prefer a native desktop app "same as for Android". Note that the planned Android app is Tauri (a web view) plus native Kotlin for the widget, share target and alarms; it is not fully native. Do you want (a) Tauri desktop in Phase 1 with GPUI as a later option, (b) GPUI desktop in Phase 1, which means two UIs from day one, or (c) native everywhere (GPUI desktop + Kotlin/Compose Android + Svelte web), which means three UIs?
2. **Hierarchy name:** Should the top level be called **Area** (the video says "domain")? Can a Task sit directly in an Area with no Project?
3. **Routines vs recurring tasks:** Should Routines be separate from tasks (own view, streaks, no due dates), replacing the earlier idea of habits as recurring tasks that count toward a goal? Can a Routine still count toward a Goal (for example "run 5K daily in June" → fitness goal)?
4. **Library:** You said Orbit isn't a notes app, but the Library (notes, quotes, journal) is a notes feature. In scope for Phase 3, or out?
5. **Records:** Should People, Content, Books and Inventory be built as one generic "Records" engine (typed fields, statuses, linked tasks) with built-in templates, or as separate hand-built modules?
6. **Push service:** ntfy (self-hosted, free) or Pushover (hosted, paid once)?
7. **AI:** Is AI capture and routing worth Phase 2, given it needs an API key and network, or should the parser be enough until later?
8. **Time logging and retainers:** Do you bill hours or run recurring client work? If not, drop both.
9. **Watch:** Do you own a Wear OS watch? If not, drop watch capture.
10. **Phase 1 size:** Phase 1 has grown (Routines, Slipping and Top 3 added). Keep it all, or move Routines to Phase 2?
