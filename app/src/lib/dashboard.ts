import type { Row, Task } from './sync'

export const COLS = 12
export const ROW_PX = 96

export type Widget = {
  id: string
  kind: string
  view_id?: string | null
  x: number
  y: number
  w: number
  h: number
  hidden: boolean
}

export const KINDS: { kind: string; title: string; w: number; h: number }[] = [
  { kind: 'glance', title: 'Life at a glance', w: 12, h: 2 },
  { kind: 'next', title: "What's next", w: 6, h: 4 },
  { kind: 'routines', title: 'Routines', w: 6, h: 2 },
  { kind: 'upcoming', title: 'Upcoming', w: 6, h: 3 },
  { kind: 'goals', title: 'Goal progress', w: 6, h: 3 },
  { kind: 'streaks', title: 'Routine streaks', w: 6, h: 2 },
  { kind: 'calendar', title: 'Calendar', w: 6, h: 2 },
  { kind: 'today', title: 'Due today', w: 6, h: 3 },
  { kind: 'overdue', title: 'Overdue', w: 6, h: 3 },
  { kind: 'done', title: 'Recently done', w: 6, h: 3 },
  { kind: 'progress', title: 'Progress', w: 4, h: 2 },
  { kind: 'quickadd', title: 'Quick add', w: 6, h: 1 },
  { kind: 'labels', title: 'Labels', w: 6, h: 2 },
]
export const titleOf = (kind: string) => KINDS.find((k) => k.kind === kind)?.title ?? 'View'

export type PageKind = 'area' | 'project'

/** Tiles an Area or Project page can show, with the pages that offer each. */
export const PAGE_KINDS: { kind: string; title: string; w: number; h: number; on: PageKind[] }[] = [
  { kind: 'stats', title: 'Overview', w: 12, h: 1, on: ['area', 'project'] },
  { kind: 'tasks', title: 'Tasks', w: 6, h: 4, on: ['area', 'project'] },
  { kind: 'next', title: 'Next up', w: 6, h: 2, on: ['area', 'project'] },
  { kind: 'goals', title: 'Goals', w: 6, h: 2, on: ['area'] },
  { kind: 'routines', title: 'Routines', w: 6, h: 2, on: ['area', 'project'] },
  { kind: 'pages', title: 'Pages', w: 6, h: 2, on: ['area', 'project'] },
  { kind: 'projects', title: 'Projects', w: 6, h: 2, on: ['area'] },
  { kind: 'roadmaps', title: 'Roadmaps', w: 6, h: 2, on: ['area', 'project'] },
  { kind: 'today', title: 'Due today', w: 6, h: 3, on: ['area', 'project'] },
  { kind: 'overdue', title: 'Overdue', w: 6, h: 3, on: ['area', 'project'] },
  { kind: 'done', title: 'Recently done', w: 6, h: 3, on: ['area', 'project'] },
  { kind: 'progress', title: 'Progress', w: 4, h: 2, on: ['area', 'project'] },
  { kind: 'quickadd', title: 'Quick add', w: 6, h: 1, on: ['area', 'project'] },
  { kind: 'labels', title: 'Labels', w: 6, h: 2, on: ['area', 'project'] },
]
export const pageKinds = (page: PageKind) => PAGE_KINDS.filter((k) => k.on.includes(page))
export const pageTitleOf = (kind: string) => PAGE_KINDS.find((k) => k.kind === kind)?.title ?? kind

const pageAt = (kind: string, x: number, y: number, h?: number): Widget => {
  const k = PAGE_KINDS.find((k) => k.kind === kind)!
  return { id: kind, kind, x, y, w: k.w, h: h ?? k.h, hidden: false }
}

/** The layout of an Area or Project page before the user changes anything. */
export const defaultPageLayout = (page: PageKind): Widget[] =>
  page === 'area'
    ? [
        pageAt('stats', 0, 0),
        pageAt('tasks', 0, 1),
        pageAt('next', 6, 1),
        pageAt('goals', 6, 3),
        pageAt('routines', 0, 5),
        pageAt('projects', 6, 5),
        pageAt('pages', 0, 7),
        pageAt('roadmaps', 6, 7),
      ]
    : [
        pageAt('stats', 0, 0),
        pageAt('tasks', 0, 1),
        pageAt('next', 6, 1),
        pageAt('routines', 6, 3),
        pageAt('pages', 0, 5),
        pageAt('roadmaps', 6, 5),
      ]

const at = (kind: string, x: number, y: number): Widget => {
  const k = KINDS.find((k) => k.kind === kind)!
  return { id: kind, kind, x, y, w: k.w, h: k.h, hidden: false }
}

/** The layout of a first run, before the user changes anything. */
export const defaultLayout = (): Widget[] => [
  at('glance', 0, 0),
  at('next', 0, 2),
  at('routines', 6, 2),
  at('upcoming', 6, 4),
  at('goals', 0, 6),
  at('streaks', 6, 7),
  at('calendar', 0, 9),
]

/** Layouts saved before the Calendar widget existed get it appended at the bottom; the widget hides itself when empty. */
export const withCalendar = (widgets: Widget[]): Widget[] =>
  widgets.some((w) => w.kind === 'calendar') ? widgets : place(widgets, at('calendar', 0, 0))

const hit = (a: Widget, b: Widget) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h

/** Keeps `fixed` where it is, pushes any widget it overlaps down and lets the rest rise to close gaps, in reading order. Hidden ones don't collide. */
function settle(widgets: Widget[], fixed: Widget): Widget[] {
  const placed: Widget[] = [fixed]
  const out = new Map<string, Widget>([[fixed.id, fixed]])
  const rest = widgets.filter((w) => w.id !== fixed.id)
  for (const w of [...rest].sort((a, b) => a.y - b.y || a.x - b.x).filter((w) => !w.hidden)) {
    let next = w
    while (placed.some((p) => hit(p, next))) next = { ...next, y: next.y + 1 }
    // Gravity: a gap left by a tile that was dragged away closes up, so no empty band is left behind.
    while (next.y > 0 && !placed.some((p) => hit(p, { ...next, y: next.y - 1 }))) next = { ...next, y: next.y - 1 }
    placed.push(next)
    out.set(next.id, next)
  }
  return widgets.map((w) => out.get(w.id) ?? w)
}

/** Lets every visible tile rise until it rests on the top or on another tile, so a finished edit leaves no empty band. */
export function compact(widgets: Widget[]): Widget[] {
  const placed: Widget[] = []
  const out = new Map<string, Widget>()
  for (const w of widgets.filter((w) => !w.hidden).sort((a, b) => a.y - b.y || a.x - b.x)) {
    let next = w
    while (next.y > 0 && !placed.some((p) => hit(p, { ...next, y: next.y - 1 }))) next = { ...next, y: next.y - 1 }
    placed.push(next)
    out.set(next.id, next)
  }
  return widgets.map((w) => out.get(w.id) ?? w)
}

const clamp = (n: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, n))

/** Moves a widget to cell (x, y), kept inside the 12 columns; whatever it lands on moves down. */
export function move(widgets: Widget[], id: string, x: number, y: number, cols = COLS): Widget[] {
  const w = widgets.find((w) => w.id === id)
  if (!w) return widgets
  return settle(widgets, { ...w, x: clamp(Math.round(x), 0, Math.max(0, cols - w.w)), y: Math.max(0, Math.round(y)) })
}

/** Resizes a widget to w × h cells (at least 1 × 1, within the columns); whatever it grows over moves down. */
export function resize(widgets: Widget[], id: string, w: number, h: number, cols = COLS): Widget[] {
  const t = widgets.find((t) => t.id === id)
  if (!t) return widgets
  const nw = clamp(Math.round(w), 1, Math.max(1, cols - t.x))
  return settle(widgets, { ...t, w: nw, h: Math.max(1, Math.round(h)) })
}

/** Smallest a tile may be dragged to, in cells. */
export const MIN_W = 2
export const MIN_H = 1

/**
 * Puts widget `id` at `rect` (a move, a resize from any edge or corner, or both), clamped to the grid and to the
 * minimum size; whatever it lands on moves down. A tablet's narrower grid passes its own `cols`.
 */
export function setRect(widgets: Widget[], id: string, rect: Rect, cols = COLS): Widget[] {
  const t = widgets.find((o) => o.id === id)
  if (!t) return widgets
  const minW = Math.min(MIN_W, cols)
  const w = clamp(Math.round(rect.w), minW, cols)
  const h = Math.max(MIN_H, Math.round(rect.h))
  const x = clamp(Math.round(rect.x), 0, cols - w)
  const y = Math.max(0, Math.round(rect.y))
  return settle(widgets, { ...t, x, y, w, h })
}

/**
 * The rectangle a drag from `handle` (any of n, s, e, w, ne, nw, se, sw) gives a tile that started as `base`,
 * after the pointer moved `dx` cells across and `dy` cells down. The edge opposite the handle stays put.
 */
export function dragRect(base: Rect, handle: string, dx: number, dy: number, cols = COLS): Rect {
  const minW = Math.min(MIN_W, cols)
  let { x, y, w, h } = base
  if (handle.includes('e')) w = base.w + dx
  if (handle.includes('s')) h = base.h + dy
  if (handle.includes('w')) {
    x = clamp(base.x + dx, 0, base.x + base.w - minW)
    w = base.w + (base.x - x)
  }
  if (handle.includes('n')) {
    y = clamp(base.y + dy, 0, base.y + base.h - MIN_H)
    h = base.h + (base.y - y)
  }
  w = clamp(w, minW, cols - x)
  return { x, y, w, h: Math.max(MIN_H, h) }
}

/** Where a widget goes when added or shown again: the first free row at the bottom, full size. */
export function place(widgets: Widget[], widget: Widget): Widget[] {
  const bottom = Math.max(0, ...widgets.filter((w) => !w.hidden && w.id !== widget.id).map((w) => w.y + w.h))
  const w = { ...widget, x: 0, y: bottom, hidden: false }
  return widgets.some((o) => o.id === w.id) ? widgets.map((o) => (o.id === w.id ? w : o)) : [...widgets, w]
}

/** The visible widgets on a phone: one column, by y then x. */
export const phoneOrder = (widgets: Widget[]): Widget[] =>
  widgets.filter((w) => !w.hidden).sort((a, b) => a.y - b.y || a.x - b.x)

export type Rect = { x: number; y: number; w: number; h: number }

/** Whether `rect` is a legal spot for tile `id`: inside the `cols` columns and clear of every other visible tile. */
export const fits = (widgets: Widget[], id: string, rect: Rect, cols = COLS): boolean =>
  rect.x >= 0 &&
  rect.y >= 0 &&
  rect.w >= 1 &&
  rect.h >= 1 &&
  rect.x + rect.w <= cols &&
  !widgets.some((o) => o.id !== id && !o.hidden && hit({ ...o, ...rect }, o))

/** Columns for a viewport width: 12 on desktop, 6 on tablet (< 1024px), 2 on phone (< 640px). */
export const colsFor = (width: number) => (width < 640 ? 2 : width < 1024 ? 6 : COLS)

/** Narrowest a tile may get on a tablet (in tablet columns). */
export const TABLET_MIN_W = 2

/**
 * The visible tiles laid out for `cols` columns, from the one saved 12-column layout. Desktop is the layout as saved
 * (reading order); on a tablet x and w scale to the column count (w at least 2) and tiles that would collide move
 * down; on a phone every tile is full width and they stack in reading order.
 */
export function reflow(widgets: Widget[], cols: number): Widget[] {
  const visible = phoneOrder(widgets)
  if (cols >= COLS) return visible
  if (cols < 4) {
    let y = 0
    return visible.map((w) => {
      const out = { ...w, x: 0, w: cols, y }
      y += w.h
      return out
    })
  }
  const placed: Widget[] = []
  for (const w of visible) {
    const nw = clamp(Math.round((w.w * cols) / COLS), Math.min(TABLET_MIN_W, cols), cols)
    let next = { ...w, w: nw, x: clamp(Math.round((w.x * cols) / COLS), 0, cols - nw) }
    while (placed.some((p) => hit(p, next))) next = { ...next, y: next.y + 1 }
    placed.push(next)
  }
  return placed
}

/** Maps tiles edited in a `cols`-column view back onto the saved 12 columns, keeping hidden tiles and saved order. */
export function expand(edited: Widget[], cols: number, saved: Widget[]): Widget[] {
  if (cols >= COLS) return edited
  const f = COLS / cols
  const by = new Map(edited.map((w) => [w.id, { ...w, x: w.x * f, w: w.w * f }]))
  return saved.map((w) => by.get(w.id) ?? w)
}

/** Share of today's Tasks finished, 0..1; an empty day counts as complete. */
export const doneShare = (done: number, left: number) => (done + left === 0 ? 1 : done / (done + left))

export type Glance = { dueToday: number; overdue: number; activeGoals: number }

/** Open Tasks due today or overdue and active Goals; `today` is YYYY-MM-DD. */
export function glance(rows: Row[], today: string): Glance {
  const g: Glance = { dueToday: 0, overdue: 0, activeGoals: 0 }
  for (const r of rows) {
    if ('trashed_at' in r && r.trashed_at != null) continue
    if (r.kind === 'task') {
      const t = r as Task
      if (t.completed_at !== null || t.due === null) continue
      if (t.due.date === today) g.dueToday++
      else if (t.due.date < today) g.overdue++
    } else if (r.kind === 'goal' && r.completed_at === null && r.archived_at == null) g.activeGoals++
  }
  return g
}

/** Whether a View row belongs in Upcoming without duplicating What's next's current tasks. */
export const taskIsUpcoming = <T extends { kind: string; due?: { date: string } | null }>(row: T, today: string) =>
  row.kind !== 'task' || row.due == null || row.due.date > today
