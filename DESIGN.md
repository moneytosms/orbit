# Orbit Design System

> **Source of truth.** Every value below comes from `prototypes/screens/index.html` on branch `prototype/screens` (commit `ac89c9c`), cited as `P:<line>`. Anything the file doesn't define is marked **TBD**. Orbit has no other theme or token file yet: `prototypes/themes/index.html` (branch `prototype/themes`) only explored palette strategies, and [Theme system and featured themes](https://github.com/moneytosms/orbit/issues/16) settled that Featured themes swap the accent on this base.
>
> **Notion reference material.**
> - *Accessed:* Notion's public brand guidelines page exists (notion.notion.site, "Notion Brand Guidelines"). It covers marks and trademark use, not UI tokens. It was found but not read in full.
> - *Not available:* Notion publishes no official product design system, token set or component spec. "Notion design system" sites (shadcn.io/design/notion, open-design.ai) are third-party extractions, and so is the pasted reference spec.
> - *How it's used here:* Notion's *rules* (one structural accent, hairline-first depth, whitespace as the grouping device, a 4/8px spacing rhythm, a tight-to-generous type hierarchy) are used as principles. **No Notion values, font names or tokens are used.**

## Overview

Orbit is a planner you open many times a day, so it should feel like a tidy desk in warm afternoon light. The page canvas is warm paper, `{colors.bg}` (#fbfaf8). Raised things such as widgets, cards, fields and popovers sit on pure white `{colors.surface}` (#ffffff), so figure and ground separate without heavy borders. The sidebar uses a second, slightly darker warm neutral, `{colors.side}` (#f5f3ef). Type is **Figtree** in warm near-black `{colors.text}` (#2a2723). It's a humanist geometric sans: friendlier than a grotesk, but still a working UI face.

Orbit has **exactly one structural accent**, terracotta `{colors.accent}` (#c2552f). It marks the primary action, the current selection, focus, Today, and the one thing that matters in a widget header. That's where Orbit departs from Notion's monochrome-plus-blue: the accent is warm, and a small **Area palette** (`{colors.career}`, `{colors.college}`, `{colors.learning}`, `{colors.home}`) adds identity. The Area colours only ever appear as 15%-tinted icon tiles, 7px dots and timeline bars. They never fill a button, a background band or body text. Status uses a muted semantic set (`ok`, `warn`, `danger`), always as soft-tint chips.

Depth is a 1px hairline plus a faint two-layer warm shadow. Widgets and panels round at 14px, cards and columns at 10–12px, controls at 8px. Pills are reserved for switches and status dots. Nothing glows or uses gradients. Personality comes from colour used sparingly, real icons (Lucide, 1.8 stroke) and well-made data components (heatmap, pace bar, roadmap canvas), not decoration.

**Key Characteristics:**
- Warm paper canvas `{colors.bg}` with white `{colors.surface}` for anything you can act on
- Figtree in `{colors.text}`, one family, weights 400/500/600/700, gentle negative tracking only at 22px and up
- One structural accent, terracotta `{colors.accent}`: actions, selection, focus, Today
- A muted Area palette used only as tints, dots and bars, never as fills or CTAs
- Hairline + two-layer warm shadow (`{shadow.1}`), with a deeper stack (`{shadow.2}`) only for floating layers
- A full light and dark set: every token has a dark value, chosen from the use scene, not a category default
- Data components (heatmap, pace bar, gated roadmap canvas) carry the product's character

## Colors

> Defined as CSS custom properties on `:root` (P:11) with dark overrides on `[data-theme=dark]` (P:21). Contrast ratios were computed from these hex values.

### Brand & Accent
- **Terracotta** (`{colors.accent}`: #c2552f light / #e2835e dark): the only structural accent. Primary button fill, selected nav icon, active segment, focus ring, Today markers, roadmap "in progress". 4.53:1 on white.
- **On-accent** (`{colors.accent-ink}`: #ffffff / #1b1a18): text and icons on accent fills. 4.53:1 light, 6.33:1 dark.
- **Accent soft** (`{colors.accent-soft}`: #f8ebe4 / #3a281f): selected-state tint for chips, segments, board drop targets and the selected routine row.
- **Accent line** (`{colors.accent-line}`: #e9b9a5 / #7a4631): borders on accent-soft surfaces, link underlines, focused-field border.

**Area palette** (identity only: tinted icon tiles, dots, timeline/pace bars):
- **Career** (`{colors.career}`: #b37419 / #dba24c)
- **College** (`{colors.college}`: #3b6cb0 / #79a2e0), also Priority 3
- **Learning** (`{colors.learning}`: #468253 / #82bb8d)
- **Home** (`{colors.home}`: #ad4f66 / #dc8ba0)

### Surface
- **Paper** (`{colors.bg}`: #fbfaf8 / #1b1a18): page canvas, roadmap canvas (with a 22px dot grid in `{colors.border-strong}`).
- **Side** (`{colors.side}`: #f5f3ef / #211f1d): sidebar, the second neutral layer.
- **Surface** (`{colors.surface}`: #ffffff / #252321): widgets, panels, cards, fields, popovers, palette.
- **Sunk** (`{colors.sunk}`: #f2efea / #1e1d1b): board columns, segmented-control track, chips, empty heatmap cells, locked roadmap phases.
- **Hover** (`{colors.hover}`: rgba(60,45,25,.055) / rgba(255,240,220,.06)) and **Press** (`{colors.press}`: rgba(60,45,25,.09) / rgba(255,240,220,.1)): row hover and selected-row fills.
- **Hairline** (`{colors.border}`: #ebe7e0 / #33302c) and **Hairline strong** (`{colors.border-strong}`: #dcd5ca / #45413b): dividers and card borders; strong is used for hover borders, unchecked controls and dashed edit outlines.
- **Scrim**: rgba(30,22,14,.32) (P:311), behind the palette, quick add, sheets and the phone drawer.

### Text
- **Ink** (`{colors.text}`: #2a2723 / #ede8e0): headings and body. 14.25:1 on paper.
- **Muted** (`{colors.muted}`: #6b665e / #a8a197): secondary copy, counts, metadata. 5.46:1 on paper, 5.14:1 on side.
- **Icon** (`{colors.icon}`: #8a847a / #8f897f): idle icon strokes only. **Not for text** (below 4.5:1).

### Semantic
- **OK** (`{colors.ok}`: #3d7a4d / #7fc08c, soft #e5f0e7 / #1f2f23): done ticks, check-ins, heatmap, "On track".
- **Warn** (`{colors.warn}`: #9a5d08 / #e2ab5a, soft #f7ecd8 / #33291a): Priority 2, streak flames, milestones, "Behind pace", callouts.
- **Danger** (`{colors.danger}`: #bb3a2a / #ef8070, soft #f9e6e2 / #3a2320): Priority 1, overdue.
- Priority colours map onto these: P1 danger, P2 warn, P3 college, P4 border-strong (P:125).

**Known contrast gaps (to fix):** accent text on accent-soft is 3.88:1 and ok text on ok-soft is 4.39:1, both below 4.5:1 for 12.5px chips. A darker on-tint text token for each is **TBD**.

## Typography

### Font Family
**Figtree** (Google Fonts, weights 400/500/600/700, P:7), falling back to `system-ui, sans-serif` (P:29). One family carries everything. Numbers use `font-variant-numeric: tabular-nums` through the `.num` utility (P:37). There's no monospace and no display face. Self-hosting the font for the Tauri/offline builds is **TBD**.

### Hierarchy

| Token | Size | Weight | Line Height | Letter Spacing | Use | Source |
|---|---|---|---|---|---|---|
| `{typography.page-title}` | 28px (23px phone) | 700 | 1.2 | −0.015em | Screen titles ("Thursday 19 November", "Rust roadmap") | P:80, P:357 |
| `{typography.stat}` | 22px | 700 | inherit (1.5) | −0.01em | Routine stat values | P:291 |
| `{typography.doc-h2}` | 20px | 600 | inherit | 0 | Page headings | P:267 |
| `{typography.detail-title}` | 17px | inherit (400) | inherit | 0 | Roadmap step title | P:239 |
| `{typography.doc-body}` | 16px | 400 | 1.7 | 0 | Page body (max 70ch) | P:266 |
| `{typography.section}` | 16px | 600 | inherit | 0 | Panel headings ("Milestones", "Pace") | P:249 |
| `{typography.input-lg}` | 15–16px | 400 | inherit | 0 | Composer (15), palette input (16) | P:156, P:316 |
| `{typography.widget-title}` | 14.5px | 600 | inherit | 0 | Widget headers | P:143 |
| `{typography.body}` | 14px | 400 | 1.5 | 0 | UI default: rows, nav, buttons (500) | P:29 |
| `{typography.meta}` | 12.5–13px | 400/600 | inherit | 0 | Chips, hints, sidebar section labels (600), counts | P:98 |
| `{typography.micro}` | 11.5–12px | 500 | inherit | 0 | Keyboard hints, tab-bar labels, axis ticks | P:55 |

### Principles
- **Tight scale, clear steps.** Product UI, not marketing: the scale steps by roughly 1.14 at body size, and weight does most of the work (600 for structure, 700 only for page titles and stats).
- **Sentence case everywhere.** No uppercase eyebrow labels. Section labels are 12.5px/600 muted sentence case.
- **Negative tracking only at 22px and up.**
- **Readable prose.** Page body runs at 16/1.7 with a 70ch measure.

## Layout

### Spacing System
- **Rule (from the Notion reference, used as a principle):** a 4px base unit, with 8px as the dominant rhythm. Groups stay tight, separations stay generous, and headings get more space above than below.
- **Scale as used in code** (counted across `padding`/`gap`/`margin` in P:10–380):

| Token | Value | Typical use |
|---|---|---|
| `{spacing.2xs}` | 2px | Segment button gap, `.props` row gap |
| `{spacing.xs}` | 4px | Day-strip gap, icon-button clusters |
| `{spacing.sm}` | 8px | Default inline gap, column padding, popover padding |
| `{spacing.md}` | 12px | Card/tick padding (10–12), group gaps |
| `{spacing.lg}` | 16px | Dashboard grid gap, panel and detail padding, widget side padding |
| `{spacing.xl}` | 24px | Empty-state padding, phone page bottom |
| `{spacing.2xl}` | 40px | Desktop page gutter (`.wrap` P:77) |
| `{spacing.3xl}` | 64px | Page bottom padding |

- **Off-scale values in code, to normalise:** 3, 5, 6, 7, 9, 10, 11, 13, 14, 22px. The most common are gap 6px (16 uses), 10px (10), 9px (4); padding `0 9px`, `0 10px`, `13px 16px 6px`. Each should snap to 4/8/12/16 when the prototype becomes real code. Until then, the scale above is the target and these are the current truth.
- **Control heights:** 22 chip, 26 icon-sm, 28 seg/sm-button, 32 button/icon, 34 field/sidebar button, 38 inline add, 42 task row min, 52 tick min, 48/52 top bar, 72 tab bar.

### Grid & Container
- **App shell:** 256px sidebar + fluid main (P:47). Top bar 48px with breadcrumbs.
- **Content column:** `.wrap` max 1160px with 40px gutters (P:77); `.wrap.narrow` 860px for Goal, Page and Upcoming (P:78); the roadmap goes full width.
- **Dashboard:** a 12-column grid with 16px gaps (P:139). Widget sizes are S=4, M=6, L=8, XL=12 columns, and users can change them.
- **Board:** 272px columns, horizontal scroll with snap (P:206).
- **Roadmap shell:** canvas + 300px detail panel (P:238).

### Whitespace Philosophy
Whitespace groups things; rules and boxes don't. Rows inside a list are split by an inset hairline that disappears on hover. Groups are split by space plus a 600-weight label. Widgets are the only boxes on the dashboard and are never nested.

### Responsive Strategy

#### Breakpoints
| Name | Width | Key Changes | Source |
|---|---|---|---|
| Desktop | > 700px viewport | Sidebar visible, 12-col dashboard, side detail panels | P:47 |
| Phone | ≤ 700px (auto) / `.phone` class, designed at 390px | Sidebar becomes a drawer, bottom tab bar with a centre add button, single-column widgets, detail panels become bottom sheets | P:346–379, script `S.device` |
| Tablet | **TBD** | Not designed yet | — |
| Wide | **TBD** | Container caps at 1160px; no wide-specific layout | — |

#### Touch Targets
Phone tab-bar items fill a 72px bar split into five columns. The add button is 48×48. Routine ticks are at least 52px tall. Desktop rows are 42px. Desktop icon buttons are 32px (26px for small ones), so a 44px phone minimum for those is **TBD**.

#### Collapsing Strategy
The sidebar slides in as a drawer (scrim + translate). Task-row meta wraps under the title. The board shows 84%-width columns so the next one peeks in. The roadmap detail panel and quick add become bottom sheets. Stats go from 4 columns to 2×2.

#### Image Behavior
Orbit has no photography. Attachments render as a file tile (icon + name + size).

## Elevation & Depth

| Level | Treatment | Use | Source |
|---|---|---|---|
| 0 Flat | `{colors.border}` hairline, no shadow | Board columns, rows, stats strip | P:206 |
| 1 Rest | `{shadow.1}`: `0 1px 2px rgba(45,32,18,.06), 0 1px 1px rgba(45,32,18,.03)` (dark: `0 1px 2px rgba(0,0,0,.3)`) | Widgets, panels, cards, buttons, fields | P:17, P:27 |
| 2 Float | `{shadow.2}`: `0 14px 36px rgba(45,32,18,.14), 0 3px 8px rgba(45,32,18,.07)` (dark: `0 16px 40px rgba(0,0,0,.5), 0 3px 8px rgba(0,0,0,.3)`) | Palette, popovers, toast, sheets, drawer | P:18, P:27 |
| Focus | `{shadow.focus}`: `0 0 0 2px var(--bg), 0 0 0 4px var(--accent)`; fields use a 3px `{colors.accent-soft}` ring | Keyboard focus, focused composer/field | P:19, P:106 |

Shadows are tinted warm brown, never neutral grey and never coloured glows. Most surfaces rely on the hairline. Level 2 is only for things that float above the page.

### Decorative Depth
Data components provide the depth: the roadmap canvas's dot grid, hatched locked phases and dashed gate lines; the heatmap's green steps; the pace bar's expected-today tick. An optional **Sketch style** applies an SVG turbulence displacement (`feTurbulence` 0.03, scale 3.2) to roadmap shapes for a hand-drawn look.

## Shapes

### Border Radius Scale

| Token | Value | Use | Source |
|---|---|---|---|
| `{rounded.xs}` | 4px | Heatmap cells (3), `<code>`, small chip buttons | P:293 |
| `{rounded.sm}` | 6px | Chips, keyboard hints, breadcrumbs, focus ring | P:98 |
| `{rounded.md}` | 7–8px | Nav rows (7), buttons, fields, icon buttons, inline add (8) | P:58, P:87 |
| `{rounded.md-plus}` | 9px | Segmented track, pace bars, task-row hover | P:95 |
| `{rounded.lg}` | 10px | Cards, callouts, radio cards, attachments, toast | P:211 |
| `{rounded.xl}` | 12px | Board columns, popovers, stats strip, gantt | P:206 |
| `{rounded.2xl}` | 14px | Widgets, panels, composer, roadmap shell, routine list | P:140 |
| `{rounded.3xl}` | 16px | Palette / quick-add dialog | P:314 |
| `{rounded.sheet}` | 20px top only (18 for detail sheet) | Phone bottom sheets | P:338 |
| `{rounded.full}` | 50% / 99px | Checkboxes, routine tick bubbles, switches, dots, Today pill | P:120 |

Off-scale radii to fold in: 2, 3, 5, 11, 15px (5 uses total).

### Photography Geometry
Not applicable. Avatars and cover images are **TBD** (not in v1 scope).

## Components

> These are prototype components (`P:` lines are CSS; the markup is generated in the script section). States documented: default, hover, selected/active, focus, and done where it applies. Disabled is only at 50% opacity on Add buttons, and loading/skeleton states are **TBD**.

### Navigation

**`sidebar`**: app shell navigation (design A)
- `{colors.side}` background, 1px right hairline, 256px wide (P:47–48). Order from top: Orbit mark + wordmark; **Search** field-button (`Ctrl K` hint) + square accent **New task** button; primary nav (Home, Inbox, Upcoming, Routines with counts); **Areas** tree (chevron toggle, Area icon tile, children: Projects, Goals, Roadmap, Pages); **Views**; footer (Profile with Stardust, Settings).

**`nav-row`**: sidebar row
- 32px tall (30 for nested rows, indented 30px), `{rounded.md}` 7px, 9px gap, icon in `{colors.icon}`. Hover fills `{colors.hover}`. Current fills `{colors.press}`, weight 600, accent icon (P:58–63).

**`area-tile`**: Area identity mark
- 22px (36px in page headers) square, 6px (10px) radius, Area colour at 15% as the fill and full colour as the 14px icon stroke (P:66). The only place Area colour fills anything.

**`top-bar`**: breadcrumb bar
- 48px, breadcrumbs as hover-revealed buttons with chevron separators, right side "New task `Q`" ghost button (P:70–75). On phone it becomes menu / centred title / search.

**`tab-bar`** (phone): Home · Upcoming · **Add** · Routines · Browse. 72px with a blurred surface at 92% opacity. The centre Add is a 48px accent square with 15px radius and neutral shadow (P:360–366).

### Buttons

**`button`**: default
- `{colors.surface}`, 1px `{colors.border}`, 32px tall, 12px side padding, `{rounded.md}`, weight 500, `{shadow.1}`. Hover: `{colors.border-strong}` border (P:87–88).

**`button-primary`**: accent fill, `{colors.accent-ink}` text; hover brightens to 1.06. One per region.

**`button-ghost`**: no border or shadow, muted text, hover `{colors.hover}`. Used for widget footers and the top-bar action.

**`button-on`**: toggled state: `{colors.accent-soft}` fill, `{colors.accent-line}` border, accent text (Filter with active filters, Sketch style).

**`icon-button`**: 32px (26px small), `{rounded.md}`, icon colour, hover fill (P:93).

**`segmented`**: view switcher (List/Board/Timeline, Canvas/Graph, Stats/Settings)
- `{colors.sunk}` track, 3px padding, 9px radius. Segments are 28px; the selected one is white with `{shadow.1}` (P:95–97). This is the Notion database-view-tabs pattern.

### Cards & Containers

**`widget`**: dashboard card (design C)
- `{colors.surface}`, hairline, `{rounded.2xl}` 14px, `{shadow.1}` (P:140). Header: accent icon + 14.5/600 title + optional right meta. Body is padded 4/8. An optional footer sits under a hairline with a ghost link. **Customize mode:** a dashed `{colors.border-strong}` outline, grab cursor, and a toolbar with drag handle, move earlier/later, size S/M/L/XL and hide (P:148–151).

**`panel`**: content section on detail screens: same chrome as a widget, 16px padding, 16/600 heading with a muted count (P:248–250).

**`board-column`** / **`board-card`**: kanban
- Columns: `{colors.sunk}`, 12px radius, 8px padding, 272px wide; header has a colour dot, label, count and add button. Cards: white, hairline, 10px radius, `{shadow.1}`, check + title, meta chips indented under the title. While dragging, the card fades to 40% and the target column takes the accent-soft fill with an accent-line border (P:206–216). Keyboard: Alt+←/→ moves a card.

**`popover`**: filter / view options: white, 12px radius, `{shadow.2}`, 8px padding; muted 12.5/600 section labels, option pills, switch rows (P:196–202).

### Inputs & Forms

**`field`**: 34px, white, hairline, 8px radius, leading icon. Focus: accent-line border + 3px accent-soft ring (P:105–107).

**`composer`**: quick-add bar (signature)
- A widget-style shell (14px radius) with an accent `+`, a 15px borderless input and an **Add** primary button. Underneath, a live row of parsed-token chips (date, priority, #label, @project) that teaches the syntax when empty (P:153–159). Appears on Home, every collection and Upcoming. The same parser powers the quick-add dialog (desktop, `Q`) and bottom sheet (phone, centre tab).

**`switch`**: 34×20, `{rounded.full}`, `{colors.border-strong}` → accent when on (P:111–113).
**`radio-card`**: bordered option card; selected gets an accent-soft fill and a 5px accent ring dot (P:303–306).

### Signature Components

**`task-row`**: the core list item
- 42px min, round 18px checkbox with priority-coloured border and an 8% tint fill; title; optional **reason line** (icon + 12.5 muted, e.g. "Deadline today"); right-aligned meta chips (due, labels, subtasks, Area chip). Inset hairline dividers vanish on hover. Done: green check fill, title struck through and muted (P:116–131).

**`chip`**: 22px, 6px radius, 12.5px, sunk fill. Variants: `danger` (overdue), `acc` (today, active filter), `warn`, `ok`, `area` (hairline + Area dot) (P:98–104).

**`quick-tick`**: routine one-tap log
- A pill card (11px radius, 52px min) with a dashed 26px bubble holding the routine icon. Done: ok-soft fill and a solid ok bubble with a check. Skipped: sunk fill with a minus. Second line shows the streak (flame) or due time (P:162–170). What a tap does is set in Routines → Settings: mark done / cycle done→skipped→clear / open a quick log.

**`day-strip`**: 7-day selector inside Upcoming: day name + date + load dots (accent when ≥3). The selected day is accent-soft with an accent-line border (P:173–178).

**`pace-bar`**: goal progress: an 8px track (12px on the goal page) filled in the Area colour, with a 2px ink tick at the *expected-by-today* position (P:183–185). Replaces progress rings.

**`roadmap-canvas`**: time-gated roadmap (signature)
- An SVG on a dot-grid canvas: month axis; phase bands (done / active / **locked = hatched**); a phase pill per band; **gate lines** (dashed accent) with a "Gate 16 Dec" pill; step nodes (34px, 9px radius) styled by status (done ok-soft, in progress accent-soft, up next white, locked sunk + dashed); milestone diamonds in warn; dependency arrows as curves (dashed into locked steps); an accent Today line with a pill. You can pan, zoom (50–200%), jump to today, and turn on Sketch style. Selecting a node opens the **detail panel** (dates, phase, gate, needs/unlocks links, goal, mark done).

**`roadmap-graph`**: Mermaid-style left-to-right dependency graph: phases as rounded clusters (locked ones dashed), steps as nodes with the same status styling, curved edges. Shares the detail panel.

**`properties`**: Notion-style property list: 140px muted key column with icons, value column with chips and links (P:245–247). Used on Goals.

**`milestone-track`**: horizontal line with diamond markers (filled when done), labels and dates, an Area-coloured fill up to today and an accent today tick (P:254–261).

**`heatmap`**: 26 weeks × 7 days of 13px cells, 3px gap, 3px radius. Missed = sunk, skipped = hatched, done = 3 ok steps; month labels; legend (P:292–297).

**`stat-strip`**: four joined cells with hairline dividers, 22/700 value and muted label (P:289–291). Two columns on phone. It sits above the heatmap, never as a standalone hero.

**`command-palette`**: search + commands (signature)
- 640px dialog, 16px radius, `{shadow.2}`. 54px input; **scope tabs** (All/Tasks/Goals/Pages/Projects/Routines/Actions; Tab cycles); **filter tokens** (`p1`, `#label`, `due:week`, `in:area`, `is:done`) shown as accent chips with **Save as view**; grouped results with fuzzy-match highlighting in accent; `>` switches to actions; when empty it shows a **syntax grid** that teaches the tokens, and with no results it offers "Create task …" (P:314–336).

**`inline-link`** / **`callout`** / **`backlinks`**: Notion-pattern page blocks. Links are accent text with an accent-line underline and a leading type icon; the callout is a warn-soft block with an icon; backlinks are a hairline-topped list (P:269–277).

**`toast`**: ink background with paper text, 10px radius, `{shadow.2}`, optional accent-line **Undo** (P:340–341).

### Notion component patterns reused
Sidebar tree with toggles · breadcrumb top bar · database view tabs (List/Board/Timeline) · filter and view-options popovers with property toggles · properties list · callout · inline mentions/links · backlinks. **Not yet built (TBD):** slash-command menu, toggle blocks, the `[[` / `@` inline picker (it's covered by [Calendar, drag-and-drop and mentions spike](https://github.com/moneytosms/orbit/issues/27)).

### Examples (illustrative)

**`ex-app-shell-row`**: `nav-row`; the active indicator is a `{colors.press}` fill plus an accent icon (no side bar).
- Properties: `backgroundColor`, `activeIndicator`, `rounded` (7px), `padding` (0 10px)

**`ex-data-table-cell`**: gantt label/track cells: 40px rows, hairline row border, body type.
- Properties: `headerBackground` ({colors.surface}), `bodyTypography` ({typography.body}), `cellPadding` (0 12px), `rowBorder` ({colors.border})

**`ex-modal-card`**: `command-palette` chrome.
- Properties: `backgroundColor` ({colors.surface}), `rounded` (16px), `shadow` ({shadow.2})

**`ex-empty-state-card`**: centred icon + one muted sentence that teaches the next step, with an optional ghost action.
- Properties: `padding` (24px 12px), `captionTypography` ({typography.body} muted)

**`ex-toast`**: see `toast`.
- Properties: `backgroundColor` ({colors.text}), `rounded` (10px), `padding` (9px 10px 9px 14px)

**`ex-auth-form-card`**, **`ex-pricing-tier`**, **`ex-cart-drawer`**, **`ex-product-selector`**: **TBD / not applicable** (Orbit has no pricing or commerce; login screen not designed).

## Do's and Don'ts

### Do
- Keep `{colors.accent}` for the primary action, current selection, focus and Today. One primary button per region.
- Put pages on `{colors.bg}` and actionable things on `{colors.surface}`.
- Show Area identity only through `area-tile`, Area dots and timeline/pace bars.
- Write sentence-case labels and let weight 600 carry structure.
- Separate groups with space and a label; separate rows with inset hairlines.
- Use Lucide icons at 16px with 1.8 stroke, in `{colors.icon}` at rest.
- Give every list an inline add and every empty state a sentence that teaches the next step.
- Show progress against expectation (pace tick, gate dates), not a bare percentage.

### Don't
- Don't fill a button, band or background with an Area colour, or add a second structural accent.
- Don't use uppercase eyebrow labels, gradient text, glassmorphism as decoration, coloured glow shadows, or side-stripe borders on cards.
- Don't nest cards or put a widget inside a widget.
- Don't use emoji or Unicode glyphs as icons.
- Don't set text in `{colors.icon}`. It fails 4.5:1.
- Don't use progress rings or big standalone hero numbers. Stats sit in a `stat-strip` next to the data they summarise.
- Don't introduce new spacing or radius values. Snap to the scale and retire the off-scale ones listed above.
