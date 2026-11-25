import type { Section, Task, Trigger } from './sync'

/** A row the copy needs to know about: Tasks, and Sections as siblings for ordering. */
export type PlanRow = Task | Section

/** One Task to create, parents before children. `patch` holds only the fields with a value to copy. */
export type CopyOp = {
  id: string
  parent_id: string
  title: string
  order: string
  patch: {
    description?: string
    due?: Task['due']
    deadline?: string
    duration_min?: number
    priority: Task['priority']
    label_ids?: string[]
    goal_id?: string
  }
  reminders: Trigger[]
}

/**
 * The commands that duplicate the Task `rootId` and its open subtasks.
 * The copy is open, with fresh ids and no completion history; it lands right after the original
 * among its siblings. Recurrence is not copied, and reminders that already fired are dropped.
 * Pure: ids and ordering come in as functions, so tests need no wasm.
 */
export function duplicatePlan(
  rows: PlanRow[],
  rootId: string,
  newId: () => string,
  orderBetween: (a?: string, b?: string) => string,
): CopyOp[] {
  const tasks = rows.filter((r): r is Task => r.kind === 'task')
  const root = tasks.find((t) => t.id === rootId)
  if (!root) return []
  const open = tasks.filter((t) => t.completed_at == null && t.trashed_at == null)
  const byOrder = (a: { order: string; id: string }, b: { order: string; id: string }) =>
    a.order < b.order ? -1 : a.order > b.order ? 1 : a.id < b.id ? -1 : 1

  // The next sibling after the original, so the copy sits between the two.
  const next = rows
    .filter((r) => r.id !== root.id && r.parent_id === root.parent_id && r.order > root.order)
    .sort(byOrder)[0]
  const rootOrder = orderBetween(root.order, next?.order)

  const ops: CopyOp[] = []
  const copy = (task: Task, parentId: string, order: string) => {
    const id = newId()
    const op: CopyOp = {
      id,
      parent_id: parentId,
      title: task.title,
      order,
      patch: { priority: task.priority },
      reminders: task.reminders
        .filter((r) => !r.spent)
        .map(({ id: _id, spent: _spent, ...trigger }) => trigger as Trigger),
    }
    if (task.description) op.patch.description = task.description
    if (task.due) op.patch.due = { ...task.due }
    if (task.deadline) op.patch.deadline = task.deadline
    if (task.duration_min != null) op.patch.duration_min = task.duration_min
    if (task.label_ids.length) op.patch.label_ids = [...task.label_ids]
    if (task.goal_id) op.patch.goal_id = task.goal_id
    ops.push(op)
    for (const child of open.filter((t) => t.parent_id === task.id).sort(byOrder)) {
      copy(child, id, child.order)
    }
    return id
  }
  copy(root, root.parent_id, rootOrder)
  return ops
}
