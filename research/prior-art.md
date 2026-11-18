# Orbit: prior art survey (as of 2026-11-19)

Question: which open-source apps could Orbit borrow from (fork, reuse code or libs, copy data model or UX), and is any of them a realistic "just use this" answer?

Orbit scope: tasks, Kanban, light markdown notes, long-term goals/roadmaps, 30-day plans, semester/career/job-application tracking, media tracking (shows, books), desktop + Android, sync.

Method: GitHub metadata and LICENSE/README/package files read via `gh` (live, 2026-11-19). Product and sync claims from docs/wiki pages via WebFetch. Items marked UNVERIFIED were not checked against a primary source in this pass. Done inline, no background agent.

## Summary table

Activity = last push and latest release seen. Stars approximate (GitHub, 2026-11-19).

| App | Repo | License | Stack | Android | Sync | Activity | Fit for Orbit |
|---|---|---|---|---|---|---|---|
| AppFlowy | AppFlowy-IO/AppFlowy | AGPL-3.0 | Flutter (Dart) + Rust | Yes (Play Store, Flutter) | Own cloud, self-host (server repo archived) | Pushed 2026-11-15, rel 0.15.1 (2026-11-15), ~79k stars | Closest all-in-one. Docs + databases + Kanban + mobile |
| AFFiNE | toeverything/AFFiNE | MIT (core), backend under separate license | TypeScript, BlockSuite, Yjs/y-octo (Rust CRDT) | Not stated | Local-first, real-time, self-host | Pushed 2026-11-18, last rel v0.27.4 (2026-08-18), ~75k stars | Whiteboard/doc canvas. Mobile unclear |
| Anytype | anyproto/anytype-ts (+ -kotlin, -swift, any-sync) | Any Source Available 1.0 (not OSI) | Electron + TypeScript; Go middleware; any-sync MIT | Client repos exist (Kotlin, Swift); product desktop-only per README | P2P + E2E (any-sync) | Pushed 2026-11-10, rel v0.57.8-alpha (2026-11-10), ~9k stars | Object/type/relation model is good. Licence blocks easy reuse |
| Logseq | logseq/logseq | AGPL-3.0 | ClojureScript, Capacitor (android/, ios/ dirs) | Native app in alpha (invite/alpha per docs) | RTC paid add-on, self-hostable | Pushed 2026-11-18, rel 2.0.2 (2026-10-07), ~46k stars | Outliner/notes, not task-first. DB version still maturing |
| SiYuan | siyuan-note/siyuan | AGPL-3.0 | TypeScript/Electron front, Go kernel | Yes (Android, iOS, HarmonyOS) | Official cloud/3rd-party cloud; 3rd-party sync disks not supported | Pushed 2026-11-18, rel v3.8.10 (2026-11-10), ~47k stars | Notes-first block editor. Weak for task/kanban |
| Vikunja | go-vikunja/vikunja | AGPL-3.0 | Go backend + Vue 3 frontend (Vite, Pinia), Electron desktop wrapper (GPL-3.0-or-later) | No first-party Android app found; Android via CalDAV clients (Tasks, DAVx5) | Server-based, REST API, CalDAV (alpha) | Pushed 2026-11-18, rel v2.8.0 (2026-11-05), ~6k stars | Task + Kanban data model and API. No notes/roadmap depth |
| Planka | plankanban/planka | PLANKA Community License 1.1 (Fair Use-style, not OSI) | Not verified in this pass | Phones/tablets = Pro only | Real-time (mechanism unverified) | Pushed 2026-11-06, rel v2.3.1 (2026-11-05), ~12.7k stars | Kanban only |
| Focalboard | mattermost-community/focalboard | Compiled: MIT. Source: AGPL-3.0 or commercial (dual) | TypeScript (Go server, unverified) | No | Self-hosted server | Repo banner: "not currently maintained"; pushed 2026-05-18; last release v8.0.0 (2024-06-13) | Board/property/view model only. Do not base on it |
| Super Productivity | super-productivity/super-productivity | MIT | Angular 22, Capacitor 8 (Android/iOS), Electron 44 (desktop), browser | Yes (Capacitor) | WebDAV, Nextcloud, Dropbox, local file, SuperSync (E2E, beta) | Pushed 2026-11-18, rel v20.0.0 (2026-11-06), ~24k stars | Best tech-stack match for one-codebase desktop + Android. Task-first |
| Trilium (TriliumNext) | TriliumNext/Trilium | AGPL-3.0 | TypeScript; server with SQLite | No native app; mobile web UI only | Self-hosted sync server, per-note encryption | Pushed 2026-11-18, rel v0.107.0 (2026-11-08), ~39k stars. zadam/trilium and TriliumNext/Notes are archived | Attribute/relation note tree. Knowledge store, not planning |
| Joplin | laurent22/joplin | AGPL-3.0-or-later (root); Personal Use License for packages/server | TypeScript; React Native mobile (packages/app-mobile) | Yes (Android, iOS) | Joplin Cloud, Nextcloud, S3, WebDAV, Dropbox, OneDrive, filesystem; E2EE | Pushed 2026-11-18, rel v3.7.22 (2026-10-28), ~57k stars | Notes + sync-target abstraction. No tasks/kanban |
| Tasks.org | tasks/tasks | GPL-3.0 | Kotlin, Android only | Yes (Android only) | CalDAV, Google Tasks, Etebase (E2E), Google Drive | Pushed 2026-11-18, rel 15.14.1 (2026-11-17), ~5.8k stars | Android task app. Borrow sync adapters, not UX |

## Per app

### AppFlowy
- Licence: AGPL-3.0 (LICENSE; README badge). https://github.com/AppFlowy-IO/AppFlowy
- Stack: Flutter UI (`appflowy_flutter/`) with Rust core (`frontend/rust-lib`). Rust crates `collab`, `collab-document`, `collab-database`, `collab-folder`, `collab-plugins` at version 0.2 (verified in `frontend/rust-lib/Cargo.toml`). Statement that the collab layer is built on Yrs/Yjs: UNVERIFIED.
- Platforms: desktop plus Android and iOS. README links a Play Store listing (`io.appflowy.appflowy`) and shows mobile screenshots (Kanban, databases, mobile editing).
- Sync: AppFlowy Cloud. The server repo https://github.com/AppFlowy-IO/AppFlowy-Cloud is archived (last push 2026-09-11), yet the README still links a self-hosting guide. Current self-host path and sync reliability: UNVERIFIED.
- Maintenance: very active (pushed 2026-11-15, release 0.15.1 dated 2026-11-15).
- Borrowable: block-based document model; databases with Board, Grid and Calendar views and custom properties (directly maps to job-application, media and semester trackers); Kanban board for tasks; Rust collab core (as a dependency, not a rewrite).
- Just use? Closest single app for breadth. Gaps: AI-first positioning, a heavy Flutter + Rust build, archived server repo, and no verified built-in sync-engine story for a personal setup. Needs a hands-on trial before deciding.

### AFFiNE
- Licence: LICENSE says content is MIT except `packages/backend` and `packages/common/native`, which carry a separate licence defined in those directories. I did not read that second licence text. https://github.com/toeverything/AFFiNE
- Stack: TypeScript, BlockSuite editor, Electron for desktop (per README's list of upstream deps). y-octo, a Rust Yjs-compatible CRDT, underpins local-first sync: https://github.com/y-crdt/y-octo
- Platforms: web and desktop. Mobile not stated in README.
- Sync: local-first with real-time sync and self-hosting (Docker). Mechanism beyond that: UNVERIFIED.
- Maintenance: pushed 2026-11-18, but the last tagged release is v0.27.4 (2026-08-18). Watch that gap.
- Borrowable: BlockSuite (editor) and y-octo (CRDT) as libraries; edgeless canvas is not relevant to Orbit.
- Just use? No. Canvas-first, no Android in its own README, and no tasks or Kanban focus.

### Anytype
- Licence: Any Source Available License 1.0 (LICENSE.md). It permits non-commercial use, plus commercial use only in "Allowed Networks". Not OSI. Source: https://github.com/anyproto/anytype-ts/blob/main/LICENSE.md
- Stack: Electron + TypeScript desktop client (anytype-ts). Middleware in Go (anytype-heart, also ASAL). Mobile clients in Kotlin (anytype-kotlin) and Swift (anytype-swift), both ASAL. Sync layer any-sync is MIT: https://github.com/anyproto/any-sync
- Platforms: README claims macOS, Windows and Linux only.
- Sync: offline-first, optional P2P sync, zero-knowledge E2E encryption (README).
- Maintenance: active (anytype-ts pushed 2026-11-10; any-sync pushed 2026-11-16).
- Borrowable: object/type/relation model. Reuse of code is blocked by the licence, but the concept can be copied.
- Just use? No. Desktop-only per README and ASAL licence blocks forking for commercial use. Relevant as a data-model reference.

### Logseq
- Licence: AGPL-3.0 (LICENSE.md). https://github.com/logseq/logseq
- Stack: ClojureScript; Capacitor config (`capacitor.config.ts`) with `android/` and `ios/` dirs in repo.
- Storage: DB graphs use SQLite (per-graph `db.sqlite`). Markdown survives mainly as import/export. Source: https://github.com/logseq/docs/blob/master/db-version.md
- Sync: RTC (real-time collab) is a paid add-on (OpenCollective sponsorship tiers) and can be self-hosted. Encryption password required.
- Mobile: iOS invite-only; Android native app not yet open for alpha (per db-version doc, dated April 2026).
- Maintenance: active (pushed 2026-11-18, release 2.0.2 on 2026-10-07).
- Borrowable: outliner UX; SQLite-backed DB-graph model; plugin API.
- Just use? No. Notes-first, mobile still in alpha, sync paid.

### SiYuan
- Licence: AGPL-3.0. https://github.com/siyuan-note/siyuan
- Stack: TypeScript/Electron front-end, Go kernel (kernel layout in README/docs).
- Storage: `.sy` documents as JSON (ciphertext if encrypted), assets and plugins in workspace folders.
- Sync: official cloud and third-party cloud storage supported. Third-party sync disks not supported (README warns of corruption).
- Mobile: Android, iOS and HarmonyOS apps via App Store, Google Play and F-Droid.
- Plugins: Plugin API (petal repo) and a marketplace (bazaar repo).
- Maintenance: active (pushed 2026-11-18, release v3.8.10 2026-11-10).
- Borrowable: block-tree model and mobile app architecture. Kanban and task depth: UNVERIFIED.
- Just use? No for tasks. Possible notes engine if Orbit's notes need to be block-based.

### Vikunja
- Licence: AGPL-3.0 (LICENSE; README and vikunja.io). https://github.com/go-vikunja/vikunja
- Stack: Go backend (`pkg/`, `cmd/`), Vue 3 + Vite + Pinia frontend (`frontend/package.json`). Electron desktop wrapper in `desktop/` (GPL-3.0-or-later). Old standalone frontend repo (go-vikunja/frontend) is archived and merged into the main repo.
- Platforms: web, desktop wrapper. No first-party Android app found in a repo or doc. Android works through third-party CalDAV clients (Tasks, DAVx5) per https://vikunja.io/docs/caldav. Native Android: UNVERIFIED.
- API: REST API with Swagger docs (`try.vikunja.io/api/v2/docs`).
- CalDAV: VTODO extension, described by the docs as "early alpha stage and has bugs". Endpoints under `/dav`.
- Maintenance: active (pushed 2026-11-18, release v2.8.0 2026-11-05).
- Borrowable: project and task data model with views (List, Kanban, Table, Gantt per vikunja.io features page, UNVERIFIED in detail); REST API surface; server to reuse as sync backend for tasks.
- Just use? Partial. Tasks and Kanban are covered, notes, goals, media and job tracking are not. Good as a self-hosted task backend.

### Planka
- Licence: PLANKA Community License 1.1 (LICENSES/PLANKA Community License EN.md). Permits personal, internal, private-hosting for family and friends, and non-profit use. Forbids hosting as a commercial service. Not OSI. https://github.com/plankanban/planka
- Stack: not verified in this pass.
- Platforms: Phones and tablets are Pro-only (README feature list). Community mobile UI: UNVERIFIED.
- Sync: "instant syncing across all users" (mechanism UNVERIFIED).
- REST API and webhooks in both editions. Pro adds recurring cards, calendar, timeline, and a single card on multiple boards.
- Maintenance: active (pushed 2026-11-06, release v2.3.1 2026-11-05).
- Borrowable: Kanban UI patterns.
- Just use? No. Kanban only, Android behind Pro, and the licence rules out hosting for others.

### Focalboard
- Licence: LICENSE.txt covers compiled builds under MIT. Source is AGPL-3.0 or commercial (dual). https://github.com/mattermost-community/focalboard
- Maintenance: README states "currently not maintained". Last push 2026-05-18, last release v8.0.0 (2024-06-13).
- Mattermost Boards plugin (https://github.com/mattermost/mattermost-plugin-boards) is active (pushed 2026-11-13), but it is a plugin for Mattermost.
- Borrowable: board, card, property and view model. The Mattermost-plugin repo is the live branch.
- Just use? No. Unmaintained standalone app. Mine its model, do not base on it.

### Super Productivity
- Licence: MIT (LICENSE). https://github.com/super-productivity/super-productivity
- Stack: Angular 22, Capacitor 8 (Android/iOS), Electron 44 (desktop), web build (package.json).
- Platforms: Linux, macOS, Windows, Android, iOS, browser (README).
- Sync: WebDAV, Nextcloud, Dropbox, local file (desktop and mobile per wiki comparison), plus SuperSync (own service, E2E encrypted, beta, self-hostable with limited support). Source: https://github.com/super-productivity/super-productivity/wiki/3.08-Sync-Integration-Comparison
- Features (README): planner with schedule timeline, Kanban and Eisenhower boards, timeboxing, time tracking, notes and file/link attachments per task and project, plugin system, custom themes.
- Maintenance: very active (pushed 2026-11-18, release v20.0.0 2026-11-06).
- Borrowable: whole app is MIT, so forking is legal and the desktop + Android single-codebase stack matches the Orbit requirement. Sync provider abstraction; plugin API.
- Gaps against Orbit: no long-horizon goal or roadmap model verified; job-application and media trackers would need custom work or the plugin API (plugin capabilities: UNVERIFIED).
- Just use? Strongest "just use" for the task and Kanban half of Orbit. Not a full replacement for notes, goals or databases.

### Trilium (TriliumNext)
- Licence: AGPL-3.0. The active line is https://github.com/TriliumNext/Trilium (pushed 2026-11-18, release v0.107.0). zadam/trilium is the original, and TriliumNext/Notes is archived.
- Stack: TypeScript; server with SQLite (per repo layout, `apps/server`).
- Platforms: desktop (Electron, `apps/desktop`) and web. No native Android app; mobile web UI served by a server install (README).
- Sync: self-hosted sync server; per-note encryption (README).
- Borrowable: note tree with attributes (labels and relations), useful as a schema for media and job records. Sync-server design.
- Just use? No. Knowledge base, not a planner, and no native mobile app.

### Joplin
- Licence: AGPL-3.0-or-later for root code. `packages/server` has its own Personal Use License. Source: https://github.com/laurent22/joplin
- Stack: TypeScript; React Native mobile (`packages/app-mobile`, with `android/` and `ios/`); desktop app; CLI; plugin system.
- Platforms: Windows, Linux, macOS, Android, iOS (README).
- Sync: Joplin Cloud, Nextcloud, S3, WebDAV, Dropbox, OneDrive, filesystem (https://github.com/laurent22/joplin/blob/dev/readme/apps/sync/index.md). E2E encryption per README. Joplin Server licence restricts commercial use.
- Maintenance: very active (pushed 2026-11-18, release v3.7.22 2026-10-28).
- Borrowable: notebook and note model, Markdown on disk, sync-target abstraction (good template for Orbit's sync layer), mobile RN app.
- Just use? No. Notes only. Strong reference for sync and E2E design.

### Tasks.org
- Licence: GPL-3.0 (LICENSE). https://github.com/tasks/tasks (product: Tasks.org; the repo was revived from Astrid).
- Stack: Kotlin, Android only. Source tree has `caldav`, `gtasks`, `etebase`, `googleapis`, `drive`, `markdown` packages.
- Platforms: Android (Play Store listing, F-Droid-style APK on GitHub). No desktop.
- Sync: CalDAV, Google Tasks, Etebase (E2E), Google Drive backup.
- Maintenance: active (pushed 2026-11-18, release 15.14.1 2026-11-17).
- Borrowable: CalDAV and Etebase sync code (GPL, copyleft). Android task UX.
- Just use? No. Android-only.

## Verdict

No single app covers Orbit's full list (tasks, Kanban, notes, goals, databases for job and media tracking, desktop + Android, sync). Realistic options:

1. Adopt one app and extend it. Best fits: Super Productivity (MIT, desktop + Android, tasks and boards) or AppFlowy (AGPL, docs + databases + boards, Flutter).
2. Build on a shared data model and borrow pieces (Vikunja for tasks and API, Joplin for sync targets, Anytype for the object/relation model, AppFlowy for block and database views).

## Top 3 borrow candidates

1. **Super Productivity**: fork candidate. MIT, one codebase for desktop and Android (Electron + Capacitor), multi-provider sync including E2E SuperSync. Borrow the task, timebox and board model and the sync adapters.
2. **AppFlowy**: borrow the database-with-views model (Board, Grid, Calendar, custom properties) for job-application, semester and media tracking, plus the block editor for light notes. AGPL, so forking means AGPL for anything distributed.
3. **Vikunja**: borrow the project, task and view model and the REST API surface. Usable as a self-hosted sync backend for tasks. AGPL.

Honourable mentions: Joplin (sync-target abstraction, E2EE), Anytype (object and relation model, licence blocks code reuse).

## Strongest "just use X" contender

**AppFlowy.** Only app here that covers docs, databases, Kanban and Android in one product. Stop before committing: check the self-hosted sync path (server repo is archived), Android quality, and whether its database views can carry the semester, job and media schemas. If any of those fail, fall back to Super Productivity plus a custom notes and database layer.

## Unverified and open items

- AFFiNE mobile support and its backend licence text.
- Planka stack and sync mechanism.
- Super Productivity plugin capabilities for custom databases.
- Vikunja native Android app and Gantt/Kanban view details.
- AppFlowy collab layer built on Yrs; current self-host/sync path after AppFlowy-Cloud archival.
- SiYuan Kanban and task features.
- Focalboard source-licence text beyond LICENSE.txt summary.

## Sources

- AppFlowy: https://github.com/AppFlowy-IO/AppFlowy ; https://github.com/AppFlowy-IO/AppFlowy-Cloud (archived)
- AFFiNE: https://github.com/toeverything/AFFiNE ; https://github.com/y-crdt/y-octo
- Anytype: https://github.com/anyproto/anytype-ts ; https://github.com/anyproto/anytype-ts/blob/main/LICENSE.md ; https://github.com/anyproto/anytype-kotlin ; https://github.com/anyproto/anytype-swift ; https://github.com/anyproto/any-sync
- Logseq: https://github.com/logseq/logseq ; https://github.com/logseq/docs/blob/master/db-version.md
- SiYuan: https://github.com/siyuan-note/siyuan
- Vikunja: https://github.com/go-vikunja/vikunja ; https://vikunja.io/ ; https://vikunja.io/docs/caldav
- Planka: https://github.com/plankanban/planka
- Focalboard: https://github.com/mattermost-community/focalboard ; https://github.com/mattermost/mattermost-plugin-boards
- Super Productivity: https://github.com/super-productivity/super-productivity ; https://github.com/super-productivity/super-productivity/wiki/3.08-Sync-Integration-Comparison
- Trilium: https://github.com/TriliumNext/Trilium
- Joplin: https://github.com/laurent22/joplin ; https://github.com/laurent22/joplin/blob/dev/readme/apps/sync/index.md
- Tasks.org: https://github.com/tasks/tasks
- Release and activity data: `gh repo view` and `gh api repos/<repo>/releases/latest`, 2026-11-19.
