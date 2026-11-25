// Pure date logic for overdue Tasks and postponing: no storage, no wasm, `today` and every date is `YYYY-MM-DD`.
import type { Due, Task } from './sync'

/** A projection of a recurring Task's later due dates (wasm `recur_occurrences`), at most `n` of them. */
export type Project = (t: Task, n: number) => Due[]

/** The quick postpone choices. `none` clears the date. */
export type Postpone = 'tomorrow' | 'nextWeek' | 'weekend' | 'plus1d' | 'plus1w' | 'none'

export const POSTPONE: [Postpone, string][] = [
  ['tomorrow', 'Tomorrow'],
  ['nextWeek', 'Next week'],
  ['weekend', 'This weekend'],
  ['plus1d', '+1 day'],
  ['plus1w', '+1 week'],
  ['none', 'No date'],
]

/** How far ahead a recurring Task's series is scanned for its first occurrence on or after a target. */
export const SCAN = 400

const DAY = 86_400_000
const toMs = (date: string) => {
  const [y, m, d] = date.split('-').map(Number)
  return Date.UTC(y, m - 1, d)
}
const fromMs = (ms: number) => new Date(ms).toISOString().slice(0, 10)

/** `date` shifted by `n` days (negative allowed). */
export const addDays = (date: string, n: number): string => fromMs(toMs(date) + n * DAY)
/** Day of the week of `date`, 0 Sunday to 6 Saturday (the Settings numbering). */
export const weekday = (date: string): number => new Date(toMs(date)).getUTCDay()

/** The open, untrashed Tasks with a due date before `today`, earliest first (ties by title). */
export function overdueTasks(tasks: Task[], today: string): Task[] {
  return tasks
    .filter((t) => t.completed_at === null && t.trashed_at === null && t.due !== null && t.due.date < today)
    .sort((a, b) => a.due!.date.localeCompare(b.due!.date) || a.title.localeCompare(b.title))
}

/** "1 overdue" or "3 overdue". */
export const overdueLabel = (n: number) => `${n} overdue`

/**
 * The day a quick choice lands on. `from` is where `plus1d` and `plus1w` count from (the later of the due date and
 * `today`, so an overdue Task is pushed out from today). `weekStart` is 0 (Sunday) or 1 (Monday); the next week starts
 * on that day after `today`. "This weekend" is today on a Saturday or Sunday, else the coming Saturday.
 * `none` has no date and returns null.
 */
export function targetDate(
  choice: Postpone | 'today',
  today: string,
  opts: { from?: string; weekStart?: number } = {},
): string | null {
  const from = opts.from && opts.from > today ? opts.from : today
  switch (choice) {
    case 'today':
      return today
    case 'tomorrow':
      return addDays(today, 1)
    case 'nextWeek': {
      const start = opts.weekStart ?? 1
      for (let k = 1; k <= 7; k++) if (weekday(addDays(today, k)) === start) return addDays(today, k)
      return addDays(today, 7)
    }
    case 'weekend': {
      const wd = weekday(today)
      return wd === 0 || wd === 6 ? today : addDays(today, 6 - wd)
    }
    case 'plus1d':
      return addDays(from, 1)
    case 'plus1w':
      return addDays(from, 7)
    case 'none':
      return null
  }
}

/**
 * The due a Task moves to for `target` (null clears it). A plain Task keeps its time of day and takes the target date.
 * A recurring Task moves to its first occurrence strictly after its current due that falls on or after `target`, so a
 * series keeps its rhythm; if the projection finds none, it takes the plain move.
 */
export function newDue(task: Task, target: string | null, project?: Project): Due | null {
  if (target === null) return null
  const plain: Due = task.due?.time ? { date: target, time: task.due.time } : { date: target }
  if (!task.recurrence || !task.due || !project) return plain
  const next = project(task, SCAN).find((d) => d.date >= target)
  return next ? { date: next.date, ...(next.time ? { time: next.time } : {}) } : plain
}

export type Move = { id: string; before: Due | null; after: Due | null }

/** One Move per Task whose due would change, for moving every Task in `tasks` to `target`. */
export function planMoves(tasks: Task[], target: string | null, project?: Project): Move[] {
  const out: Move[] = []
  for (const t of tasks) {
    const after = newDue(t, target, project)
    if (sameDue(t.due, after)) continue
    out.push({ id: t.id, before: t.due, after })
  }
  return out
}

const sameDue = (a: Due | null, b: Due | null) =>
  a === b || (a !== null && b !== null && a.date === b.date && (a.time ?? null) === (b.time ?? null))
