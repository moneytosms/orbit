# Dioxus for Orbit: research findings

Snapshot: sources retrieved 2026-11-19. Anything not fetched from a source is marked **UNVERIFIED**.

## Verdict

Dioxus is viable for **desktop** (Linux/Windows). For **Android** it is not a good fit for Orbit's must-haves as of this snapshot. The home-screen widget, share-sheet target, and notifications are not supported by Dioxus, and the Android activity and manifest are not customizable through documented APIs. Those features would need undocumented hacks or a Kotlin layer that the project does not officially support.

Suggested direction (not researched here): a Rust core crate (domain model, SQLite, sync) shared with a native Android UI, plus Dioxus for desktop if wanted. Evaluate the alternative (Kotlin/Compose, possibly with a Rust core via UniFFI) before committing.

## 1. Version, cadence, maturity, breaking changes

- **Latest stable: 0.7.10**, released 2026-07-30. Marked "Latest" and not a pre-release. [GitHub releases](https://github.com/DioxusLabs/dioxus/releases), [crates.io versions API](https://crates.io/api/v1/crates/dioxus/versions)
- **0.8 is still alpha** in this snapshot: `v0.8.0-alpha.0` (2026-05-19) through `alpha.3` (2026-10-16). No `0.8.0` on crates.io. [GitHub releases](https://github.com/DioxusLabs/dioxus/releases), [crates.io](https://crates.io/api/v1/crates/dioxus/versions)
- **Major versions**: 0.6.0 on 2024-12-07; 0.7.0 on 2025-10-31 per crates.io. [crates.io](https://crates.io/api/v1/crates/dioxus/versions). The blog's "Dioxus 0.7" post is dated 2025-09-08 and calls it "a new release", not stable. The date gap is **UNVERIFIED** (likely alpha/RC timing). [Dioxus blog](https://dioxuslabs.com/blog/), [0.7 post](https://dioxuslabs.com/blog/release-070)
- **Cadence**: 0.7.x patch releases roughly every 1 to 4 weeks (0.7.5 on 2026-04-06, 0.7.6 on 04-22, 0.7.7 on 05-01, 0.7.8 on 05-07, 0.7.9 on 05-08, 0.7.10 on 07-30). [GitHub releases](https://github.com/DioxusLabs/dioxus/releases)
- **Breaking changes in the 0.8 alphas** (from release notes):
  - alpha.1: 0.7 asset fallback removed; CLI cannot run apps built with Dioxus older than 0.7.2. [GitHub releases](https://github.com/DioxusLabs/dioxus/releases)
  - alpha.2: HTML elements moved out of `dioxus_elements`; `libxdo` made opt-in on Linux. [GitHub releases](https://github.com/DioxusLabs/dioxus/releases)
  - alpha.3: keyboard-types 0.8, named keys moved to `NamedKey` enum. [GitHub releases](https://github.com/DioxusLabs/dioxus/releases)
- **0.6 to 0.7**: a migration guide exists; specific breaking changes are not listed in the blog post. [0.7 post](https://dioxuslabs.com/blog/release-070)
- **Maturity signals**: Blitz is "still considered a work in progress"; "Not every CSS feature is supported yet" (0.7 post, 2025-09-08). [0.7 post](https://dioxuslabs.com/blog/release-070). Open issues matching "android": 52 total (page 1 of 3 inspected). [GitHub issues search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)

### Org risk

- On 2026-09-10 the Dioxus team announced it is joining **Cognition** (maker of Devin). Lead Blitz engineer Nico Burns is to work on Dioxus full time. "Going forward, we will be investing heavily in Dioxus-Native and Blitz." The post says the move lets them continue "without financial pressure". It makes **no commitments** on release cadence, licensing, or governance. [Joining Cognition](https://dioxuslabs.com/blog/joining-cognition)

## 2. Android support

### Official status (verbatim)

- Mobile apps are rendered with the platform WebView, or "experimentally with WGPU". [Mobile guide 0.7](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile)
- "While native Android animations and widgets aren't currently supported, CSS-based animations and styling provide a powerful alternative." [Mobile guide 0.7](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile)
- Since 0.6, `dx` has built-in mobile support. [Mobile guide 0.7](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile)

### Build tooling

- Requires Android Studio, SDK, command-line tools, NDK, and CMake. Rust targets: `aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`. Run with `dx serve` against an emulator. [Mobile guide 0.7](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile)
- The guide does **not** document an APK or AAB build command. Release-build status: open issue #5251 ("Android Release Build Fails with Multiple Gradle Compatibility Issues", 2026-01-22). [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)
- The CLI generates a Gradle project rather than a prebuilt APK, because Kotlin and Java must be compiled. [dioxus-cli 0.7.9 android.rs](https://docs.rs/crate/dioxus-cli/0.7.9/source/src/build/android.rs)
- Android device support was added with community help in 0.7 (`dx serve --platform android`), with hot-patching on real devices. [0.7 post](https://dioxuslabs.com/blog/release-070)

### Runtime wiring (fixed names)

- The Rust side binds to `dev.dioxus.main` with activity `WryActivity` via `tao::android_binding!` and `wry::android_binding!`. The doc comment says the CLI expects these names and that changing them "you'll need to change the CLI as well". [dioxus-desktop 0.7.9 mobile.rs](https://docs.rs/crate/dioxus-desktop/0.7.9/source/src/mobile.rs)
- No documented way to supply a custom Activity, AndroidManifest entries, or Gradle customization. [dioxus-desktop 0.7.9 mobile.rs](https://docs.rs/crate/dioxus-desktop/0.7.9/source/src/mobile.rs)
- The mobile guide does not document JNI or native API access. [Mobile guide 0.7](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile)

### Known Android blockers (open issues)

- Startup crashes: #6067 "random abort at launch, RefCell already mutably borrowed" (2026-11-06); #5878 white screen from a Wry activity init race (2026-10-02). [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)
- Blank or frozen webview: #5885 (2026-10-03). [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)
- Linking on Windows hosts: backslashes lost in the linker response file, breaking Android fat-link: #5847, #5661, #5976. [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)
- NDK path with a space breaks `dx serve --android`: #5835. [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)
- OpenSSL `.so` not bundled into APK: #5565. [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)
- Input and navigation: onclick not firing on Android (#5786); back navigation minimizes the app (#5155); missing context menu and text-selection toolbar (#5653). [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)
- Meta-issue "Android and IOS build": #5794 (open). [Issue search](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android)

### Real-world apps shipped

**UNVERIFIED**. Not searched in this pass.

## 3. Widgets, share intents, notifications, background sync

| Feature | Dioxus status | Path | Source |
|---|---|---|---|
| Home-screen widget | Not supported. Official docs say "widgets aren't currently supported". Feature request #3637 open since 2025-01-24 with no maintainer comment. | Native `AppWidgetProvider` + RemoteViews or Jetpack Glance, in Kotlin. Needs manifest entry and a bridge to Rust data. | [Mobile guide](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile), [#3637](https://github.com/DioxusLabs/dioxus/issues/3637), [Android widgets](https://developer.android.com/develop/ui/views/appwidgets/overview) |
| Widget interaction | Android widgets allow only touch and vertical swipe. No horizontal drag, so in-widget Kanban drag is not possible. Tap-to-open or quick-add is the realistic pattern. | Native only. | [Android widgets](https://developer.android.com/develop/ui/views/appwidgets/overview) |
| Share-sheet target | Not documented. Needs an `ACTION_SEND` intent-filter in the manifest with `android:exported="true"`, and the Activity is fixed as `WryActivity`. | Manifest change plus Activity handling. Blocked without CLI or manifest customization. | [Android share receive](https://developer.android.com/training/sharing/receive), [mobile.rs](https://docs.rs/crate/dioxus-desktop/0.7.9/source/src/mobile.rs) |
| Notifications | Not documented. Android 13+ requires runtime permission `android.permission.POST_NOTIFICATIONS`, off by default on new installs. | Kotlin prompt plus posting. Needs glue. | [Notification permission](https://developer.android.com/develop/ui/views/notifications/notification-permission) |
| Background sync | Not documented. Android's recommended API is WorkManager (Kotlin, `androidx.work` 2.12.0). | Kotlin `Worker` scheduled from app code. Rust sync logic called from it via JNI. | [WorkManager](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started) |
| Native API access (general) | Feature request #3855 "Dioxus Desktop & Mobile Native APIs" open (2025-03-11). A "native plugin" example (`geolocation-native-plugin`) is the maintainers' answer to #3972 "Java/Kotlin ffi on dioxus Mobile", closed 2026-04-30. | Example-based pattern, not a documented API. | [#3972](https://github.com/DioxusLabs/dioxus/issues/3972), [#3855](https://github.com/DioxusLabs/dioxus/issues?q=is%3Aissue+is%3Aopen+android) |

Rust-to-JNI path: a non-maintainer described using `ndk_context::android_context()` to reach the Android VM and class loader, and needing to make the webview field public (a "hack") (comment on #3972, 2025-05-13). [#3972](https://github.com/DioxusLabs/dioxus/issues/3972). The `dioxus-desktop` source exposes `Java_dev_dioxus_main_WryActivity_create` and `start_app` as JNI entry points. [mobile.rs](https://docs.rs/crate/dioxus-desktop/0.7.9/source/src/mobile.rs)

No public docs or issues were found on widgets or share intents from a Dioxus app. **Whether a Kotlin module can be added to the generated Gradle project without forking the CLI is UNVERIFIED.** The 0.7.6 CLI source has a `buildSrc` layout, which may be a hook point. [dioxus-cli 0.7.6 android.rs](https://docs.rs/crate/dioxus-cli/0.7.6/source/src/build/android.rs)

## 4. Desktop rendering and local DB

- **Desktop**: Dioxus desktop uses **wry (system webview) and tao** windowing. [mobile.rs](https://docs.rs/crate/dioxus-desktop/0.7.9/source/src/mobile.rs), [0.8 alpha.3 notes](https://github.com/DioxusLabs/dioxus/releases) ("tao 0.37 / wry 0.57").
- **Native renderer (Blitz)**: a modular HTML/CSS engine using Stylo, Taffy, Parley. README says "currently in a beta state", with "many bugs and missing features". Dioxus Native uses it. [Blitz README](https://github.com/DioxusLabs/blitz)
- **SQLite**: `rusqlite` 0.43.0 with the `bundled` feature compiles SQLite from source via `cc`. Page says it helps "where linking to SQLite is complicated, such as Windows". **Android is not mentioned. Cross-compile to Android is UNVERIFIED.** [rusqlite README](https://github.com/rusqlite/rusqlite). `sqlx` on Android: **UNVERIFIED**, not researched.

## 5. Ecosystem: Kanban DnD and markdown

**Kanban / drag-and-drop**
- One web search found no Dioxus-specific Kanban crate; general DnD crates below. Search-only, not exhaustive. Search results: [dioxus-dnd-kit](https://docs.rs/crate/dioxus-dnd-kit/0.1.3/source/Cargo.toml).
- **dioxus-dnd 3.1.0** (2026-08-30), built on Dioxus 0.7. Keyboard (Space/Enter pick up, arrows move, Esc cancels), touch, auto-scroll, live sortable preview. Repo: https://github.com/kindintelligence/dioxus-dnd. Install: `cargo add dioxus-dnd`. [docs.rs](https://docs.rs/crate/dioxus-dnd/latest). Maturity, download counts, and maintenance: **UNVERIFIED**.
- **dioxus-dnd-kit 0.1.3**: Draggable, Droppable, DraggableOverlay; examples include `simple_list`. Maturity **UNVERIFIED**. [docs.rs](https://docs.rs/crate/dioxus-dnd-kit/0.1.3/source/Cargo.toml)

**Markdown**
- `pulldown-cmark`: CommonMark pull parser in Rust. [pulldown-cmark README](https://cdn.jsdelivr.net/gh/pulldown-cmark/pulldown-cmark@main/README.md)
- Official `DioxusLabs/markdown` converts to HTML and injects it via `dangerous_inner_html`. Simple read-only rendering; check sanitization. [WebSearch: dioxus markdown](https://github.com/DioxusLabs/markdown)
- `dioxus-nox-markdown` 0.13.x: headless editor, previewer, and display components with byte-offset AST. Escapes raw HTML by default. Its crate docs state the code was **generated by AI (Claude)**; vet before use. [docs.rs source](https://docs.rs/crate/dioxus-nox-markdown/0.13.0/source/src/lib.rs)
- `dioxus-markdown` 0.0.1 (rambip, 2025-03): unstable, lightly documented. [docs.rs](https://docs.rs/crate/dioxus-markdown/latest)

## 6. Risks vs payoff

**Payoff**: one Rust codebase for desktop, with type-safe state and a fast RSX/CSS-based UI. Hot-patching during development. Strong fit for the desktop half.

**Risks**:
1. **Android widgets and share-sheet cannot be done in Dioxus alone.** Official docs say widgets are unsupported. Manifest and Activity are fixed by the CLI. The only documented Kotlin path is an example, not an API. This is the decisive risk for Orbit's capture requirements.
2. **Android stability**: many open crash and build issues (startup aborts, white screens, Windows-host linking, release Gradle failures). The project is pre-1.0 in practice, with 0.8 still in alpha.
3. **Org and roadmap**: the team is now inside Cognition, with focus shifting to Blitz and Dioxus-Native (not mobile widgets). No stated commitments on cadence or licensing. Stated investment is Dioxus-Native and Blitz (rendering); Android widget request #3637 has had no maintainer response since 2025.

**Mitigation if chosen anyway**: keep the Rust core crate framework-neutral (domain, SQLite, sync), and build the Android app as a separate Kotlin/Compose project that calls the core. This keeps the Dioxus desktop UI optional.

## Unverified items (summary)

- Real-world Dioxus Android apps in production.
- Whether the generated Gradle project accepts a user Kotlin module or manifest changes without a CLI fork.
- APK/AAB build command.
- rusqlite and sqlx cross-compilation to Android.
- Periodic WorkManager minimum interval (not in fetched text).
- Maturity and maintenance of dioxus-dnd and dioxus-dnd-kit.
- Dioxus 0.7.0 stable release notes beyond the blog post.
- Whether 0.8.0 stable shipped after 2026-10-16.
