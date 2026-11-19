# PROTOTYPE: UI libs spike (throwaway, never merged)

Answers issue #27: week/day calendar, list/board drag-and-drop with touch in Tauri Android,
and Tiptap `[[` / `@` mentions, in one Svelte 5 build. Tabs: `?tab=calendar|dnd|editor`, dark toggle.

## Run

```sh
pnpm install && pnpm dev          # build + preview on :5199 (vite dev breaks svelte-tiptap, see notes)
# Android: reuse the tauri-android spike shell with this dist
cd ../../../orbit-tauri-android/spikes/tauri-android
cargo tauri android build --debug --apk --target x86_64 \
  --config '{"build":{"frontendDist":"<abs path>/spikes/ui-libs/dist"}}'
```

## Results (2026-11-19; Chrome desktop + emulator API 36, System WebView 133, Tauri 2.12.7)

| Check | Desktop | Android touch |
|---|---|---|
| FullCalendar 7.1.5 vanilla, week/day/month, themed by `--fc-classic-*` -> Orbit tokens, light/dark | PASS | PASS (Day view on narrow) |
| FC move event | PASS | PASS (long-press 300 ms, then drag) |
| FC resize event | not run | PASS (long-press handle; a quick drag scrolls instead) |
| svelte-dnd-action 0.9.80 list reorder | PASS | PASS (`delayTouchStart: 200`) |
| svelte-dnd-action board, cross-column + in-column order | PASS | PASS, auto-scrolls board to off-screen column |
| @thisux/sveltednd 0.8.0 list reorder | not run (HTML5 DnD, synthetic events can't drive it) | PASS |
| Swipe scrolls page, hold drags (both libs, `touch-action: auto`) | n/a | PASS |
| Tiptap 3.34 + svelte-tiptap 3.0.2, Mention `suggestions[]` with `@` and `[[` (allowSpaces) | PASS | PASS (adb text through Gboard) |

## Notes

- Schedule-X dropped on paper: v4 drag-and-drop is premium (`@sx-premium`), free plugin stuck on v3.7.3 (2026-01). Svelte wrapper stale.
- FullCalendar v7: theme is a JS plugin plus CSS (`fullcalendar/themes/classic` + `theme.css` + `palette.css`); class names are hashed, so style through CSS vars / class-name hooks only. Override vars with higher specificity than palette `:root`.
- svelte-tiptap ships `.svelte` without a `svelte` export condition: `optimizeDeps.exclude` is needed, which then duplicates prosemirror-state in vite dev ("different instances of a keyed plugin"). Production build fine. Real app: fix with dedupe/include config.
- Mentions are one node type; kind = `mentionSuggestionChar` (`@` or `[[`). Markdown serialization not tested.
- Not tested: Gboard composition mid-word on real device, Linux WebKitGTK, svelte-dnd-action keyboard DnD.
