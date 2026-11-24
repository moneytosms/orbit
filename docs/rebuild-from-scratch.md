# Orbit: local teardown record + rebuild guide

Written 2026-11-24 before wiping local build state. Source of truth is GitHub `moneytosms/orbit` (main pushed, v0.4.3). No secret values in this file.

## 1. KEEP before wiping (irreplaceable)

| Item | Path | Why |
|---|---|---|
| Android release keystore | `~/.config/orbit/release.jks` + `release.properties` | APK signing. Lose it = existing installs can't update. Also in GH secrets (`ORBIT_KEYSTORE_B64`, `_PASSWORD`, `ORBIT_KEY_ALIAS`, `ORBIT_KEY_PASSWORD`), but GH secrets are write-only |
| Tauri updater key | `~/.config/orbit/updater.key` (+ `.pub`) | Signs AppImage/Windows updates; pubkey baked in app. GH secret `TAURI_SIGNING_PRIVATE_KEY` |
| Fix plan | `~/.claude/plans/orbit-fix-list.md` | Audit notes |
| Orbit data (optional) | `~/.local/share/orbit/{orbit.db,replica.db}`, `~/.local/share/dev.orbit.app/` | Dev-era local DB + desktop webview state. Real data lives on the VPS server |
| Desktop device token | libsecret entry `desktop-device@orbit:default` (localhost:8080 dev server) | Dev only, disposable |

Keys go into `pass` (user approval first). Local branches (all already in main by commit message) and `local-issues/` were deleted on 2026-11-24; only `main` remains locally.

## 2. What exists on this machine (sizes at 2026-11-24)

Disk was 3.2G free on `/` (97% used) at recon time.

### Toolchain `~/.toolchain` (18G, delete whole dir to uninstall)
- `android/` 7.9G: ndk 28.2.13676358 (2.2G), system-images android-36 google_apis x86_64 (4.3G), emulator (863M), platforms android-36 + android-37.0, build-tools 36.0.0, cmdline-tools/latest, platform-tools
- `android-user/` 5.2G: AVDs `orbit`, `orbit-local-run` (5.2G), `debug.keystore`
- `rustup/` 1.3G + `cargo/` 1.8G: rustc/cargo 1.100.0 stable, targets x86_64-linux, wasm32-unknown-unknown, aarch64/armv7/i686/x86_64-linux-android. cargo bin: `just` 1.59.0, `wasm-pack` 0.15.1, `cargo-tauri` (tauri-cli 2.12.6), `git-cliff`, `sccache` 0.18.0 (system or cargo)
- `gradle/` 1.3G: GRADLE_USER_HOME (dist 9.6.1, AGP 9.3.1, Kotlin)
- `jdk/` 347M: OpenJDK 21.0.13
- `env.sh`: exports above; sourced by `~/.zshenv`

### Other caches
- `~/.cache/cargo-target` 16G: Rust build output (CARGO_TARGET_DIR was set in dotfiles 00-env.zsh; that line is now removed)
- `~/.cache/sccache` 3.1G (shared with other Rust work? none found; only orbit)
- `~/.local/share/pnpm` 1.3G (pnpm store), `~/.cache/pnpm` 115M, `~/.npm` 605M: shared with other node projects, optional
- `~/.cargo/registry` 1.1G + `~/.rustup/toolchains` 1.9G: stale duplicate of toolchain (CARGO_HOME/RUSTUP_HOME point to `~/.toolchain`). No other Cargo project in `~/Projects` found. Verify `rustup` not used elsewhere before deleting
- `~/.config/.android`, `~/.android` (debug keystore, adb keys, tiny)

### In repo `~/Projects/orbit` (1.9G, `.git` 240M)
- `app/src-tauri/gen/android/app/build` 1.1G, `app/node_modules` 280M, `site/node_modules` 329M, `site/dist` 16M, `app/dist` 3.4M, `app/src-tauri/gen/android/.gradle` 8M
- `target/` already gone (cargo clean earlier)
- `.remember/` 2.6M, `.claude/` untracked

### Stray orbit dirs
- `~/Projects/orbit-local-run.ZbNt3a` 1.8G (cache 1.6G, checks 269M, scratch 143M, logs 19M): old codex orchestration run, progress.md/handoff.md inside. Safe to delete
- `~/Projects/orbit-wt` 392K (check logs), `~/Projects/orbit-delegate-wt` (empty), `~/Projects/.delegate-wt`, `~/Projects/.delegate-wt-GEx2` (logs only/empty), `~/Projects/Burrow` (empty, unrelated?)
- `/tmp`: `orbit-app-base.svelte`, `orbit-cargo-build.lock`, `orbit-handoff.yieRP7.md`, `orbit-local-run.ZbNt3a`, `delegate-qakP` (tmpfs, vanishes on reboot)
- `~/.cache/delegate-*` Claude project dirs; `~/.claude/projects/-home-sms-Projects-orbit*` 318M+ transcripts (+ ~35 `delegate-wt-*` dirs, small)

### Installed artifacts
- `~/Applications/Orbit_0.4.1_amd64.AppImage` (87M), `~/Downloads/Orbit_0.3.0_amd64.AppImage`
- `~/.local/share/applications/Orbit.desktop`, `~/.local/share/icons/hicolor/512x512/apps/orbit.png`
- `~/.local/bin/orbit` (CLI, 14M)
- `~/.config/orbit/` (keys above), `~/.local/share/orbit/`, `~/.local/share/dev.orbit.app/`
- `~/Pictures/Screenshots/*orbit-app.png` (8 files), `~/Projects/certificates/certificate-dark-orbit.svg` (unrelated cert art)
- `~/.config/ponytail/band/orbit.csv` (usage telemetry)

### Processes / units
- 45 failed transient systemd user units `orbit-zbnt3a-*` (`systemctl --user reset-failed`)
- `adb` server (pid 3167131), only if emulator used

### Remote (not touched by local wipe)
- GitHub repo `moneytosms/orbit`, Actions secrets listed above, releases up to v0.4.3, GHCR image `ghcr.io/moneytosms/orbit`, Pages site, VPS running orbit-server (update via docker compose / systemd; see `docs/deploy.md`)

## 3. Rebuild from zero

System packages (Arch): `webkit2gtk-4.1 gtk3 librsvg libsoup3 patchelf xdotool base-devel nodejs pnpm docker git github-cli`. (Tauri Linux deps; check `docs/linux-desktop.md`.)

```sh
# repo
git clone git@github.com:moneytosms/orbit.git ~/Projects/orbit && cd ~/Projects/orbit

# rust (default ~/.cargo, no custom toolchain dir needed)
curl https://sh.rustup.rs -sSf | sh     # rust-toolchain.toml pins stable+clippy/rustfmt+wasm32
cargo install just wasm-pack git-cliff sccache tauri-cli --locked   # tauri-cli 2.x; sccache optional
just hooks                               # pre-push runs just check

# gate (needs ~5G free; first build slow without sccache)
just check
```

Android (only if building APK/emulator; ~13G):
```sh
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
# JDK 21 (pacman -S jdk21-openjdk), Android cmdline-tools, then:
sdkmanager "platforms;android-36" "build-tools;36.0.0" "ndk;28.2.13676358" "platform-tools" \
  "emulator" "system-images;android-36;google_apis;x86_64"
export ANDROID_HOME=... NDK_HOME=$ANDROID_HOME/ndk/28.2.13676358 JAVA_HOME=...
avdmanager create avd -n orbit -k "system-images;android-36;google_apis;x86_64"
cd app/src-tauri && cargo tauri android build --apk     # gradle wrapper pulls Gradle 9.6.1 / AGP 9.3.1
```
Release signing: restore `~/.config/orbit/release.jks` + `release.properties` and `updater.key` from backup; CI uses the GH secrets, so tagging a release works with no local keys.

Dev loop: `just server` (127.0.0.1:8080, ./data), `just web` (needs `just wasm`), `cargo tauri dev` in `app/src-tauri`, `just demo` (web demo into site/public/demo).
Release: `just release X.Y.Z --dry-run` then `--publish` (needs git-cliff, clean main). See `docs/releasing.md`.
Server deploy: `docs/deploy.md`. Desktop install: AppImage from Releases.

### Original env (for reference, replaceable by rustup defaults)
`~/.toolchain/env.sh` exported RUSTUP_HOME, CARGO_HOME, JAVA_HOME, ANDROID_HOME/SDK_ROOT/USER_HOME/AVD_HOME, GRADLE_USER_HOME, NDK_HOME and prepended cargo/jdk/android bins to PATH. `~/.zshenv` sourced it and set `RUSTC_WRAPPER=sccache` if installed. After the wipe, remove those lines from `~/.zshenv` and the comment block at `~/.config/zsh/config/00-env.zsh:36`.

## 4. Full teardown plan (plain terms)

Step 0 is done: local branches other than main deleted (their commits are already in main).

| # | What | In plain words | Frees | Needs |
|---|---|---|---|---|
| 1 | Keys into `pass` | Save the Android signing key + update-signing key in your password store | 0 | your OK on pass path |
| 2 | `local-issues/` | Delete the private task-notes folder inside the repo (never pushed) | 1M | you run `rm -rf` (hook blocks me) |
| 3 | `~/.zshenv` lines | Stop every new terminal from loading the Rust/Java/Android setup. Do this BEFORE step 4 | 0 | edit |
| 4 | `~/.toolchain` | Delete the whole dev kit: Rust, Java, Android SDK, emulator + virtual phones, Gradle | 18G | `rm -rf` by you |
| 5 | `~/.cache/sccache` + `~/.cache/cargo-target` | Delete the compile-speedup cache and Rust build output | 19G | `rm -rf` |
| 6 | Repo build output | Delete downloaded JS packages + built Android/web output inside the repo (`node_modules`, `dist`, `gen/android/app/build`, `.gradle`). Source stays | 1.8G | `rm -rf` |
| 7 | `~/Projects/orbit-local-run.ZbNt3a`, `orbit-wt`, `orbit-delegate-wt`, `.delegate-wt*` | Delete leftover scratch dirs from the old AI worker runs (logs, caches, copies of binaries) | 1.8G | `rm -rf` |
| 8 | `~/.cargo/registry`, `~/.rustup` | Old duplicate Rust downloads from before the toolchain dir existed. Confirm no other Rust project first | 3G | `rm -rf` |
| 9 | Leftover system bits | `adb kill-server`; clear 45 failed background-job entries (`systemctl --user reset-failed`); remove `/tmp/orbit-*` | 0 | no root |
| 10 | Installed app (optional) | Delete the Orbit app file, menu entry, icon, CLI at `~/.local/bin/orbit`, app data in `~/.local/share/{orbit,dev.orbit.app}`, config `~/.config/orbit` (AFTER keys are in pass) | ~110M | `rm` |
| 11 | Claude history (optional) | Delete saved chat logs for this project (`~/.claude/projects/-home-sms-Projects-orbit*`, 318M+) | 0.3G | `rm -rf` |
| 12 | Remote branches (optional) | 13 old `prototype/*` and `research/*` branches on GitHub. Needs your OK | 0 | `git push origin --delete` |
| 13 | System packages (optional) | `webkit2gtk-4.1 patchelf xdotool` etc. were installed for Tauri; other apps may use them | small | `csudo pacman -Rns` |
| 14 | Repo itself (optional) | Delete `~/Projects/orbit` (everything is on GitHub) | 0.2G after 6 | `rm -rf` |

Check after: `df -h /` and `ls ~/.toolchain ~/.cache/sccache` should be gone.
