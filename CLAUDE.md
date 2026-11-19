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
- CI: `.github/workflows/check.yml` runs `just check` (bonus; local hook is the real gate).

## Toolchain

Everything is already installed under `~/.toolchain`; don't install Rust, JDK, Gradle or Android SDK again. `~/.zshenv` sources `~/.toolchain/env.sh` for every zsh (incl. headless agents); other shells: `source ~/.toolchain/env.sh`.

- Rust: `RUSTUP_HOME`/`CARGO_HOME` in `~/.toolchain/{rustup,cargo}`. `rust-toolchain.toml` pins stable + clippy/rustfmt + wasm32.
- Rust targets already installed: x86_64-linux-gnu, wasm32, and all four Android (aarch64, armv7, i686, x86_64). No `rustup target add`.
- sccache: `RUSTC_WRAPPER=sccache` (set in `~/.zshenv`), cache `~/.cache/sccache` (~2 GiB warm, 10 GiB cap), ~95% hit rate. It already holds compiled deps, so a fresh worktree or post-`cargo clean` build is mostly cache hits: don't avoid builds or copy `target/` around to save time. Don't override `RUSTC_WRAPPER` or `CARGO_TARGET_DIR`. `sccache --show-stats` to check.
- JDK 21: `JAVA_HOME=~/.toolchain/jdk`.
- Gradle: no system `gradle`; use the wrapper. `GRADLE_USER_HOME=~/.toolchain/gradle` already caches dist 9.6.1, Android Gradle Plugin 9.3.1 and Kotlin (~1 GiB), so builds work offline-ish. Don't point Gradle at another home.
- Android: `ANDROID_HOME=~/.toolchain/android` with platforms 36/37, build-tools 36.0.0, NDK 28.2 (`NDK_HOME`), system image android-36 google_apis x86_64, `adb`, `emulator`, `sdkmanager` on PATH. AVD `orbit` in `~/.toolchain/android-user/avd`. Don't `sdkmanager --install` what's listed here.
- Disk: `target/` grows fast; `cargo clean` when space is low (sccache keeps rebuilds quick). `/tmp` is RAM-backed: put worktrees and big builds on disk.

## Agent skills

### Issue tracker

GitHub Issues via `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five canonical labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `GLOSSARY.md` + `docs/adr/`. See `docs/agents/domain.md`.
