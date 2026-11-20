import type { CheckState, Goal, Milestone, Routine, Task } from './sync'

/** Minutes a timed Task takes on the grid when it has no `duration_min`. */
export const DEFAULT_MIN = 30

export type CalEvent = {
  id: string
  title: string
  start: string
  end?: string
  allDay: boolean
  editable: boolean
  durationEditable?: boolean
  classNames: string[]
  extendedProps: { kind: 'task' | 'deadline' | 'routine' | 'goal' | 'milestone'; id: string }
}
export type RoutineDays = {
  routine: Pick<Routine, 'id' | 'name' | 'time'>
  days: { date: string; state: CheckState | 'due' }[]
}

const pad = (n: number) => String(n).padStart(2, '0')
/** `YYYY-MM-DDTHH:MM` plus `min` minutes (naive local clock arithmetic). */
export function addMinutes(at: string, min: number): string {
  const d = new Date(`${at}:00Z`)
  d.setUTCMinutes(d.getUTCMinutes() + min)
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}T${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`
}
/** Minutes between two `YYYY-MM-DDTHH:MM` strings. */
export const minutesBetween = (a: string, b: string) =>
  Math.round((Date.parse(`${b}:00Z`) - Date.parse(`${a}:00Z`)) / 60_000)

/** FullCalendar events for the visible rows: timed Tasks sized by duration, everything dated-only as all-day markers. */
export function toEvents(
  tasks: Task[],
  routines: RoutineDays[],
  goals: Pick<Goal, 'id' | 'name' | 'target_date' | 'completed_at'>[],
  milestones: Pick<Milestone, 'id' | 'title' | 'date' | 'done'>[],
): CalEvent[] {
  const out: CalEvent[] = []
  for (const t of tasks) {
    if (t.completed_at !== null || t.trashed_at !== null) continue
    if (t.due) {
      if (t.due.time) {
        const start = `${t.due.date}T${t.due.time}`
        out.push({
          id: `task:${t.id}`,
          title: t.title,
          start,
          end: addMinutes(start, t.duration_min ?? DEFAULT_MIN),
          allDay: false,
          editable: true,
          durationEditable: true,
          classNames: ['cal-task'],
          extendedProps: { kind: 'task', id: t.id },
        })
      } else {
        out.push({
          id: `task:${t.id}`,
          title: t.title,
          start: t.due.date,
          allDay: true,
          editable: true,
          durationEditable: false,
          classNames: ['cal-task'],
          extendedProps: { kind: 'task', id: t.id },
        })
      }
    }
    if (t.deadline) {
      out.push({
        id: `deadline:${t.id}`,
        title: `Deadline: ${t.title}`,
        start: t.deadline,
        allDay: true,
        editable: false,
        classNames: ['cal-deadline'],
        extendedProps: { kind: 'deadline', id: t.id },
      })
    }
  }
  for (const { routine, days } of routines) {
    for (const d of days) {
      out.push({
        id: `routine:${routine.id}:${d.date}`,
        title: routine.name,
        start: d.date,
        allDay: true,
        editable: false,
        classNames: ['cal-routine', `cal-routine-${d.state}`],
        extendedProps: { kind: 'routine', id: routine.id },
      })
    }
  }
  for (const g of goals) {
    if (!g.target_date || g.completed_at !== null) continue
    out.push({
      id: `goal:${g.id}`,
      title: `Goal: ${g.name}`,
      start: g.target_date,
      allDay: true,
      editable: false,
      classNames: ['cal-goal'],
      extendedProps: { kind: 'goal', id: g.id },
    })
  }
  for (const m of milestones) {
    if (!m.date) continue
    out.push({
      id: `milestone:${m.id}`,
      title: m.title,
      start: m.date,
      allDay: true,
      editable: false,
      classNames: ['cal-milestone', ...(m.done ? ['cal-done'] : [])],
      extendedProps: { kind: 'milestone', id: m.id },
    })
  }
  return out
}
