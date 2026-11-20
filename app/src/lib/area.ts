import type { Row, Task } from './sync'

/** The ids of everything that lives under `areaId`: its Projects, Sections, Goals and Tasks, however deep. */
export function underArea(rows: Row[], areaId: string): Set<string> {
  const out = new Set<string>([areaId])
  let grew = true
  while (grew) {
    grew = false
    for (const r of rows) {
      if (!out.has(r.id) && 'parent_id' in r && out.has(r.parent_id)) {
        out.add(r.id)
        grew = true
      }
    }
  }
  return out
}

export type AreaStats = { open: number; completedThisWeek: number; overdue: number; activeGoals: number }

const WEEK_MS = 7 * 24 * 3600 * 1000

/** Counts for an Area page; `today` is YYYY-MM-DD, `now` is epoch ms. Hidden (trashed) rows don't count. */
export function areaStats(rows: Row[], areaId: string, today: string, now: number): AreaStats {
  const under = underArea(rows, areaId)
  const s: AreaStats = { open: 0, completedThisWeek: 0, overdue: 0, activeGoals: 0 }
  for (const r of rows) {
    if (!under.has(r.id) || ('trashed_at' in r && r.trashed_at != null)) continue
    if (r.kind === 'task') {
      const t = r as Task
      if (t.completed_at === null) {
        s.open++
        if (t.due !== null && t.due.date < today) s.overdue++
      } else if (t.completed_at > now - WEEK_MS) s.completedThisWeek++
    } else if (r.kind === 'goal' && r.completed_at === null && r.archived_at == null) s.activeGoals++
  }
  return s
}

/** The first `n` What's next items that sit under the Area (or are a Task/Milestone of its Goals). */
export function upNextIn<T extends { id: string }>(items: T[], rows: Row[], areaId: string, n = 5): T[] {
  const under = underArea(rows, areaId)
  const goalIds = new Set(rows.filter((r) => r.kind === 'goal' && under.has(r.id)).map((r) => r.id))
  const owned = new Set(rows.filter((r) => r.kind === 'milestone' && goalIds.has(r.goal_id)).map((r) => r.id))
  return items.filter((i) => under.has(i.id) || owned.has(i.id)).slice(0, n)
}

/** Open Tasks per Project of the Area. */
export function openCounts(rows: Row[], projectIds: string[]): Map<string, number> {
  const out = new Map(projectIds.map((id) => [id, 0]))
  for (const p of projectIds) {
    const under = underArea(rows, p)
    let n = 0
    for (const r of rows)
      if (r.kind === 'task' && r.completed_at === null && r.trashed_at == null && under.has(r.id)) n++
    out.set(p, n)
  }
  return out
}

/** Roadmaps owned by the Area, or by one of its Projects or Goals. */
export function roadmapsIn<T extends { parent_id: string }>(rows: Row[], roadmaps: T[], areaId: string): T[] {
  const under = underArea(rows, areaId)
  return roadmaps.filter((r) => under.has(r.parent_id))
}
