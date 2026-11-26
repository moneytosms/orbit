// Pure data helpers for the Home, Area and Project tiles. `rows` come from `visibleSnapshot`; `today` is YYYY-MM-DD.
import { underArea } from './area'
import type { Row, Task } from './sync'

const WEEK_MS = 7 * 24 * 3600 * 1000

const tasksOf = (rows: Row[], parentId?: string): Task[] => {
  const under = parentId ? underArea(rows, parentId) : null
  return rows.filter((r): r is Task => r.kind === 'task' && r.trashed_at == null && (under === null || under.has(r.id)))
}
const byDue = (a: Task, b: Task) =>
  (a.due?.date ?? '').localeCompare(b.due?.date ?? '') || a.order.localeCompare(b.order)

/** Open Tasks due `today`, under `parentId` (any depth) or everywhere. */
export const dueToday = (rows: Row[], today: string, parentId?: string): Task[] =>
  tasksOf(rows, parentId)
    .filter((t) => t.completed_at === null && t.due?.date === today)
    .sort(byDue)

/** Open Tasks due before `today`, oldest first. */
export const overdueTasks = (rows: Row[], today: string, parentId?: string): Task[] =>
  tasksOf(rows, parentId)
    .filter((t) => t.completed_at === null && t.due != null && t.due.date < today)
    .sort(byDue)

/** The last `n` completed Tasks, newest first. */
export const recentlyDone = (rows: Row[], n = 8, parentId?: string): Task[] =>
  tasksOf(rows, parentId)
    .filter((t) => t.completed_at !== null)
    .sort((a, b) => (b.completed_at ?? 0) - (a.completed_at ?? 0))
    .slice(0, n)

export type Progress = { done: number; total: number }

/**
 * Done in the last 7 days against everything that was on the plate: those plus the open Tasks due today or earlier.
 * `now` is epoch ms.
 */
export function weekProgress(rows: Row[], today: string, now: number, parentId?: string): Progress {
  let done = 0
  let left = 0
  for (const t of tasksOf(rows, parentId)) {
    if (t.completed_at !== null) {
      if (t.completed_at > now - WEEK_MS) done++
    } else if (t.due != null && t.due.date <= today) left++
  }
  return { done, total: done + left }
}

/** Share of `total` finished, 0..1; nothing planned counts as 0. */
export const share = (p: Progress) => (p.total === 0 ? 0 : Math.min(1, p.done / p.total))

/** The `limit` Labels with the most open Tasks, ties by name; unused Labels are left out. */
export const topLabels = <T extends { label: { name: string }; tasks: number }>(usage: T[], limit = 12): T[] =>
  usage
    .filter((u) => u.tasks > 0)
    .sort((a, b) => b.tasks - a.tasks || a.label.name.localeCompare(b.label.name))
    .slice(0, limit)
