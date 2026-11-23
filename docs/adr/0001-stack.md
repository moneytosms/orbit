# Server-authoritative sync with a shared Rust core

Status: Accepted
Date: 2026-11-19

Orbit uses a Rust (axum) server with SQLite as the source of truth, and a Todoist-style sync: client-generated ids, an idempotent command queue, sync-token deltas, per-field last-writer-wins. A shared Rust crate (`orbit-core`: quick-add parser, recurrence, reminder scheduling, Stardust) compiles native for the server, CLI and Tauri shell, and to wasm for the UI. One Svelte 5 UI ships as the web app and inside Tauri 2 for Linux and Android; a Kotlin module owns the Home-screen widgets, share target, WorkManager scheduling and AlarmManager reminders, and calls the Rust core over JNI (no window needed), so Rust stays the only database writer and captures sync with the app closed. Android 12+ (minSdk 31).

## Considered options

- **CRDT local-first (Loro/Automerge)**: true concurrent merge, but single-user rarely conflicts; harder queries, more RAM, weaker Android story. May return for Page bodies only.
- **PocketBase (Go) backend**: fastest start (auth, files, admin), but pre-1.0, sync model bounded by its API, and the parser would live in TypeScript with no shared core.
- **Dioxus, Flutter, Compose Multiplatform, Expo**: rejected for weaker Android widget support, no Linux, or losing the shared Rust core (see `research/initial` branch).

## Consequences

- The Tauri 2 Android path is unproven; if the spike fails, Android becomes a native Kotlin thin client of the same sync API.
- Web reminders only fire with a tab open; Linux reminders need the tray app running.
