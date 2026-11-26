# Changelog

## Unreleased (0.9.0)

### Features

- Snooze a task's reminder from the task drawer or the command palette (10 min, 30 min, 1 hour, 3 hours, tomorrow 9:00, Monday 9:00)
- Completing a task gives a short shake, a few specks of stardust and a light tick on phones, with the existing Undo toast; all of it follows the motion setting
- Linux desktop reminders have Complete and Snooze (10 min, 1 h) buttons, like the web and Android ones
- Android widgets: the + button opens the app's own capture window with the list's @Area or #label typed in; the list widget shows how many tasks are left

### Fixes

- Android widget and picker screens are readable in dark mode and use the app's palette; the native update and battery dialogs match
- Android: the task-complete tick works (the app now declares the vibrate permission)
- Sharing text to Orbit on Android confirms with a toast
- Command palette quick add shows what it understood and can open the full capture window
- Adding a page link in the editor uses an in-app dialog instead of the browser prompt
- "Add a task" in the dashboard widgets opens capture; add-field wording is consistent across screens
- The landing page lists the Windows installer
- Capture window and badge ring colours follow the theme tokens

## 0.8.0 - 2026-11-25

### Features

- Quick add reads relative times, at-times and day parts: in 5 minutes, in half an hour, today at 10pm, tomorrow morning, tonight, at noon
- Quick add reads richer dates: next fri, this weekend, end of month, mid jan, the 27th, dec 5th 2027, 27/1 (day/month)
- Deadline and duration in quick add: by friday, {dec 5}, for 2h, 3-4pm
- Recurrence: every other fri, every quarter, every 2nd wed, every last fri, every first workday, plus starting, until and for N weeks
- Reminder shortcuts: !later, !morning, !evening, !tonight; reminders take the same date phrases
- Visual option bar under every capture input (destination, date calendar, deadline, duration, priority, labels, goal, reminder)
- Overdue banner with bulk reschedule, postpone actions on tasks and one-step undo
- Duplicate task, copy task link, Completed screen and sidebar favorites
- Group and sort menus on Inbox, Area and Project lists; future occurrences of recurring tasks on the calendar
- Multi-line paste adds one task per line, with subtasks from indentation
- Filter text: overdue, no date, no time, recurring, subtask, created:, search: with wildcards, #label* patterns, view all
- Settings: turn date recognition off, choose what next week and weekend mean, several default reminders, due-count badge on the web app icon
- Usage guide in Settings with a live try-it box, and a quick add cheat sheet on the website
- CLI: orbit add - and --lines for many tasks, postpone, reschedule, snooze, overdue and export --md

### Bug fixes

- Keep the personal rebuild record out of the public guide
- A bare time that already passed today now means tomorrow

## 0.6.1 - 2026-11-24

### Bug fixes

- Re-provision desktop token when saved server differs

## 0.6.0 - 2026-11-24

### Bug fixes

- Allow wasm in bundled CSP so capture window renders
- Roadmap owner shows No area and disables Add when no areas
- Type views mock in palette test
- Remove-feed button variant; note future calendar scope

### Chores

- Remove the AUR package, scripts and docs

### Features

- Palette go-to actions for every screen
- First-run onboarding checklist
- Badge progress on locked badges
- Backup status and export panel in Settings
- ICal feed events, SSRF-safe fetch, poll and endpoints
- Show iCal feed events on Calendar and Dashboard, Settings panel
- Multiple calendar feeds with colors, visibility, local all-day dates
- Multi-calendar settings, Home Calendar widget, local all-day dates

## 0.5.0 - 2026-11-24

### Bug fixes

- Web black bar; profile page, badge icons, 30-day chart, breadcrumbs
- Task nodes show their task title and rename it
- Reload canvas after import; wrap tool dock on narrow screens

### Features

- Clearer roadmap canvas with help tips, convert kinds, full size
- Roadmap tool dock, border-touching arrows, tidy directions, richer import prompt

## 0.4.3 - 2026-11-24

### Bug fixes

- Friendly dates in Upcoming widget (#83)

### Chores

- Build the web demo in just check and pages via just demo
- Keep astro check off the built demo

### Documentation

- Mark v1 scope built, tag deferred items (#88)

### Features

- Android bar icons follow page theme (#86)
- In-browser web demo at /demo (#90)

## 0.4.2 - 2026-11-24

### Bug fixes

- Chevron-only area collapse, draw Android content under the nav bar

### Chores

- Build the server image natively per arch; center nav icons above the gesture bar

## 0.4.1 - 2026-11-24

### Bug fixes

- Key login lockout on forwarded client IP for loopback peers
- Reject plain http server URLs for non-local hosts in the app
- Allow plain http for Tailscale addresses
- Narrow the Android FileProvider paths

### Chores

- Keep .env files out of the docker build context
- Scope release workflow permissions to the jobs that need them
- Pin GitHub Actions to commit SHAs
- Remove unused deleteView and checkinsOf

### Documentation

- Consolidate env var tables in deploy and cli-install
- Document real contributor prerequisites and gate steps
- Record Android spike outcome in ADR 0001

### Features

- Add baseline security headers and trim Android remote grants
- Add ORBIT_SECURE_COOKIE to mark the web cookie Secure
- 30-day sliding web session cookie

### Performance

- Enable SQLite WAL and batch blob gc in one transaction
- Throttle purge and blob gc to once per 5 minutes
- Skip badge table scan for ops that cannot earn badges
- Fire sync change callback only on real changes
- Scan rows once when archiving ended routines
- Share one visible-rows scan across same-tick getters
- Lazy-load Calendar and Editor to shrink main bundle

## 0.4.0 - 2026-11-24

### Bug fixes

- Fill the strip under the Android bottom nav with the nav colour
- Reparent tasks locally when a section is deleted
- End guide redirects with a slash

### Documentation

- Restructure the guide into five sections with a page-shape lint
- Add current-commit latency rerun and measured verdict

### Features

- Build a Windows installer in the release workflow
- Toggle area on name click
- Move area and project menus to page headers, redesign glance widget
- Finish roadmap canvas bands, headings, code prompt and round-trip tests

## 0.3.0 - 2026-11-23

### Bug fixes

- Align task composer headers
- Normalize captured task titles and wrap legacy titles
- Simplify the new Area control
- Simplify CLI listing arguments
- Remove duplicate entity action rows
- Report desktop shell version in About
- Pass borrowed tray handle directly
- Clear graph migration test lints
- Forward --capture before generating Tauri context
- Hide view menu for builtins and add board complete control
- Check only tracked markdown in site verify
- Stop Android sync abort, capture-window IPC and login flash

### Documentation

- Add a Cloudflare Tunnel deployment path
- Structure the Orbit guide
- Dedupe install, deploy and CLI guides

### Features

- Make sidebar Areas collapsible
- Make CLI references and output usable
- Add scoped entity create menus
- Add the server version and upgrade contract
- Surface Web and PWA updates
- Add entity actions menus
- Handle desktop runtime updates
- Require Android shell updates
- Verify CLI and staged release updates
- Undo task completion across replicas
- Add compatible roadmap graph rows
- Add page editor commands and foldable sections
- Add offline desktop capture overlay
- Sync routine icons and improve Home widgets
- Sync a name and profile picture
- Edit roadmaps on a graph canvas
- Take band membership from graph nodes in phase progress
- Finish task sections with server-side reparent
- Add env-gated sync trace and latency harness
- Add Orbit roadmap code import and export
- Render roadmap list and board lenses from the graph
- Lint guide markdown and add site checks to just check

## 0.2.0 - 2026-11-21

### Bug fixes

- Apply Android insets on the content root and show retry page for unreachable server
- Restore sheet padding and make calendar view switches work

### Documentation

- CLI binaries ship with each release

### Features

- Sage accent palette and new app icon
- Redesign site with reactive orbit, tour video, quick-add demo and theme toggle
- Ship orbit CLI binaries with checksums, install script and CLI page
- Airier UI with larger text, balanced mobile tab bar and bigger tap targets
- Site scroll story with retina screens, new tour video, phone trio and updated copy

## 0.1.1 - 2026-11-21

### Bug fixes

- Orbit add applies quick-add syntax via shared client capture
- Keep WorkManager and app classes in android release build
- Desktop app opens server page on first launch

### Documentation

- Add project site, guide and README redesign
- Restructure deploy guide, auto-create data dir, add systemd unit

### Features

- Labelled Add task, Add area and Add project in sidebar

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
- Install only platform-tools in the android release job

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


