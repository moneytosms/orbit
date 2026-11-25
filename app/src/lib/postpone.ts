// Writes for overdue handling and quick postpone: every change goes through `updateTask` and can be taken back.
import { getTask, getWeekStart, updateTask, type Task } from './sync'
import { planMoves, POSTPONE, targetDate, type Move, type Postpone, type Project } from './overdue'
import { pushRestore } from './undo'
import { recur_occurrences } from '../../wasm/orbit_wasm'

const pad = (n: number) => String(n).padStart(2, '0')
/** The device's local date, `YYYY-MM-DD`. */
export const localToday = () => {
  const d = new Date()
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

/** A Task's later occurrences from core's recurrence (wasm); none for a plain Task or an unreadable rule. */
export const projectOccurrences: Project = (t, n) => {
  if (!t.recurrence || !t.due) return []
  try {
    return JSON.parse(recur_occurrences(JSON.stringify(t.recurrence), JSON.stringify(t.due), n))
  } catch {
    return []
  }
}

/** Writes each move and offers one undo that puts every original due back. */
export async function applyMoves(moves: Move[], text: string) {
  if (!moves.length) return
  for (const m of moves) await updateTask(m.id, { due: m.after })
  pushRestore({
    text,
    restore: async () => {
      for (const m of moves) await updateTask(m.id, { due: m.before })
    },
  })
}

/** The label of a quick choice, for the undo message. */
export const postponeLabel = (choice: Postpone) => POSTPONE.find(([c]) => c === choice)?.[1] ?? ''

/** Postpones one Task by a quick choice (see `targetDate`), with undo. */
export async function postponeTask(task: Task, choice: Postpone) {
  const target = targetDate(choice, localToday(), { from: task.due?.date, weekStart: await getWeekStart() })
  const moves = planMoves([task], target, projectOccurrences)
  const when = choice === 'none' ? 'cleared' : `moved to ${postponeLabel(choice).toLowerCase()}`
  await applyMoves(moves, `"${task.title}" ${when}`)
}

/** Postpones the Task with this id (the palette's way in). */
export async function postponeById(id: string, choice: Postpone) {
  const task = await getTask(id)
  if (task) await postponeTask(task, choice)
}

/** Moves each Task in `tasks` to `target` (null clears), with one undo. Returns how many changed. */
export async function moveAll(tasks: Task[], target: string | null, when: string): Promise<number> {
  const moves = planMoves(tasks, target, projectOccurrences)
  await applyMoves(moves, `${moves.length} ${moves.length === 1 ? 'task' : 'tasks'} ${when}`)
  return moves.length
}
