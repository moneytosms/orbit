import type { Task } from './sync'

/** How a list screen shows its Tasks: grouped under headings and sorted within each group. */
export type GroupBy = 'none' | 'date' | 'priority' | 'label'
export type SortBy = 'manual' | 'name' | 'date' | 'priority' | 'added'
export type Display = { group: GroupBy; sort: SortBy }

export const DEFAULT_DISPLAY: Display = { group: 'none', sort: 'manual' }
export const groupOptions: [GroupBy, string][] = [
  ['none', 'No grouping'],
  ['date', 'Date'],
  ['priority', 'Priority'],
  ['label', 'Label'],
]
export const sortOptions: [SortBy, string][] = [
  ['manual', 'Manual'],
  ['name', 'Name'],
  ['date', 'Date'],
  ['priority', 'Priority'],
  ['added', 'Date added'],
]

/** Anything carrying a Task (a list row with its subtasks). */
type Item = { task: Task }
export type Section<T> = { key: string; title: string; items: T[] }

/** Whether the display differs from the manual, ungrouped list (which keeps drag-and-drop). */
export const isCustom = (d: Display) => d.group !== 'none' || d.sort !== 'manual'

const key = (screen: string) => `orbit.display.${screen}`

/** The saved display for a screen; anything unreadable gives the default. */
export function loadDisplay(screen: string): Display {
  try {
    const v = JSON.parse(localStorage.getItem(key(screen)) ?? 'null')
    const group = groupOptions.some(([g]) => g === v?.group) ? v.group : 'none'
    const sort = sortOptions.some(([s]) => s === v?.sort) ? v.sort : 'manual'
    return { group, sort }
  } catch {
    return DEFAULT_DISPLAY
  }
}

export function saveDisplay(screen: string, d: Display) {
  try {
    localStorage.setItem(key(screen), JSON.stringify(d))
  } catch {
    // Storage blocked: the choice lasts for this visit.
  }
}

const NONE = '￿'
const cmp = (a: string | number, b: string | number) => (a < b ? -1 : a > b ? 1 : 0)

/** Sorted copy. Dateless Tasks and ties fall back to the manual order; `manual` keeps the input order. */
export function sortItems<T extends Item>(items: T[], sort: SortBy): T[] {
  if (sort === 'manual') return items
  const val = (t: Task): string | number => {
    if (sort === 'name') return t.title.toLowerCase()
    if (sort === 'date') return t.due ? `${t.due.date} ${t.due.time ?? '00:00'}` : NONE
    if (sort === 'priority') return t.priority
    return t.created_at ?? 0
  }
  return items
    .map((it, i) => ({ it, i }))
    .sort((a, b) => cmp(val(a.it.task), val(b.it.task)) || a.i - b.i)
    .map((x) => x.it)
}

const addDays = (date: string, n: number) => {
  const [y, m, d] = date.split('-').map(Number)
  const x = new Date(Date.UTC(y, m - 1, d + n))
  return x.toISOString().slice(0, 10)
}

/** The date heading a due date falls under, relative to `today` (`YYYY-MM-DD`). */
export function dateBucket(date: string | undefined, today: string): [string, string] {
  if (!date) return ['9-none', 'No date']
  if (date < today) return ['0-overdue', 'Overdue']
  if (date === today) return ['1-today', 'Today']
  if (date === addDays(today, 1)) return ['2-tomorrow', 'Tomorrow']
  if (date <= addDays(today, 7)) return ['3-week', 'Next 7 days']
  return ['4-later', 'Later']
}

/**
 * Items grouped under headings, each sorted by `display.sort`. Empty groups are dropped. A Task with several
 * labels appears under each; one with none goes under "No label". `labelNames` maps label ids to names.
 */
export function groupItems<T extends Item>(
  items: T[],
  display: Display,
  today: string,
  labelNames: Map<string, string> = new Map(),
): Section<T>[] {
  const sorted = sortItems(items, display.sort)
  if (display.group === 'none') return [{ key: 'all', title: '', items: sorted }]
  const map = new Map<string, Section<T>>()
  const put = (k: string, title: string, it: T) => {
    const s = map.get(k) ?? { key: k, title, items: [] }
    s.items.push(it)
    map.set(k, s)
  }
  for (const it of sorted) {
    const t = it.task
    if (display.group === 'date') {
      const [k, title] = dateBucket(t.due?.date, today)
      put(k, title, it)
    } else if (display.group === 'priority') {
      put(String(t.priority), `Priority ${t.priority}`, it)
    } else {
      const names = t.label_ids.flatMap((id) => (labelNames.has(id) ? [labelNames.get(id) as string] : []))
      if (!names.length) put(NONE, 'No label', it)
      for (const n of names) put(n.toLowerCase(), n, it)
    }
  }
  return [...map.values()].sort((a, b) => cmp(a.key, b.key))
}
