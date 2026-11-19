# Orbit

Personal task + notes app (Todoist-style). Rust axum server + SQLite is source of truth; clients sync via idempotent command queue, sync-token deltas, per-field LWW. One Svelte 5 UI ships as web (PWA) and in Tauri 2 (Linux, Android minSdk 31). See `docs/adr/0001-stack.md`, `docs/scope.md`, spec #31, layout #22.

## Layout

- `crates/orbit-core`: shared logic (parser, recurrence, reminders, Stardust); native + wasm, every fn takes `now`/tz as input
- `crates/orbit-wasm`: wasm-bindgen wrapper over core
- `crates/orbit-client`: sync client, SQLite replica, outbox (Tauri + CLI)
- `crates/orbit-server`: axum + SQLite, bin `orbit-server`
- `crates/orbit-cli`: bin `orbit`
- `app/`: Svelte 5 + Vite (pnpm); `app/src-tauri`: Tauri crate `orbit-app`

## Commands

- `just check`: the gate. pnpm install, svelte-check, vitest, web build, cargo fmt/clippy (`-D warnings`)/test, wasm32 build. Must pass before done.
- `just hooks`: once per clone; pre-push runs `just check`.
- Toolchain in `~/.toolchain` (sourced by `~/.zshenv`); non-zsh shells: `source ~/.toolchain/env.sh`. `rust-toolchain.toml` pins stable + clippy/rustfmt + wasm32.
- CI: `.github/workflows/check.yml` runs `just check` (bonus; local hook is the real gate).

## Agent skills

### Issue tracker

GitHub Issues via `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five canonical labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `GLOSSARY.md` + `docs/adr/`. See `docs/agents/domain.md`.
