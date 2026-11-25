/** The fields the Completed screen reads from a Task. */
export type DoneTask = { id: string; title: string; description: string; completed_at: number | null }

/** Completed Tasks for one local day, newest first. */
export type DayGroup<T> = { day: string; label: string; items: T[] }

const pad = (n: number) => String(n).padStart(2, '0')

/** The local `YYYY-MM-DD` a timestamp (ms) falls on. */
export const dayKey = (ms: number) => {
  const d = new Date(ms)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

/** `Today`, `Yesterday`, or a short date such as `Mon 2 Mar 2026`. */
export function dayLabel(day: string, today: string): string {
  if (day === today) return 'Today'
  const at = Date.parse(`${day}T12:00:00Z`)
  if (day === dayKey(Date.parse(`${today}T12:00:00Z`) - 86_400_000)) return 'Yesterday'
  return new Date(at).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    timeZone: 'UTC',
  })
}

/** Whether every word of `query` appears in the title or description (case-insensitive, any order). */
export function matchesWords(task: Pick<DoneTask, 'title' | 'description'>, query: string): boolean {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean)
  if (!words.length) return true
  const text = `${task.title} ${task.description}`.toLowerCase()
  return words.every((w) => text.includes(w))
}

/**
 * Groups completed Tasks matching `query` by the day they were completed.
 * Days are newest first, and Tasks within a day are newest first too.
 */
export function groupCompleted<T extends DoneTask>(
  tasks: T[],
  query: string,
  today: string,
  dayOf: (ms: number) => string = dayKey,
): DayGroup<T>[] {
  const byDay = new Map<string, T[]>()
  for (const t of tasks) {
    if (t.completed_at == null || !matchesWords(t, query)) continue
    const day = dayOf(t.completed_at)
    byDay.set(day, [...(byDay.get(day) ?? []), t])
  }
  return [...byDay.entries()]
    .sort(([a], [b]) => (a < b ? 1 : a > b ? -1 : 0))
    .map(([day, items]) => ({
      day,
      label: dayLabel(day, today),
      items: items.sort((a, b) => (b.completed_at ?? 0) - (a.completed_at ?? 0) || (a.id < b.id ? -1 : 1)),
    }))
}
