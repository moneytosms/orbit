import { beforeEach, describe, expect, it } from 'vitest'
import { duplicatePlan } from './duplicate'
import type { Section, Task } from './sync'

const task = (id: string, p: Partial<Task> = {}) =>
  ({
    kind: 'task',
    id,
    parent_id: 'proj',
    title: id,
    completed_at: null,
    trashed_at: null,
    order: id,
    description: '',
    due: null,
    deadline: null,
    duration_min: null,
    priority: 4,
    label_ids: [],
    goal_id: null,
    reminders: [],
    ...p,
  }) as Task

// Orders as plain strings; `between` just reports what it was asked for.
const between = (a?: string, b?: string) => `${a ?? ''}|${b ?? ''}`
let n = 0
const newId = () => `new-${++n}`
beforeEach(() => {
  n = 0
})

describe('duplicatePlan', () => {
  it('copies every field, with fresh ids, placed right after the original', () => {
    const root = task('a', {
      title: 'Write report',
      description: 'Q3 numbers',
      due: { date: '2026-12-01', time: '09:00' },
      deadline: '2026-12-05',
      duration_min: 45,
      priority: 2,
      label_ids: ['l1', 'l2'],
      goal_id: 'g1',
      reminders: [{ kind: 'before', field: 'due', minutes: 30, id: 'r1', spent: false }],
    })
    const sibling = task('c', { order: 'c' })
    const [op, ...rest] = duplicatePlan([root, sibling], 'a', newId, between)
    expect(rest).toEqual([])
    expect(op).toEqual({
      id: 'new-1',
      parent_id: 'proj',
      title: 'Write report',
      order: 'a|c',
      patch: {
        description: 'Q3 numbers',
        due: { date: '2026-12-01', time: '09:00' },
        deadline: '2026-12-05',
        duration_min: 45,
        priority: 2,
        label_ids: ['l1', 'l2'],
        goal_id: 'g1',
      },
      reminders: [{ kind: 'before', field: 'due', minutes: 30 }],
    })
  })

  it('leaves unset fields out of the patch and places a last sibling at the end', () => {
    const [op] = duplicatePlan([task('a')], 'a', newId, between)
    expect(op.patch).toEqual({ priority: 4 })
    expect(op.order).toBe('a|')
    expect(op.reminders).toEqual([])
  })

  it('does not copy recurrence, fired reminders or the completion', () => {
    const root = task('a', {
      completed_at: 123,
      reminders: [
        { kind: 'at', at: '2026-01-01T09:00', id: 'old', spent: true },
        { kind: 'at', at: '2026-12-01T09:00', id: 'new', spent: false },
      ],
    })
    const [op] = duplicatePlan([root], 'a', newId, between)
    expect(op.reminders).toEqual([{ kind: 'at', at: '2026-12-01T09:00' }])
    expect(op).not.toHaveProperty('completed_at')
  })

  it('copies open subtasks under the new parent and skips completed or trashed ones', () => {
    const rows = [
      task('a', { title: 'Parent' }),
      task('b', { parent_id: 'a', order: 'b' }),
      task('c', { parent_id: 'a', order: 'c', completed_at: 5 }),
      task('d', { parent_id: 'a', order: 'd', trashed_at: 9 }),
      task('e', { parent_id: 'b', order: 'e', title: 'Grandchild' }),
    ]
    const ops = duplicatePlan(rows, 'a', newId, between)
    expect(ops.map((o) => [o.id, o.parent_id, o.title, o.order])).toEqual([
      ['new-1', 'proj', 'Parent', 'a|'],
      ['new-2', 'new-1', 'b', 'b'],
      ['new-3', 'new-2', 'Grandchild', 'e'],
    ])
  })

  it('ignores sections and completed siblings when choosing the position', () => {
    const section = { kind: 'section', id: 's', parent_id: 'proj', name: 'S', order: 'b', trashed_at: null } as Section
    const done = task('x', { order: 'b5', completed_at: 1 })
    const [op] = duplicatePlan([task('a'), section, done], 'a', newId, between)
    expect(op.order).toBe('a|b')
  })

  it('returns nothing for an unknown id', () => {
    expect(duplicatePlan([task('a')], 'zzz', newId, between)).toEqual([])
  })
})
