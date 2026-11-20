import type { PhaseSummary } from './sync'

const DAY = 86_400_000
const UNDATED_DAYS = 7

export const dayNumber = (date: string): number => Math.round(Date.parse(`${date}T00:00:00Z`) / DAY)
export const dayDate = (n: number): string => new Date(n * DAY).toISOString().slice(0, 10)

export type TimelineItem = {
  id: string
  title: string
  shape: 'bar' | 'diamond' | 'tick'
  /** Left edge and width as a percentage of the whole timeline. */
  x: number
  w: number
  lane: number
  outside: boolean
  progress: number
}
export type TimelineBand = {
  id: string
  name: string
  x: number
  w: number
  /** An undated Phase, drawn 7 days wide with a dashed edge. */
  dashed: boolean
  locked: boolean
  completed: boolean
  current: boolean
  lanes: number
  items: TimelineItem[]
}
export type Timeline = { bands: TimelineBand[]; todayX: number | null; first: string; last: string }

/** Lays Phases out as bands on a shared date axis; undated Phases follow the last dated one. */
export function layoutTimeline(phases: PhaseSummary[], today: string): Timeline {
  const range = (p: PhaseSummary): [number, number] | null => {
    const s = p.start ?? p.end
    const e = p.end ?? p.start
    if (!s || !e) return null
    const a = dayNumber(s)
    const b = dayNumber(e)
    return a <= b ? [a, b] : [b, a]
  }
  const dated = phases.map(range)
  const todayN = dayNumber(today)
  const known = dated.flatMap((r) => (r ? r : []))
  let cursor = known.length ? Math.max(...known) + 1 : todayN
  const spans = phases.map((_, i) => {
    const r = dated[i]
    if (r) return { from: r[0], to: r[1] + 1, dashed: false }
    const from = cursor
    cursor += UNDATED_DAYS
    return { from, to: cursor, dashed: true }
  })
  const itemDates = phases.flatMap((p) =>
    p.items.flatMap((i) => [i.date, i.start, i.end].flatMap((d) => (d ? [dayNumber(d)] : []))),
  )
  const lo = Math.min(...spans.map((s) => s.from), ...itemDates, todayN)
  const hi = Math.max(...spans.map((s) => s.to), ...itemDates.map((n) => n + 1), todayN + 1)
  const total = hi - lo
  const pct = (n: number) => ((n - lo) / total) * 100
  const bands = phases.map((p, i): TimelineBand => {
    const span = spans[i]
    const items = p.items.map((it, lane): TimelineItem => {
      const point = it.date ?? it.start ?? it.end
      const base = { id: it.id, title: it.title, lane, outside: it.outside ?? false, progress: it.progress }
      if (it.kind === 'milestone') {
        const n = point ? dayNumber(point) : span.from
        return { ...base, shape: 'diamond', x: pct(n), w: 0 }
      }
      if (it.start && it.end) {
        const a = dayNumber(it.start)
        const b = dayNumber(it.end)
        return { ...base, shape: 'bar', x: pct(Math.min(a, b)), w: pct(Math.max(a, b) + 1) - pct(Math.min(a, b)) }
      }
      const n = point ? dayNumber(point) : span.from
      return { ...base, shape: 'tick', x: pct(n), w: 0 }
    })
    return {
      id: p.id,
      name: p.name,
      x: pct(span.from),
      w: pct(span.to) - pct(span.from),
      dashed: span.dashed,
      locked: p.locked,
      completed: p.completed,
      current: p.current,
      lanes: Math.max(1, items.length),
      items,
    }
  })
  return { bands, todayX: pct(todayN), first: dayDate(lo), last: dayDate(hi - 1) }
}
