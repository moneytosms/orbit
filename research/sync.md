# Orbit: sync and storage architecture

Research date: 2026-11-19. Primary sources cited per claim. `[UNVERIFIED]` = claim from memory or a third-party summary, not confirmed on a primary page this session. `[FETCH-FAILED]` = primary page could not be read.

## Requirements (assumed)

- Single user, several devices: Linux/Windows desktop, Android.
- Offline-first: all reads and writes local, sync when online.
- Data: tasks, Kanban, markdown notes, goals, media tracking.
- Rust core (Dioxus or Tauri, undecided).
- Solo developer: low ops, low dependency risk.
- Conflicts are rare (one user, usually one device active at a time), but must never silently lose data.

## Summary table

| Option | Rust | Android | Conflicts | Self-host cost | Solo complexity | Maturity |
|---|---|---|---|---|---|---|
| SQLite + custom sync server | Yes (rusqlite, sqlx) | Yes, via NDK [UNVERIFIED] | You design it | Small VPS | High (you own it) | n/a (your code) |
| Automerge 3 | Core is Rust, but Rust API "low level and not well documented" | No binding listed | CRDT, automatic merge | Repo/sync server you run | Medium | JS stable, Rust secondary |
| Loro 1.0 | Yes, Rust-first | Swift/Python/RN/C#/Go listed; Kotlin not listed | CRDT, automatic merge | You run transport | Medium | 1.0 released, no stability text fetched |
| Yrs (y-crdt) | Yes, Rust crate | Kotlin binding `ykt` exists [UNVERIFIED Android target] | CRDT, Yjs-compatible | You run transport | Medium | Active, no stability statement |
| cr-sqlite | Yes, loadable extension | Not mentioned | Per-column LWW, counters | Your transport | Medium | Latest release v0.16.3 (year not shown); "main may not be 100% stable" |
| PowerSync | Rust SDK beta; Tauri alpha | Kotlin SDK stable | Server authority, upload queue [UNVERIFIED] | Open Edition self-host (Postgres + service) | Medium-high | Stable SDKs for web/Kotlin/Swift |
| ElectricSQL | No Rust/Kotlin SDK | No | Writes are app-owned | Self-host possible | High | Read-path sync only |
| Turso embedded replicas / Turso Sync | Rust example in docs; Rust in Sync beta [UNVERIFIED] | Not mentioned | Sync: "last push wins" | Cloud-first; self-host not covered | Low-medium | Sync: beta quality per search snippets |
| Triplit | Not verified | Not verified | LWW server timestamp [UNVERIFIED] | Unclear | n/a | Stalled? [UNVERIFIED] |
| InstantDB | No Rust/Kotlin SDK | No | n/a | Self-host only after cloud shutdown | n/a | Cloud sunsetting Aug 31, 2027 |
| Markdown files + Syncthing | Files, any language | Syncthing Android app discontinued | Conflict copies (`.sync-conflict-...`) | None (P2P) | Low | Syncthing core active; Android app dead |
| Obsidian Sync | n/a (files) | Android not confirmed on page | Not described on page | Closed, not self-hostable | Low | Commercial product |

## Option details

### 1. Plain SQLite (local source of truth) + custom sync server

- Local SQLite file on every device. Writes go to SQLite first, always.
- Sync layer you write: each row carries a change ID and an HLC (hybrid logical clock) or per-row version; push/pull an append-only op log; per-field last-writer-wins for scalar fields; explicit merge rules for Kanban position (fractional index) and notes text.
- Server: small HTTP service (Axum or similar) plus SQLite or Postgres, one user, token auth. Trivially cheap to host.
- Rust: `rusqlite` / `sqlx` [UNVERIFIED versions]. Android: SQLite is in the NDK path; Dioxus/Tauri mobile both build Rust for Android targets (see Tauri section).
- Conflicts: entirely your code. Single user makes this easy: LWW by HLC is acceptable for tasks/goals; for markdown notes, keep a conflict copy rather than merging text.
- Complexity: highest in code you own, lowest in external dependency risk. No third-party service can shut off.
- Maturity: the parts are mature; the sync protocol is your responsibility.

### 2. Automerge 2/3

- Automerge 3.0 released July 2025. Memory drops "over 10x" via compressed in-memory representation; same file format as Automerge 2; API "nearly fully backwards compatible". Source: https://automerge.org/blog/automerge-3/
- Local-first: each device keeps a full copy; works offline; network-agnostic; conflicts merged automatically. Source: https://automerge.org/docs/hello/
- Rust: core implementation is Rust, exposed via WASM (JS), C (`rust/automerge-c`), "and soon other languages". Page says "the API for Rust code is low level and not well documented" and suggests `autosurgeon` for Rust apps. Only the JS package `@automerge/automerge` is called stable. Source: https://github.com/automerge/automerge
- Android: no Kotlin/Android binding named on the primary pages. Would go through C FFI [UNVERIFIED feasibility].
- Self-hosting: `automerge-repo` (JS) provides networking; a sync server is needed for multi-device without P2P. Rust server equivalent not confirmed.
- Verdict: good CRDT for JS-heavy stacks. Weak fit for a Rust-first app.

### 3. Loro 1.0

- README: "Loro 1.0 is out!" Source: https://github.com/loro-dev/loro . Release date not on fetched page [UNVERIFIED date].
- Data types: Text (Fugue), Rich text, Tree (movable), List (movable), Map (LWW), Counter listed in types discussion but not confirmed in README fetch. Shallow snapshots, delta updates, time travel. MIT license.
- Rust: Rust-first. Bindings: Swift (loro-swift), Python (loro-py), React Native, C# (community), Go (community). Source: https://github.com/loro-dev/loro-ffi
- Android/Kotlin: not listed on either page. Loro uses UniFFI internally, and UniFFI can generate Kotlin, so a Kotlin binding is plausible [UNVERIFIED].
- Caveats from loro-ffi: legacy JSON exports "panic on unreadable history" (use `try_` variants); `subscribe_jsonpath` "fires (with possible false positives)"; pending operations are not preserved by ordinary snapshots and need separate persistence. Source: https://github.com/loro-dev/loro-ffi
- Verdict: best Rust-native CRDT option on paper. Android path unconfirmed; 1.0 stability text not fetched (loro.dev blog was 403).

### 4. Yrs (y-crdt)

- Rust crate of the Yjs algorithm, binary-compatible with Yjs. Feature table: yrs 0.21, yffi 0.21, ywasm 0.18. Source: https://github.com/y-crdt/y-crdt
- Bindings: Kotlin (`ykt`), Swift (`yswift`), Python (`pycrdt`, replacing `ypy`), Ruby, R, .NET. Source: same.
- Android: `ykt` exists; whether it targets Android (vs plain JVM) is [UNVERIFIED].
- No production-readiness statement on the page. Stated aim: maintain compatibility with Yjs.
- Ecosystem benefit: Yjs providers (y-websocket, etc.) and editor bindings. Usable with a custom Rust server.
- Verdict: viable for notes/text if you want Yjs ecosystem; more moving parts than SQLite-only.

### 5. cr-sqlite (vlcn-io)

- Run-time loadable SQLite/libSQL extension adding CRDT columns. Source: https://github.com/vlcn-io/cr-sqlite (path fetched).
- Conflict resolution per column: LWW, fractional index (LWW), counters (summed). Multi-value registers, PERITEXT rich text, and history log are planned; counters/rich text still incomplete. Source: same.
- Limits: inserts about 2.5x slower than normal tables; `crsql_as_crr()` needed on tables; extension loaded on every connection; `crsql_finalize()` before close.
- Release: v0.16.3 marked latest, dated "17 Jan" (year not shown on page) [UNVERIFIED year]. Source: https://github.com/vlcn-io/cr-sqlite/releases
- README: "For a stable version, build against a release tag as main may not be 100% stable." Needs Rust nightly to build the extension.
- Android: not mentioned. Build from source for each Android target [UNVERIFIED].
- Verdict: closest fit to "SQLite everywhere", but small maintainer footprint, nightly build, no Android story. Risky as sole sync layer.

### 6. PowerSync

- Architecture: client SQLite synced from a backend DB (Postgres, MongoDB, MySQL beta, SQL Server beta). Source: https://docs.powersync.com/
- Client SDKs: stable JS web, React Native/Expo, Node.js, Dart/Flutter, Kotlin, Swift; beta Capacitor, .NET, Rust; alpha Tauri. Source: same.
- Conflicts: client writes go to an upload queue, applied server-side; server is authority. Not described on fetched pages [UNVERIFIED, from general PowerSync docs knowledge].
- Self-host: "Self-hosting of free open-source edition" with Postgres/MongoDB/MySQL/SQL Server backends, all SDKs, partial sync, config-file setup. License name not stated on fetched page. Not included in OSS: custom checkpoints, CVE-patched image early access, email support. Source: https://www.powersync.com/pricing
- Cloud (usage-based): Free $0 with 2 GB synced/month, 500 MB hosted data, 2 instances; Pro from $49/mo; Team from $599/mo. Free projects deactivated after inactivity (page gives both 1 and 2 weeks; assume 1). Source: same.
- Solo complexity: need Postgres + PowerSync service. Your Rust app becomes a PowerSync client only via beta Rust SDK, or via a JS/Kotlin UI.
- Verdict: strong, stable, but the Rust side is beta and Tauri is alpha. Best if the UI is JS (Tauri webview with JS SDK) or Kotlin.

### 7. ElectricSQL

- "A read-path sync engine for Postgres": syncs to clients over HTTP via "Shapes". Writes are app-owned via your API. Source: https://electric.ax/docs/intro (redirect from electric-sql.com/docs/intro).
- Local writes/offline: not addressed on the docs index page.
- SDKs: official TypeScript; Elixir API client. No Rust or Kotlin SDK listed. Source: same.
- Self-hosting: Deployment guide; the docs say "the easiest way to use Electric in production is the Electric Cloud". License not on the index page.
- Verdict: not a fit. No Rust/Android client, no offline write story on the primary page.

### 8. Turso (embedded replicas and Turso Sync)

- Embedded replicas: local file, reads local; writes go to remote primary by default; `offline: true` allows local writes but reconciliation is not described. Rust example `Builder::new_remote_replica`. Source: https://docs.turso.tech/features/embedded-replicas/introduction
- Turso Sync (recommended replacement): reads and writes local by default; explicit `push()`/`pull()`; conflict strategy "last push wins"; `push()` sends logical statements. Languages on page: TypeScript, Python, Go; Rust not on that page. Self-hosting not covered. Source: https://docs.turso.tech/sync
- Beta status: web search snippets quote Turso's offline-writes posts as "beta quality, not yet recommended for production, no durability guarantees" [UNVERIFIED current status; the cited snippets are blog posts, and the current docs page carries no label].  Source: https://turso.tech/blog/introducing-offline-writes-for-turso
- Verdict: simplest sync API for a SQLite app, but cloud-dependent, beta-quality per Turso's own earlier posts, and "last push wins" conflicts. Runner-up if you accept a cloud dependency.

### 9. Triplit

- Triplit was acquired/joined by Supabase. Announcement 2025-10-08: "Triplit is already largely open-source"; "Our focus isn't to directly integrate Triplit into our platform"; offline plans "still open". Source: https://supabase.com/blog/triplit-joins-supabase
- Repo URL github.com/triplit/triplit returned 404 at fetch time; `triplit.dev` returned no page text. [FETCH-FAILED]
- Third-party sources (not primary): last push to repo 2026-01-19 and "stalled" [UNVERIFIED]; license reported as MIT by one source and AGPL-3.0 by another [UNVERIFIED].
- Verdict: do not build on it. Development uncertain, provenance unclear.

### 10. InstantDB

- Instant Cloud shuts down. Announcement: team joining OpenAI; new signups closed; subscriptions after 2026-07-31 refunded; all cloud apps shut down 2027-08-31; backups available until 2028-08-31. Source: https://instantdb.com/essays/instant_team_joins_openai (fetched; announcement date from search snippet, 2026-08-22).
- "All of Instant is open source"; self-host guide at https://www.instantdb.com/docs/self-hosting (linked as `/docs/self-hosting`). [UNVERIFIED URL exact path; the fetch returned the relative path only.]
- SDKs: JS-family only (React, vanilla JS, Svelte, Vue, Solid, Python on getting-started page). No Rust/Kotlin. Source: https://www.instantdb.com/docs
- Verdict: excluded. Hosted service ends in 2027.

### 11. Markdown files + Syncthing / git (Obsidian-style)

- Syncthing: no central server; TLS; device certificates. Conflict detection: files changed on two devices with differing contents. Losing copy renamed `<filename>.sync-conflict-<date>-<time>-<modifiedBy>.<ext>`. Older mtime loses; tie broken by device ID. Conflict copies sync to all devices and must be resolved manually. Source: https://docs.syncthing.net/users/syncing.html and https://syncthing.net/
- Syncthing Android app: discontinued. Last release with December 2024 Syncthing version; repo archived. Source: https://github.com/syncthing/syncthing-android
- Android alternative to the official app: not verified in this session [UNVERIFIED].
- Obsidian Sync: closed, commercial. Help page does not describe conflict handling; warns about conflicts when mixed with Dropbox/Drive/OneDrive; has Headless Sync for CLI; Android not confirmed on the page; no self-host or API mentioned. Source: https://obsidian.md/help/sync
- Git: not researched against primary sources this session [UNVERIFIED; general knowledge: merge conflicts are text-based and need manual resolution, Android git clients are limited].
- Fit for Orbit: fine for markdown notes if you accept manual conflict copies; poor fit for structured tasks/goals/media data. Would need a second store anyway.

## Tauri vs Dioxus (affects Android path)

- Tauri v2 Android setup documented: Android Studio, NDK, Rust Android targets. The page gives no stability label for Android. Source: https://v2.tauri.app/start/prerequisites/ (fetch returned 403 on some pages; this one succeeded).
- PowerSync lists Tauri as alpha, Rust SDK as beta. Source: https://docs.powersync.com/
- Dioxus mobile status: [UNVERIFIED, not fetched]. Do not assume parity with Tauri.

## Recommendation

**Primary: SQLite as local source of truth + your own thin sync layer.**

- Schema: tables for tasks/goals/media/Kanban; every row has `id`, `hlc`, `device_id`, `deleted` (tombstone).
- Sync: append-only change log pushed/pulled over HTTPS to a single-user server (SQLite or Postgres, token auth).
- Conflicts: LWW per field by HLC for scalars; fractional index for Kanban order; markdown notes keep a conflict copy instead of auto-merge.
- Why: no third-party can shut it off; works in Rust on all targets; single user keeps conflict rules tiny; hosting cost near zero.
- Cost: you write ~a few hundred lines of sync code and tests. Verification: property tests for convergence (two simulated devices, random ops, assert equal state).
- Upgrade path: if notes need real collaborative merge later, add Automerge or Loro for the notes document only.

**Runner-up: Turso Sync (cloud) if you accept a cloud dependency and beta risk.** Simplest API, SQLite-native, Rust examples exist. Blocked by unverified Rust/Android status and "last push wins". Re-check the docs before deciding.

Not recommended: Triplit (stalled/acquired, unclear license), InstantDB (sunsetting 2027-08-31), ElectricSQL (read-only sync, no Rust/Kotlin client), cr-sqlite as sole layer (small maintainer base, nightly build, no Android story), Syncthing (Android app discontinued).

## Open questions (to resolve before committing)

1. Dioxus vs Tauri: does Dioxus target Android well enough? Decide framework first; it changes the sync choice.
2. Do you want markdown notes as plain files (Obsidian-compatible) or rows in SQLite? Files push you to file-based sync and conflict copies.
3. Hosting: willing to run one small VPS or box at home for the sync server?
4. Is a 2-3 device, rarely simultaneous edit pattern acceptable with LWW? (Assumed yes.)

## Sources

- Automerge docs: https://automerge.org/docs/hello/
- Automerge 3 post: https://automerge.org/blog/automerge-3/
- Automerge Rust repo: https://github.com/automerge/automerge
- Loro README: https://github.com/loro-dev/loro
- Loro FFI: https://github.com/loro-dev/loro-ffi
- Loro docs (403, not read): https://loro.dev/docs [FETCH-FAILED]
- Yrs: https://github.com/y-crdt/y-crdt
- cr-sqlite repo: https://github.com/vlcn-io/cr-sqlite (see releases: https://github.com/vlcn-io/cr-sqlite/releases)
- cr-sqlite alt URL (404): https://github.com/cr-sqlite/cr-sqlite [FETCH-FAILED]
- PowerSync docs: https://docs.powersync.com/
- PowerSync pricing/self-host: https://www.powersync.com/pricing
- ElectricSQL docs: https://electric.ax/docs/intro (redirect from https://electric-sql.com/docs/intro)
- Turso embedded replicas: https://docs.turso.tech/features/embedded-replicas/introduction
- Turso Sync: https://docs.turso.tech/sync
- Turso offline writes blog (beta wording): https://turso.tech/blog/introducing-offline-writes-for-turso
- Triplit/Supabase: https://supabase.com/blog/triplit-joins-supabase
- Triplit repo (404): https://github.com/triplit/triplit [FETCH-FAILED]
- InstantDB essay: https://instantdb.com/essays/instant_team_joins_openai
- InstantDB docs: https://www.instantdb.com/docs
- Syncthing sync docs: https://docs.syncthing.net/users/syncing.html
- Syncthing home: https://syncthing.net/
- Syncthing Android: https://github.com/syncthing/syncthing-android
- Obsidian Sync help: https://obsidian.md/help/sync
- Tauri v2 prerequisites: https://v2.tauri.app/start/prerequisites/
