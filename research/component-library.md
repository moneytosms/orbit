# Svelte component library for Orbit UI (issue #17)

Date: 2026-11-19. Scope: Svelte 5, themable via CSS vars, Tauri 2 webview, AGPL-3.0 inbound.

**Recommendation: Bits UI (MIT, Svelte 5 runes-era, unstyled) as the primitive layer, styled with shadcn-svelte copy-in code mapped onto Orbit CSS tokens; add specialist libs for table, toast, DnD, editor and gantt. No single library covers Orbit's list.**

Why: Bits UI is the only candidate with the broadest verified primitive set (dialog, menus, popover, tabs, command with fuzzy scoring, calendar, date/time fields) and no styling to fight the token system. shadcn-svelte is built on it and exposes CSS-var theming. Skeleton, Flowbite and Carbon ship opinionated styles that conflict with runtime semantic tokens. Gaps (week/day calendar, Svelte gantt, markdown mentions, tree) need separate libs.

## 1. Per-need table

| need | library/component | Svelte 5 ready? | gap |
|---|---|---|---|
| command palette (cmdk-style) | Bits UI `Command` (`command-score` fuzzy scoring, `command-viewport`) [1][2]; shadcn-svelte `command` wraps it [3] | Yes (bits peer `svelte ^5.33.0`) [4] | Styling only; keyboard/grouping behaviour not tested here |
| dialogs / sheets / drawers | Bits UI `dialog`; shadcn-svelte `sheet`, `drawer`, `alert-dialog` [2][3] | Yes | none |
| popovers / menus | Bits UI `popover`, `dropdown-menu`, `context-menu`, `menubar`, `toolbar` [2] | Yes | none |
| date picker | Bits UI `date-picker`, `date-field`, `date-range-picker` [2]; Carbon `DatePicker` [5] | Bits: yes. Carbon: devDep `svelte ^5.57.3`, no peer [6] | none for dates |
| time picker | Bits UI `time-field`, `time-range-field` [2]; Carbon `TimePicker` [5] | Yes (Bits) | no time-picker popup verified |
| calendar month | Bits UI `calendar`, `range-calendar` [2] | Yes | none |
| calendar week / day | none verified. FullCalendar core (MIT, no Svelte wrapper found) [7]; Schedule-X `@schedule-x/svelte` [8] | Schedule-X wrapper stale (last modified 2025-10-20, peer `@schedule-x/calendar ^3`, core now 4.9.1) [8][9] | **gap**: no maintained Svelte 5 week/day view |
| data table (sort, column visibility) | TanStack Table `@tanstack/svelte-table` (headless) [10]; alt Carbon `DataTable` (sort, `ToolbarColumnVisibility`, search, density) [5] | TanStack: peer `svelte ^5.0.0`, devDep `^5.57.3` [10]. Carbon yes [6] | TanStack column visibility/sort not fetched from docs, unverified for this version. Carbon brings its own styling |
| tabs | Bits UI `tabs` [2]; Skeleton `tabs` [11]; Carbon `Tabs` [5] | Yes | none |
| toasts | `svelte-sonner` (peer `svelte ^5.0.0`) [12]; alt Skeleton `toast` (zag) [11] | Yes | Bits UI has no toast. `svelte-sonner` theming vars unverified |
| drag-and-drop lists/boards | `@thisux/sveltednd` (HTML5 + pointer events, opt-in keyboard) [13]; `svelte-dnd-action` (touch, keyboard, aria) [14]; `@dnd-kit-svelte/svelte` (peer `svelte ^5.40.0`) [15] | Yes for all three (peer ^5). `svelte-dnd-action` uses `on:` events per its README [14] | Board/kanban components not verified. Touch in Android WebView unverified |
| pragmatic-drag-and-drop | `@atlaskit/pragmatic-drag-and-drop` 4.0.0 [16] | No Svelte adapter found in repo tree [16] | **gap**: would need own Svelte wiring. Not recommended |
| tree view | Skeleton `tree-view` (zag) [11]; Melt-UI `Tree` builder (runes `.svelte.ts`) [17]; Carbon `TreeView` [5] | Skeleton yes (peer ^5.40). Melt yes but last release 2026-01-04 [17] | Melt stale. Skeleton styling tied to its tokens |
| timeline (events) | svelte-ux `Timeline` [18]; Flowbite `timeline` [19] | svelte-ux: peer range includes 5, no `.svelte.ts` runes files found [18] | Event timeline only |
| gantt (tasks + deps) | SVAR Svelte Gantt `@svar-ui/svelte-gantt` 2.8.3, MIT, peer `svelte ^5.0.0` [20] | Yes | Open-source edition; PRO edition (paid) has more automation features [21] |
| gantt alt | frappe-gantt 1.2.2 (vanilla, MIT) [22]; svelte-gantt 4.5.0 (Svelte 4 only) [23] | No (frappe: vanilla; svelte-gantt: peer `^4`) | needs wrapper for frappe |
| markdown editor with `[[` and `@` | Tiptap core 3.34.0 + `@tiptap/extension-mention` + `svelte-tiptap` 3.0.2 [24][25][26]. Suggestion `char` is a string, regex-escaped, so `[[` is accepted by source [27] | svelte-tiptap: peer `svelte ^5.0.0`, runes `.svelte.ts` present [25] | Multi-char trigger `[[` read from source only, not run. Wiki-link node needs custom work |
| markdown editor alt (plain-text, source mode) | CodeMirror 6 via `svelte-codemirror-editor` 2.1.0 (peer `svelte ^5.0.0`) [28]; `@codemirror/autocomplete` 6.20.4 for custom `[[`/`@` sources [29] | Yes | last release 2025-10-10 [28] |
| markdown editor alt (Lexical) | `svelte-lexical` 0.6.5 (peer `svelte ^5.0.0`) [30] | Yes | no mention/typeahead plugin verified in README |
| markdown editor alt (Milkdown) | Milkdown 7.23.0 [31] | No Svelte wrapper found in repo or npm (`@milkdown/svelte`, `svelte-milkdown` not on npm) [31][32] | **gap** |

## 2. Candidate summary

| library | Svelte 5 | last release | licence | theming via CSS vars | touch/a11y notes |
|---|---|---|---|---|---|
| Bits UI 2.21.0 [4] | peer `svelte ^5.33.0` + `@internationalized/date ^3.8.1` | 2026-11-11 [33] | MIT [4] | Unstyled, you style with any CSS [34] | Says "accessible"; ARIA and touch details not verified |
| shadcn-svelte 1.8.1 (copy-in, built on Bits) [3][35] | peer `svelte ^5.0.0` [36] | npm 2026-11-11; GH release 2026-11-12 [35] | MIT [37] | Yes: `--background`, `--foreground`, `--primary`, `--card`, `--popover`, oklch values [38] | Code lives in Orbit repo once copied (you own it). Inherits Bits a11y |
| Skeleton 5.0.5 (`skeleton-svelte`) [11][39] | peer `svelte ^5.40.0`, tailwind `^4.0.0` [39] | npm 2026-11-17; GH release 5.0.4 2026-11-08 [40] | MIT [39] | Yes: `--color-*` CSS custom properties via `@theme`, themes override vars [41] | Built on zag. A11y claim not verified here. Skeleton Plus is a separate paid set |
| Flowbite-Svelte 1.33.1 [19][42] | peer `svelte ^5.40.0`, tailwind `^4.1.4` [42] | GH release 2026-04-07 (stale ~7 months) [42] | MIT [43] | Tailwind v4 classes via `@plugin flowbite`, theme in `app.css` [44]. Var mapping unverified | Kanban uses native HTML5 `draggable` [45]; touch unverified. Blocks/admin packages licence unverified |
| Carbon Components Svelte 0.114.3 [5][6] | devDep `svelte ^5.57.3`, no peer declared [6] | GH release 2026-11-18 [5] | Apache-2.0 [6] | Yes: `--cds-*` CSS variables on `:root` in 5 theme sheets [46] | Claims WCAG 2.1 AA, keyboard/screen reader [46]. Carbon look is strong; conflicts with Orbit tokens |
| Melt-UI next-gen 0.44.0 [17] | peer `svelte ^5.30.1` [17] | GH release 2026-01-04 (stale ~10 months) [17] | MIT [17] | Unstyled | Headless. Legacy `@melt-ui/svelte 0.86.6` last published 2025-03-28 [47] |
| svelte-material-ui 9.0.6 [48] | README: "SMUI v8 and higher requires Svelte 5" [48] | npm 2026-10-14; repo pushed 2026-10-16; no GH releases [48] | Apache-2.0 + MIT parts (MDC-Web) [48] | Sass/MDC theming. Runtime CSS var path unverified | Material Design look, not a fit for token theming |
| svelte-ux 1.0.11 [18] | peer `^3.56 \|\| ^4 \|\| ^5` [18]; no runes files found | GH release 2026-08-20 [18] | MIT [18] | Unverified | Broad set: DateField, DateRange, Table, Timeline, TreeList, QuickSearch, Dialog, Drawer. Legacy store API |
| Ark UI (`@ark-ui/svelte` 5.25.1) [49] | peer `svelte >=5.29.0` [49] | GH release 2026-11-02 [49] | MIT [49] | Unverified | Zag-based. Component list not fetched |
| svelte-tiptap 3.0.2 [25] | peer `svelte ^5.0.0`, `@tiptap/core ^3.0.0` [25] | npm 2026-10-02; repo pushed 2026-11-16 [25] | MIT [25] | Unstyled editor, CSS yours | Mobile on Android WebView unverified |
| svelte-dnd-action 0.9.80 [14] | peer `>=3.23.0 \|\| ^5.0.0-next.0` [14] | npm 2026-10-05; GH tag not checked | MIT [14] | Not applicable | Touch, keyboard, aria, screen reader instructions [14] |
| @thisux/sveltednd 0.8.0 [13] | peer `svelte ^5.0.0` [13] | GH release 2026-09-01 [13] | MIT [13] | Not applicable | HTML5 + Pointer Events, opt-in keyboard [13] |
| @dnd-kit-svelte/svelte 0.2.0 [15] | peer `svelte ^5.40.0` [15] | GH release 2026-11-05 [15] | MIT [15] | Not applicable | npm name mismatch: `@dnd-kit-svelte/core` 0.0.11 (2025-07-22) and `svelte-dnd-kit` deprecated [50] |
| SVAR Svelte Gantt 2.8.3 [20][21] | peer `svelte ^5.0.0` [20] | npm 2026-11-08 [20] | MIT (open-source edition) [51] | "flexible CSS styling" per README [21] | PRO edition is paid, not evaluated |
| TanStack Table 9.3.2 [10] | peer `svelte ^5.0.0` [10] | GH tag 2026-11-14 [10] | MIT [10] | Headless | n/a |
| svelte-sonner 1.2.3 [12] | peer `svelte ^5.0.0` [12] | npm 2026-11-02 [12] | MIT [12] | Unverified | Not verified |
| Lexical 0.53.0 / svelte-lexical 0.6.5 [30][52] | svelte-lexical peer `^5.0.0` [30] | svelte-lexical GH 2026-07-15 [30] | MIT [30][52] | Unstyled | Mention plugin not verified |
| Schedule-X `@schedule-x/svelte` 3.0.0 [8] | peer `svelte ^4 \|\| ^5` [8] | npm modified 2025-10-20 [8] | MIT [8] | Unverified | Stale. Core 4.9.1 (2026-09-29) is Preact-based [9] |
| frappe-gantt 1.2.2 [22] | none (vanilla) | npm 2026-02-25; GH tag v1.0.3 2025-02-03 [22] | MIT [22] | Unverified | No Svelte wrapper |

## 3. Licences and AGPL-3.0 compatibility

Permissive inbound licences (MIT, BSD-3-Clause, ISC, 0BSD, Apache-2.0) are compatible with AGPL-3.0 outbound. FSF lists Apache 2.0 as compatible with GPLv3 [53]. Flags:

- **Apache-2.0**: Carbon [6], svelte-material-ui [48], pragmatic-drag-and-drop [16]. Compatible. Keep NOTICE/attribution. Pragmatic's npm licence field reads `NOASSERTION`; repo LICENSE is Apache-2.0 [16].
- **LIL** (non-OSI): `svelte-virtual-list` 3.0.1 has `license: LIL` and repo at a corporate host (`github.devops.utc.com`) [54]. **Do not use** until terms are read.
- **No licence field**: `svelte-calendar` 3.1.6 [55], `svelte-tree` 0.7.0 [56], `svelte-mention` 1.0.2 [57]. Treat as all rights reserved. **Do not use.**
- **Stale but permissive**: `svelte-sortable` 0.1.0 (MIT, peer `svelte ^3`, 2022) [58], `svelte-tabs` 1.1.0 (MIT, last 2022) [59], `svelte-timeline` 0.1.2 (BSD-3, 2022) [60], `svelte-select` 6.0.0 (ISC, 2026-08) [61] ok, `svelte-command` 3.0.70 (0BSD) [62] ok.
- **Paid/proprietary tiers in MIT repos** (not used by Orbit unless bought): Skeleton Plus [63], SVAR PRO Gantt [21], Tiptap Pro extensions (not checked, see unverified).
- Flowbite's `flowbite-svelte-blocks` and `flowbite-svelte-admin-dashboard` are separate packages; their licences are unverified.

## 4. Unverified

- Tauri 2 Android System WebView and Linux WebKitGTK behaviour for any candidate. Not tested.
- Touch and drag behaviour on Android for Flowbite kanban and Bits-based DnD. Not tested.
- TanStack column visibility and sort API for v9. Docs not fetched.
- Bits UI command fuzzy scoring behaviour (only source file path confirmed, not run).
- Tiptap `[[` multi-char trigger: source reads correct, not run. Wiki-link node not checked.
- Carbon Svelte 5 runtime: devDep only, no peer; uses legacy store API.
- svelte-ux and svelte-dnd-action Svelte 5 runtime behaviour: no runes files, legacy `on:` API.
- Ark UI component list (tree, toast, date picker, calendar). Not fetched.
- svelte-sonner, Skeleton, Flowbite, Ark: accessibility claims. Not verified.
- Week/day calendar: no maintained Svelte 5 library confirmed. FullCalendar `@fullcalendar/core` 7.1.5 exists [7]; no `@fullcalendar/svelte` on npm (NOTFOUND). Premium scheduler licence not checked.
- Milkdown and svelte-lexical mention/typeahead: no Svelte-specific mention API verified.
- Melt-UI latest release (2026-01-04) vs npm `@melt-ui/svelte` (2025-03-28): unclear which is maintained. Not resolved.
- `@dnd-kit-svelte/svelte` on npm: not checked directly (only repo and deprecated `svelte-dnd-kit`).
- Whether Flowbite `Datepicker` and `Timeline` are Svelte 5 runes-based (peer range only).

## 5. Sources

[1] https://github.com/huntabyte/bits-ui/tree/main/packages/bits-ui/src/lib/bits/command
[2] https://github.com/huntabyte/bits-ui (component directory `packages/bits-ui/src/lib/bits/`)
[3] https://github.com/huntabyte/shadcn-svelte (docs/content/components/command.md, data-table.md, calendar.md)
[4] https://registry.npmjs.org/bits-ui ; https://github.com/huntabyte/bits-ui/blob/main/packages/bits-ui/package.json
[5] https://github.com/carbon-design-system/carbon-components-svelte (src/DataTable, src/DatePicker, src/TreeView, README)
[6] https://registry.npmjs.org/carbon-components-svelte ; https://github.com/carbon-design-system/carbon-components-svelte/blob/main/package.json
[7] https://registry.npmjs.org/@fullcalendar/core ; https://registry.npmjs.org/@fullcalendar/svelte (NOTFOUND)
[8] https://registry.npmjs.org/@schedule-x/svelte
[9] https://registry.npmjs.org/@schedule-x/calendar
[10] https://registry.npmjs.org/@tanstack/svelte-table ; https://github.com/TanStack/table
[11] https://github.com/skeletonlabs/skeleton (packages/skeleton-svelte/src/components/) ; https://registry.npmjs.org/@skeletonlabs/skeleton-svelte
[12] https://registry.npmjs.org/svelte-sonner ; https://github.com/wobsoriano/svelte-sonner
[13] https://github.com/thisuxhq/sveltednd ; https://registry.npmjs.org/@thisux/sveltednd
[14] https://github.com/isaacHagoel/svelte-dnd-action ; https://registry.npmjs.org/svelte-dnd-action
[15] https://github.com/HanielU/dnd-kit-svelte (packages/svelte/package.json)
[16] https://github.com/atlassian/pragmatic-drag-and-drop ; https://registry.npmjs.org/@atlaskit/pragmatic-drag-and-drop
[17] https://github.com/melt-ui/next-gen (packages/melt/src/lib/builders/) ; https://registry.npmjs.org/melt
[18] https://github.com/techniq/svelte-ux (packages/svelte-ux/src/lib/components/) ; https://registry.npmjs.org/svelte-ux
[19] https://github.com/themesberg/flowbite-svelte (src/lib/timeline, src/lib/kanban)
[20] https://registry.npmjs.org/@svar-ui/svelte-gantt
[21] https://github.com/svar-widgets/gantt (apps/widget/readme.md)
[22] https://github.com/frappe/gantt ; https://registry.npmjs.org/frappe-gantt
[23] https://registry.npmjs.org/svelte-gantt
[24] https://registry.npmjs.org/@tiptap/core ; https://github.com/ueberdosis/tiptap
[25] https://github.com/sibiraj-s/svelte-tiptap ; https://registry.npmjs.org/svelte-tiptap
[26] https://registry.npmjs.org/@tiptap/extension-mention
[27] https://github.com/ueberdosis/tiptap (packages/suggestion/src/suggestion.ts, findSuggestionMatch.ts)
[28] https://github.com/touchifyapp/svelte-codemirror-editor ; https://registry.npmjs.org/svelte-codemirror-editor
[29] https://registry.npmjs.org/@codemirror/autocomplete
[30] https://github.com/umaranis/svelte-lexical ; https://registry.npmjs.org/svelte-lexical ; https://registry.npmjs.org/lexical
[31] https://github.com/Milkdown/milkdown ; https://registry.npmjs.org/@milkdown/kit
[32] https://registry.npmjs.org/@milkdown/svelte (NOTFOUND); https://registry.npmjs.org/svelte-milkdown (NOTFOUND)
[33] https://github.com/huntabyte/bits-ui/releases (tag bits-ui@2.21.0)
[34] https://bits-ui.com (README: "headless components for Svelte"; https://github.com/huntabyte/bits-ui/blob/main/README.md)
[35] https://github.com/huntabyte/shadcn-svelte/releases (tag @shadcn-svelte/registry@0.1.2)
[36] https://registry.npmjs.org/shadcn-svelte
[37] https://github.com/huntabyte/shadcn-svelte/blob/main/LICENSE ; packages/cli/package.json
[38] https://github.com/huntabyte/shadcn-svelte/blob/main/docs/content/theming.md
[39] https://github.com/skeletonlabs/skeleton/blob/main/packages/skeleton-svelte/package.json
[40] https://github.com/skeletonlabs/skeleton/releases (tag @skeletonlabs/skeleton-svelte@5.0.4)
[41] https://github.com/skeletonlabs/skeleton/blob/main/sites/skeleton.dev/src/content/docs/design/themes.mdx
[42] https://github.com/themesberg/flowbite-svelte/releases (v1.33.1) ; https://github.com/themesberg/flowbite-svelte/blob/main/package.json
[43] https://github.com/themesberg/flowbite-svelte/blob/main/LICENSE
[44] https://github.com/themesberg/flowbite-svelte/blob/main/src/app.css
[45] https://github.com/themesberg/flowbite-svelte/blob/main/src/lib/kanban/KanbanCard.svelte
[46] https://github.com/carbon-design-system/carbon-components-svelte (README: themes and CSS variables, WCAG 2.1 AA)
[47] https://registry.npmjs.org/@melt-ui/svelte
[48] https://github.com/hperrin/svelte-material-ui (README, LICENSE section) ; https://registry.npmjs.org/svelte-material-ui
[49] https://github.com/chakra-ui/ark (tag @ark-ui/svelte@5.25.1) ; https://registry.npmjs.org/@ark-ui/svelte
[50] https://registry.npmjs.org/svelte-dnd-kit ; https://registry.npmjs.org/@dnd-kit-svelte/core
[51] https://github.com/svar-widgets/gantt/blob/main/license.txt
[52] https://github.com/facebook/lexical
[53] https://www.gnu.org/licenses/license-list.html#apache2
[54] https://registry.npmjs.org/svelte-virtual-list ; https://github.devops.utc.com/40000519/virtual-list.git (repo, not accessed)
[55] https://registry.npmjs.org/svelte-calendar
[56] https://registry.npmjs.org/svelte-tree
[57] https://registry.npmjs.org/svelte-mention
[58] https://registry.npmjs.org/svelte-sortable
[59] https://registry.npmjs.org/svelte-tabs ; https://github.com/joeattardi/svelte-tabs
[60] https://registry.npmjs.org/svelte-timeline
[61] https://registry.npmjs.org/svelte-select ; https://github.com/rob-balfre/svelte-select
[62] https://registry.npmjs.org/svelte-command ; https://github.com/arlac77/svelte-command
[63] https://github.com/skeletonlabs/skeleton (sites/plus.skeleton.dev)
