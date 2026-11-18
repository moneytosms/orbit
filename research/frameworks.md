# Orbit: cross-platform framework comparison (as of 2026-11-19)

Requirements: Linux + Windows desktop, Android, cross-device sync, Android home-screen widget, share target, quick-add notification, offline local DB, Kanban drag-and-drop, markdown notes.

Method: primary sources (official docs, release notes, package registries, repo READMEs) fetched 2026-11-19. Items marked **[unverified]** are from memory, search snippets, or secondary sources. Dioxus is not covered here.

## 1. Summary table

| | Tauri 2 | Flutter | Expo / React Native | Compose Multiplatform | PWA |
|---|---|---|---|---|---|
| Android maturity | Stable-ish, mobile DX admitted weak | Mature | Mature (dev build needed for widgets) | Stable for Android app UI | Browser-dependent |
| Linux/Windows desktop | Mature (WebKitGTK / WebView2) | Windows stable since 2.10; Linux status unclear **[unverified]** | Windows only (Microsoft RNW); no Linux | JVM desktop, Linux + Windows packaging (deb/rpm) | Installable; Chrome/Edge |
| Android widget | No official; community `tauri-plugin-widgets` + native Kotlin | `home_widget` (native Kotlin AppWidget or Glance) | `expo-widgets` (alpha/stable conflict) + config plugin | Glance is Android-only, native module | None |
| Share target | Community plugins only | `receive_sharing_intent` 1.9.0 | `expo-sharing` receive (Android stable path, iOS experimental) | Native Android intent filter | `share_target` manifest; experimental, install required |
| Quick-add notification | Official notification plugin; persistent/actions need custom Kotlin **[unverified]** | `flutter_local_notifications` 23.0.3 (Android action buttons) | Native module needed **[unverified]** | Native Android | Not viable **[unverified]** |
| Offline DB | sqlx-based SQL plugin (SQLite); community libsql | `drift` (Linux, Windows, Android, etc.) | `expo-sqlite` (Android/iOS/macOS/web; no Win/Linux) | SQLDelight/Room **[unverified]** | OPFS (Chrome 86, FF 111, Safari 15.2) + SQLite-WASM |
| Kanban DnD | Web libs (dnd-kit etc.), touch via WebView | `super_drag_and_drop` (experimental) or built-in widgets **[unverified]** | `react-native-draggable-flatlist` (Reanimated + gesture-handler) | `Reorderable` 3.1.0 (Android, desktop, wasm, js) | dnd-kit (touch sensors) |
| Markdown editor | Any web editor (Milkdown, CodeMirror, TipTap) | `flutter_quill` (no native MD; Delta) | `react-native-live-markdown` (Android/iOS/web, no desktop) | `compose-rich-editor` 1.1.0 (MD import/export) | Milkdown etc. |
| Solo velocity | High if you know web UI; Rust core fits | High (Dart) | High (JS/TS) | Medium-high (Kotlin; smaller ecosystem for desktop UI) | Highest, but misses widget/quick-add |
| Verdict for Orbit | Strong candidate | Strong candidate | Weak (no Linux desktop) | Strong candidate | Not sufficient alone |

## 2. Per framework

### Tauri 2

- Stable since 2024-10-02. Mobile claimed "production ready" in RC, with admitted caveats: "We are not completely happy about the developer experience at the moment." Not all plugins support mobile. `tauri-action` does not automate mobile builds. Source: https://v2.tauri.app/blog/tauri-20/ and https://v2.tauri.app/blog/tauri-2-0-0-release-candidate/ (search snippet, not fetched).
- Native plugin code in Swift/Kotlin is the official extension path (same source above). Android setup: Android Studio, NDK, Rust Android targets. Source: https://v2.tauri.app/start/prerequisites/
- Official mobile plugins: barcode, biometric, haptics, NFC, notification (desktop/mobile), sql (sqlx-based). No official share or widget plugin. Source: https://v2.tauri.app/plugin/
- Community: `tauri-plugin-sharesheet`, `tauri-plugin-mobile-sharetarget` (share intent), `tauri-plugin-widgets` (WidgetKit + AppWidgetManager), `tauri-plugin-libsql`. Source: https://v2.tauri.app/plugin/ (community list). Maturity of these: **[unverified]**.
- Linux: WebKitGTK 4.1. Windows: WebView2. Source: https://v2.tauri.app/start/prerequisites/
- Latest release seen: tauri 2.12.7 (2026-11-18 per GitHub page). Source: https://github.com/tauri-apps/tauri/releases
- Community note (unverified): Android WebView lacks `env()`, so safe-area CSS needs a JS fallback. Source: community skill guide, not official.
- PowerSync Tauri SDK is alpha. Source: https://docs.powersync.com/client-sdks/overview

### Flutter

- Windows stable in Flutter 2.10 (Feb 2022). macOS/Linux were beta at that time. Current Linux stability not stated in https://docs.flutter.dev/platform-integration/linux/building. Sources: https://www.infoworld.com/article/3649201/google-flutter-210-brings-stable-support-for-windows-targets.html (2022); https://docs.flutter.dev/platform-integration/desktop (no stability label). **[unverified current status]**
- `home_widget` 0.10.0 (Android + iOS). Widget UI must be written natively; Glance support added in 0.5.0; configurable Android widgets in 0.9.1. Source: https://pub.dev/packages/home_widget and https://pub.dev/packages/home_widget/changelog. Flutter's own docs page for Android widgets returned 404 (not found).
- `receive_sharing_intent` 1.9.0 (Android, iOS). Android intent filters for SEND/SEND_MULTIPLE; iOS needs a Share Extension with App Groups. Source: https://pub.dev/packages/receive_sharing_intent
- `drift` 2.35.4: Android, iOS, Linux, macOS, web, Windows. Source: https://pub.dev/packages/drift
- `flutter_local_notifications` 23.0.3: Android action buttons; Linux and Windows supported. Ongoing/persistent flag not confirmed. Source: https://pub.dev/packages/flutter_local_notifications
- `super_drag_and_drop` 0.9.1: README says "very early stages... quite experimental." Source: https://pub.dev/packages/super_drag_and_drop
- `flutter_quill` 11.6.0: Delta format; Markdown via separate `markdown_quill`. Source: https://pub.dev/packages/flutter_quill

### Expo / React Native

- `expo-widgets`: launched alpha 2026-03-04 (iOS focus). Docs say SDK 56 promoted it to stable (2026-06-18 post), but the SDK 58 reference page gives no maturity label. Android is opt-in via `enableAndroid`, uses a Compose subset, requires `android.package` and a dev build (not Expo Go). Sources: https://expo.dev/blog/home-screen-widgets-and-live-activities-in-expo ; https://docs.expo.dev/versions/latest/sdk/widgets/
- `expo-sharing` receives Android ACTION_SEND and SEND_MULTIPLE via config plugin; iOS share extension is "currently experimental." Source: https://docs.expo.dev/versions/latest/sdk/sharing/
- `expo-sqlite`: Android, iOS, macOS, web (web needs COOP/COEP). SQLCipher on Android, iOS, macOS. Windows and Linux not listed. Source: https://docs.expo.dev/versions/latest/sdk/sqlite/
- Desktop: `react-native-windows` (Microsoft) is Windows-only; Linux not mentioned. Sources: https://github.com/microsoft/react-native-windows ; https://microsoft.github.io/react-native-windows/support . `react-native-macos` not checked **[unverified]**. No Linux RN target found.
- `react-native-live-markdown`: Android, iOS, web. No desktop. Source: https://github.com/Expensify/react-native-live-markdown
- `react-native-draggable-flatlist`: needs Reanimated and gesture-handler; platforms not stated. Source: https://github.com/computerjazz/react-native-draggable-flatlist

### Compose Multiplatform

- Latest seen: 1.12.0 (released 25 Aug 2026 per GitHub). New desktop window API v2 is experimental. Sources: https://github.com/JetBrains/compose-multiplatform/releases/tag/v1.12.0 ; https://kotlinlang.org/docs/multiplatform/whats-new-compose-112.html
- Minimum platforms: Android API 21, Windows 10, macOS 13, Ubuntu 20.04 (Linux). No per-platform stability labels found on the official compatibility page. Source: https://kotlinlang.org/docs/multiplatform/compose-compatibility-and-versioning.html
- Linux packaging: .deb and .rpm targets via the Compose Gradle plugin, built on the target OS. Source (search snippet, not fetched): https://github.com/JetBrains/compose-jb/blob/master/tutorials/Native_distributions_and_local_execution/README.md
- Glance (Android widgets): Android-only, "in active development," not interoperable with Compose UI, so widget code lives in the Android module, not shared code. Source: https://developer.android.com/develop/ui/compose/glance
- Share intent and notifications: native Android code (no shared API found) **[unverified]**.
- `Reorderable` 3.1.0: Android, iOS, desktop JVM, wasm, JS. Source: https://github.com/Calvin-LL/Reorderable
- `compose-rich-editor` 1.1.0: Android, iOS, desktop, web; `setMarkdown`/`toMarkdown`. Source: https://github.com/MohamedRejeb/compose-rich-editor (trigger system is experimental)

### PWA

- Web Share Target: experimental, not Baseline, must be installed. Files need POST + multipart. Source: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/share_target
- Chrome support: Android 76+, desktop 89+ (from Chrome dev guide search snippet, not fetched): https://developer.chrome.com/docs/capabilities/web-apis/web-share-target . Safari/Firefox support **[unverified]**.
- Android: URLs often arrive in `text` or `title`, not `url`. Source (search snippet): https://developer.chrome.com/docs/capabilities/web-apis/web-share-target
- No widget API in the manifest or Chromium found. `shortcuts` only add app-icon context menus. Source: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/shortcuts . Widget absence: not found in search **[unverified absence]**.
- OPFS: Chrome 86, Edge 86, Firefox 111, Safari 15.2. Chrome lacks folder rename/move. Source: https://web.dev/articles/origin-private-file-system
- dnd-kit: framework-agnostic, pointer/touch/keyboard sensors; web only. Source: https://github.com/clauderic/dnd-kit
- Milkdown: MIT, ProseMirror + Yjs + Remark, headless, no CSS. Source: https://milkdown.dev/

## 3. Sync

No framework in this set has a first-party cross-device sync engine. PowerSync client SDKs list JavaScript Web, React Native/Expo, Dart/Flutter, Kotlin, Swift, Capacitor (beta), Tauri (alpha), .NET (beta), Rust (beta), with no GA label for most. Source: https://docs.powersync.com/client-sdks/overview . Sync is an independent decision; each option needs a sync layer or CRDT library **[unverified fit]**.

## 4. Unverified or not covered

- Flutter Linux desktop stable status as of late 2026.
- react-native-macos status; any RN Linux path.
- Compose Multiplatform Linux stability label.
- Persistent or action-button quick-add notifications in Tauri, Expo, PWA.
- Community Tauri widget/share plugin maturity.
- Expo widget stability (docs conflict).
- Dioxus: not researched. From memory only, mobile support is experimental. Not verified.

## 5. Recommendation

Shortlist: Tauri 2 (Rust core, web UI, best web ecosystem for Kanban and markdown, full Linux/Windows desktop) and Compose Multiplatform (best native Android widget and share fit, real desktop JVM, Kotlin). Flutter is viable if you accept Dart. Expo fails the Linux requirement. PWA fails the widget and quick-add requirements.

Widget and share glue is native Kotlin in every viable option, so it does not decide the framework.

Next step: spike one week on the top two. Build (a) an Android AppWidget reading local DB, (b) a share intent that writes a task, (c) Kanban drag in a desktop build. Go with whichever passes all three with the least native code.

## 6. Sources

- https://v2.tauri.app/blog/tauri-20/
- https://v2.tauri.app/start/prerequisites/
- https://v2.tauri.app/plugin/
- https://github.com/tauri-apps/tauri/releases
- https://docs.flutter.dev/platform-integration/desktop
- https://docs.flutter.dev/platform-integration/linux/building
- https://www.infoworld.com/article/3649201/google-flutter-210-brings-stable-support-for-windows-targets.html
- https://pub.dev/packages/home_widget
- https://pub.dev/packages/home_widget/changelog
- https://pub.dev/packages/receive_sharing_intent
- https://pub.dev/packages/drift
- https://pub.dev/packages/flutter_local_notifications
- https://pub.dev/packages/super_drag_and_drop
- https://pub.dev/packages/flutter_quill
- https://expo.dev/blog/home-screen-widgets-and-live-activities-in-expo
- https://docs.expo.dev/versions/latest/sdk/widgets/
- https://docs.expo.dev/versions/latest/sdk/sharing/
- https://docs.expo.dev/versions/latest/sdk/sqlite/
- https://github.com/microsoft/react-native-windows
- https://microsoft.github.io/react-native-windows/support
- https://github.com/Expensify/react-native-live-markdown
- https://github.com/computerjazz/react-native-draggable-flatlist
- https://github.com/JetBrains/compose-multiplatform/releases/tag/v1.12.0
- https://kotlinlang.org/docs/multiplatform/compose-compatibility-and-versioning.html
- https://kotlinlang.org/docs/multiplatform/whats-new-compose-112.html
- https://developer.android.com/develop/ui/compose/glance
- https://github.com/Calvin-LL/Reorderable
- https://github.com/MohamedRejeb/compose-rich-editor
- https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/share_target
- https://developer.chrome.com/docs/capabilities/web-apis/web-share-target
- https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/shortcuts
- https://web.dev/articles/origin-private-file-system
- https://github.com/clauderic/dnd-kit
- https://milkdown.dev/
- https://docs.powersync.com/client-sdks/overview
