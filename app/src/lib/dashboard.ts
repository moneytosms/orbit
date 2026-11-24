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
]
export const titleOf = (kind: string) => KINDS.find((k) => k.kind === kind)?.title ?? 'View'

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
]

const hit = (a: Widget, b: Widget) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h

/** Keeps `fixed` where it is and pushes any widget it overlaps down, in reading order. Hidden ones don't collide. */
function settle(widgets: Widget[], fixed: Widget): Widget[] {
  const placed: Widget[] = [fixed]
  const out = new Map<string, Widget>([[fixed.id, fixed]])
  const rest = widgets.filter((w) => w.id !== fixed.id)
  for (const w of [...rest].sort((a, b) => a.y - b.y || a.x - b.x).filter((w) => !w.hidden)) {
    let next = w
    while (placed.some((p) => hit(p, next))) next = { ...next, y: next.y + 1 }
    placed.push(next)
    out.set(next.id, next)
  }
  return widgets.map((w) => out.get(w.id) ?? w)
}

const clamp = (n: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, n))

/** Moves a widget to cell (x, y), kept inside the 12 columns; whatever it lands on moves down. */
export function move(widgets: Widget[], id: string, x: number, y: number): Widget[] {
  const w = widgets.find((w) => w.id === id)
  if (!w) return widgets
  return settle(widgets, { ...w, x: clamp(Math.round(x), 0, COLS - w.w), y: Math.max(0, Math.round(y)) })
}

/** Resizes a widget to w × h cells (at least 1 × 1, within the columns); whatever it grows over moves down. */
export function resize(widgets: Widget[], id: string, w: number, h: number): Widget[] {
  const t = widgets.find((t) => t.id === id)
  if (!t) return widgets
  const nw = clamp(Math.round(w), 1, COLS - t.x)
  return settle(widgets, { ...t, w: nw, h: Math.max(1, Math.round(h)) })
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
