# Changelog

## 0.1.0 - 2026-11-20

### Bug fixes

- Show focus ring, settings errors and login error codes
- Keep words after every out of due date (#39)
- Close audit gaps in views, recurrence and sub-goals
- Order new milestones and phases after their siblings
- Import undated mermaid flowchart nodes without a due date
- Make roadmap import tests independent of item order
- Resolve e2 merge, import release recipes
- Dedupe merge leftovers
- Repair merged widget manifest blocks
- Restore orbit-cli dependencies lost in merge
- Serve web app from server; android first-run server page
- Let remote server origin call app commands on android
- Android launcher page redirects after load; capture applies @place
- Widgets follow system theme; trashed goals leave widgets; check-in syncs at once
- Gate Android commands to trusted origin; goals widget lists active only; widget dot uses Area colour
- Use Todoist's published colour palette for import
- Build the web stage with Node 22 in the server image
- Keep Project id narrowed for the banner
- Widgets use the app accent on Android 12+; name quick-add and list widgets in the picker
- Forward routine id through notification service worker

### Chores

- Init repo with agent docs and AGPL-3.0 license
- Ignore agent worktrees
- Add prettier config and just fmt
- Refresh Cargo.lock after merges
- Ignore unused generated icon sizes

### Documentation

- Add glossary and stack ADR
- Add phased scope draft with open questions
- Trim scope to cheap features and add routines
- Add areas, inbox, calendar and editable views to scope
- Restore v1 reminders on phone and desktop
- Fix scope status line
- Record item tree domain model
- Add what's next terms to glossary
- Record views model and drop roadmap-as-view
- Add roadmap and phase to glossary
- Move search into v1 as command palette
- Add conflict copy to glossary
- Add stardust levels and targets to glossary
- Add reminder to glossary
- Add device to glossary
- Add attachment to glossary
- Add badge, patch and gamification to glossary
- Add design system spec from screens prototype
- Add android list widget to v1 scope
- Add home-screen widgets to glossary and scope
- Kotlin calls rust over jni in stack adr
- Widen link targets in glossary
- Add project overview and commands to claude.md
- Document preinstalled toolchain in CLAUDE.md
- Note warm sccache, Android targets and Gradle cache
- Add #68 android checklist and fix #70 widget accent note
- Add server deployment guide and compose examples
- Add client install guide and refresh README, scope, releasing
- Export command usage and data dir ownership in deploy guide

### Features

- Add workspace skeleton and just check gate
- Add inbox task tracer through wasm core, outbox and sync
- Add field lww, rejection restore and multi-device sync harness
- Add login, devices and app shell with orbit tokens (#34, #36)
- Add item tree, fractional order and item.move (#37)
- Add nightly snapshots and backup/restore cli (#73)
- Add task fields, labels and settings record (#38)
- Add quick-add parser with wasm export (#39)
- Add theme palettes, mapping and contrast grades (#42)
- Add export zip and disk-free routes (#73)
- Add appearance themes, export button and disk warning (#42, #73)
- Add area tree sidebar and drag reorder (#37)
- Add task detail drawer, row chips and synced appearance (#38, #42)
- Add quick-add composer with chips and q dialog (#39)
- Drag tasks onto sidebar to reparent, sync accent reset (#37, #42)
- Add recurring tasks with every and every! (#40)
- Add goals, milestones and sub-goals (#43)
- Add pages with markdown editor and conflict copies (#54)
- Add trash, archive and purge (#41)
- Add views with filter tree and built-ins (#45)
- Add view screen, filter editor and sidebar pins (#45)
- Add board and table layouts (#47, #50)
- Add typed filter syntax (#46)
- Add routines and check-ins (#44)
- Add what's next ranking and screen (#51)
- Add links and backlinks (#55)
- Add timeline layout and horizon view (#49)
- Add reminders with web notifications (#64)
- Add attachments with blob storage and cache (#56)
- Add roadmaps with phases and progress (#59)
- Pick a view's kind incl. roadmaps and phases (#59)
- Add search index and command palette (#57)
- Add keyboard shortcuts and cheat sheet (#58)
- Add area pages (#53)
- Add stardust, levels and targets (#62)
- Add calendar layout (#48)
- Add dashboard widget grid (#52)
- Add linux desktop shell with tray, capture and reminders (#65)
- Add roadmap canvas with gates and step links (#60)
- Add roadmap timeline, 30-day builder and mermaid import (#61)
- Add badges, patches and gamification switch (#63)
- Add orbit CLI core with login, sync and task commands
- Add android app with jni bridge, keystore token and workmanager sync
- Add todoist import (#72)
- Add cli help examples, docs, guide and mcp server (#67)
- Add android jni seams for reminders and widgets
- Add release pipeline (#74)
- Add android reminders (#69)
- Add row data to widget jni exports
- Rearm reminders after sync
- Add android quick-add and list widgets (#70)
- Add android routines and goal widgets (#71)
- Colour field on areas, projects, labels; import Todoist colours (#72)
- Widget row/goal taps open in app; fix widget refresh, config insets, check-in sync, picker dupes
- Orbit logo and icons; icon, cover and colour on Area and Project
- Emoji icons and covers for Areas and Projects; fix mention chip crash; centre mobile nav add button
- Text size, typeface, line spacing and motion settings
- Widget picker previews with sample data; shared rounded widget panel
- Redesign widgets with the app's tokens: accent progress, chips, headers, ring check-ins
- Restructure Settings into sections with progressive disclosure; empty states across screens
- Widget polish: day progress, row dividers, behind-pace bar, streak only when set; inset and status-bar fixes
- Add sub-goal creation button on Goals screen
- Dashboard default, backlinks on screens, project 30-day plan
- Archive ended Routines on sync and add Done to Routine reminders
- Back up blobs and export pages and attachments (#73)
- Desktop server connect and Tauri updater

### Refactoring

- Widget config base class, replica smells; feat: goal counts, day-start refresh, unreachable-server page, README


