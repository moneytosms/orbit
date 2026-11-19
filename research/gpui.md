# GPUI as an Orbit desktop client

Issue: moneytosms/orbit#10. Date: 2026-11-19.
Scope: is GPUI viable now for a standalone Linux desktop client (later Windows/macOS), vs Tauri 2 reusing the Svelte 5 UI (ADR 0001).
Method: primary sources only (crates.io API, github.com/zed-industries/zed, gpui.rs, github.com/longbridge/gpui-kit, v2.tauri.app, zed.dev/blog). Facts read via summarizing fetch, not quote-checked. No code built. Secondary sources flagged.

## Verdict

**Not viable now. Revisit later, not as a replacement for Tauri.** GPUI is pre-1.0, not regularly published to crates.io, and has no tray, global hotkey or notification API in core. Tauri 2 covers all three and reuses the Svelte UI. Keep ADR 0001.

## Findings

### Release status and versioning

- crates.io `gpui`: latest and only stable is **0.2.2**, published **2025-10-22**. Versions 0.1.0 (2022, yanked), 0.2.0 (2025-10-09), 0.2.1 (2025-10-14), 0.2.2. Last publish is about 13 months before today. Source: https://crates.io/api/v1/crates/gpui
- The gpui-component crate is at 0.10.0 (2026-11-16), so the component layer ships far more often than the core. Source: https://crates.io/api/v1/crates/gpui-component
- Zed itself is at 1.x: Zed 1.0 shipped **2026-04-29**. The Zed blog does not describe GPUI's status or publishing policy. Source: https://zed.dev/blog/zed-1-0 (date verified; GPUI status not stated there).
- Implication: real users depend on git or Zed-pinned snapshots, not a versioned crate. **Unverified**: how gpui-component pins GPUI (it says gpui-kit "pins the matching GPUI release").

### API stability

- README: "GPUI is still in active development as we work on the Zed code editor, and is still pre-1.0." and "There will often be breaking changes between versions." Source: https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
- gpui.rs: "For the near future, GPUI is tied to Zed, so contributions will need to be made there and kept in sync with it." Source: https://gpui.rs
- Docs point at source: the README says the best way to learn the APIs is reading Zed's source. Source: same README.

### Docs

- Thin. README plus `docs/contexts.md`, `docs/key_dispatch.md`, `_ownership_and_data_flow`, `_accessibility` (the last has no body on the fetched page). Examples directory. Core API lives in `crates/gpui/src/gpui.rs`. Sources: https://github.com/zed-industries/zed/tree/main/crates/gpui , https://gpui.rs
- The crate's docs.rs field is null on crates.io (no documentation URL). Source: https://crates.io/api/v1/crates/gpui

### Platform support

- Windows: "no features are required. Windowing uses Win32 and text uses DirectWrite." Source: README (above).
- Linux: enable `wayland`, `x11`, or both for desktop windows. Source: README.
- macOS: Metal always available; glyph rasterization needs `font-kit`. Source: README.
- Zed 1.0 is described as available on macOS, Windows and Linux. **Secondary**: a Neowin report says the Windows build is stable. Source (secondary): https://www.neowin.net/news/windows-1011-users-have-a-reason-to-rejoice-as-microsofts-rival-gains-platform-support/
- Source tree: `crates/gpui_linux`, `gpui_macos`, `gpui_apple`, `gpui_ios` exist. A `gpui_windows` crate was not seen in the listing (the listing was cut off), so it is **unverified**. Source: https://github.com/zed-industries/zed/tree/main/crates

### Third-party apps shipped with it

- **Longbridge Pro** (commercial, per gpui-component README: "powered Longbridge Pro from day one"). Vendor claim, not independently verified. Source: https://github.com/longbridge/gpui-kit (README, 2026)
- **Discord desktop client rewrite to Rust + GPUI**: **secondary only**, unconfirmed by Discord. Source (secondary): https://www.buildmvpfast.com/blog/discord-electron-replacement-rust-gpui-desktop-frameworks-2026
- Open-source examples from search, unverified, mostly hobby or early stage: Zedis (Redis GUI), a GPU Git client, Herdr GPUI (latest release 2026-10-03), Mobie Studio, Rovar. Source: https://github.com/topics/gpui (via search)
- Forks: Kael (GPUI fork) and Liora (component library). Source: https://docs.rs/crate/kael/0.2.0/source/README.md

### Component libraries (gpui-component / gpui-kit)

- Repo is github.com/longbridge/gpui-kit. The crate `gpui-component` is published from there. Licence Apache-2.0 (code), CC BY 4.0 (docs). Source: https://crates.io/api/v1/crates/gpui-component and https://github.com/longbridge/gpui-kit
- Claims: 75+ components, virtual lists, data tables, dock layout, Tree-sitter code editor, Markdown/HTML rendering, wasm target. Source: gpui-kit README.
- **Gap**: no standalone text input, IME, dialog, notification or system tray is named on the README. The code editor is the only editing component named. Text input quality on Linux is **unverified**.
- gpui-kit says the default backend is "the pinned upstream GPUI snapshot" and offers a `gpui-fast` feature. Its usage example is `gpui-kit = "0.7"` while the crate is at 0.10.0, so the README and crate are out of sync. **Unverified** which is current.

### Text input and IME

- GPUI core has an `EntityInputHandler` trait ("Implement this trait to allow views to handle textual input when implementing an editor, field, etc."). Methods include `replace_text_in_range`, `replace_and_mark_text_in_range`, `marked_text_range`, `unmark_text`, `bounds_for_range`, `text_input_configuration`. Source: https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/src/input.rs
- So the hooks for IME (marked text, candidate window bounds) exist in core. Whether Linux IBus/fcitx works on Wayland is **unverified**: I found no Linux IME backend file in `crates/gpui_linux/src` listing (the listing was incomplete).

### Tray, global hotkey, notifications

- **GPUI core**: no tray, global hotkey or notification API found. Searched the `crates/gpui/src/platform` listing and the `test/` directory. Not conclusive: the listing was partial and no source was read. **Treat as absent until verified.** Source: https://github.com/zed-industries/zed/tree/main/crates/gpui/src/platform
- **Tauri 2 (for comparison)**:
  - System tray is in core behind the `tray-icon` Cargo feature, no plugin needed. Source: https://v2.tauri.app/learn/system-tray/
  - Caveat: "Linux: Unsupported. The event is not emitted even though the icon is shown." Tray click events do not fire on Linux; the icon and context menu still work. Same source.
  - Global shortcut is an official plugin for Windows, Linux and macOS, requires Rust 1.90+, and needs explicit capability permissions. Wayland support is **not stated**. Source: https://v2.tauri.app/plugin/global-shortcut/
- For Orbit, ADR 0001 already notes "Linux reminders need the tray app running". Tauri's Linux tray-event gap affects click-to-open behaviour and needs a workaround regardless of framework.

### Effort vs Tauri 2 with the existing Svelte 5 UI

Judgment, not measured:

- **Tauri**: reuses the Svelte 5 UI and the wasm `orbit-core`. Remaining work is the shell (tray, global hotkey, notifications, sync glue). Low risk, known APIs.
- **GPUI**: rewrites the whole UI in Rust, drops the Svelte and wasm UI sharing that ADR 0001 relies on, and builds its own text input, IME glue, tray, hotkey and notifications. The component library helps but has no text input named. Cost is multiples of Tauri, and the target moves under you (pre-1.0, git-pinned).

## Revisit triggers

Revisit only if most of these are true:

1. `gpui` is published to crates.io again with a regular cadence (last publish is 2025-10-22), or Zed states a stability or versioning policy for GPUI.
2. A text input component with verified Linux Wayland IME (IBus/fcitx) exists, tested by someone other than the vendor.
3. A tray, global hotkey and notification story exists in GPUI or a maintained crate. Tauri's plugins are the bar.
4. Orbit's Svelte UI is no longer the plan (e.g. the Tauri Android spike in ADR 0001 fails and the UI must be native anyway). Then GPUI is compared against Compose and native Kotlin, not Tauri.
5. Tauri's Linux tray gap or Wayland shortcut limits become blocking for the reminder path.

## Unverified / not checked

- Whether gpui-component pins GPUI by git or crates.io (README and crate versions disagree: 0.7 vs 0.10.0).
- Linux IME behaviour (Wayland and X11) for GPUI and gpui-component.
- Windows backend crate (`gpui_windows`) existence and maturity.
- Discord, Longbridge Pro and the open-source apps: all vendor, blog or search claims, no code read.
- Tauri global shortcut behaviour on Wayland.
- Issue #10 says to run after docs/scope.md Q1 is answered. `docs/scope.md` has no Q1 heading, so this ran without that gate.
