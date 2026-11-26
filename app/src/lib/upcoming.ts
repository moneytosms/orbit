// Pure logic for the Upcoming screen: a 7-day strip starting from today and the open Tasks bucketed onto it.
// No storage, no wasm; `today` and every date is `YYYY-MM-DD`.
import { addDays, weekday } from './overdue'
import type { Task } from './sync'

export const STRIP_DAYS = 7
/** Later lists this many Tasks before the rest sit behind "Show more". */
export const LATER_LIMIT = 5

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

export type StripDay = { date: string; weekday: string; day: number; today: boolean; count: number }
export type Later = { date: string; tasks: Task[] }

const dayDiff = (a: string, b: string) => {
  const ms = (d: string) => {
    const [y, m, n] = d.split('-').map(Number)
    return Date.UTC(y, m - 1, n)
  }
  return Math.round((ms(a) - ms(b)) / 86_400_000)
}

/** The Tasks the screen shows: open, untrashed and dated. */
export const datedTasks = (tasks: Task[]): Task[] =>
  tasks.filter((t) => t.completed_at === null && t.trashed_at === null && t.due !== null)

/** First day of the strip that holds `selected`: today, then a week at a time. A date before today counts as today. */
export function stripStart(today: string, selected: string): string {
  const k = Math.max(0, Math.floor(dayDiff(selected, today) / STRIP_DAYS))
  return addDays(today, k * STRIP_DAYS)
}

/** The selection moved by `days` (negative allowed), never before today. */
export const moveSelection = (today: string, selected: string, days: number): string => {
  const next = addDays(selected, days)
  return next < today ? today : next
}

/** One week later or earlier; the earliest is today. */
export const shiftWeek = (today: string, selected: string, dir: 1 | -1): string =>
  moveSelection(today, selected, dir * STRIP_DAYS)

/** Whether there is a week before the one holding `selected`. */
export const hasPrevWeek = (today: string, selected: string) => stripStart(today, selected) > today

/** Tasks by earlier time first (all-day ahead of timed), then priority (1 first), then title. */
export function sortDay(tasks: Task[]): Task[] {
  return [...tasks].sort(
    (a, b) =>
      (a.due?.time ?? '').localeCompare(b.due?.time ?? '') || a.priority - b.priority || a.title.localeCompare(b.title),
  )
}

/** The Tasks due on each of `days`; anything else (overdue, later) is left out. Each day is sorted. */
export function bucketByDay(tasks: Task[], days: string[]): Map<string, Task[]> {
  const out = new Map<string, Task[]>(days.map((d) => [d, []]))
  for (const t of datedTasks(tasks)) out.get(t.due!.date)?.push(t)
  for (const [d, list] of out) out.set(d, sortDay(list))
  return out
}

/** The 7 days of the strip holding `selected`, with each day's Task count. */
export function buildStrip(today: string, selected: string, tasks: Task[]): StripDay[] {
  const start = stripStart(today, selected)
  const dates = Array.from({ length: STRIP_DAYS }, (_, i) => addDays(start, i))
  const buckets = bucketByDay(tasks, dates)
  return dates.map((date) => ({
    date,
    weekday: WEEKDAYS[weekday(date)],
    day: Number(date.slice(8)),
    today: date === today,
    count: buckets.get(date)!.length,
  }))
}

/** Tasks due after the first 7 days (from today), grouped by date, earliest first. */
export function laterGroups(tasks: Task[], today: string): Later[] {
  const edge = addDays(today, STRIP_DAYS - 1)
  const byDate = new Map<string, Task[]>()
  for (const t of datedTasks(tasks)) {
    if (t.due!.date <= edge) continue
    byDate.set(t.due!.date, [...(byDate.get(t.due!.date) ?? []), t])
  }
  return [...byDate.keys()].sort().map((date) => ({ date, tasks: sortDay(byDate.get(date)!) }))
}

/** `groups` cut to its first `limit` Tasks. */
export function limitGroups(groups: Later[], limit: number): Later[] {
  const out: Later[] = []
  let left = limit
  for (const g of groups) {
    if (left <= 0) break
    out.push({ date: g.date, tasks: g.tasks.slice(0, left) })
    left -= g.tasks.length
  }
  return out
}
