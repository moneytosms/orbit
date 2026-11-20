import { expect, test } from 'vitest'
import { areaStats, openCounts, roadmapsIn, underArea, upNextIn } from './area'
import type { Row } from './sync'

const task = (id: string, parent_id: string, o: Record<string, unknown> = {}) =>
  ({ kind: 'task', id, parent_id, completed_at: null, due: null, trashed_at: null, ...o }) as unknown as Row
const rows: Row[] = [
  { kind: 'area', id: 'a', name: 'A' } as Row,
  { kind: 'area', id: 'b', name: 'B' } as Row,
  { kind: 'project', id: 'p', parent_id: 'a' } as Row,
  { kind: 'goal', id: 'g', parent_id: 'a', completed_at: null, archived_at: null, trashed_at: null } as Row,
  { kind: 'goal', id: 'g2', parent_id: 'a', completed_at: 5, archived_at: null, trashed_at: null } as Row,
  { kind: 'milestone', id: 'm', goal_id: 'g' } as Row,
  task('t1', 'a', { due: { date: '2026-01-01' } }),
  task('t2', 'p'),
  task('t3', 't2', { completed_at: 699_000_000 }),
  task('t4', 'p', { completed_at: 1 }),
  task('t5', 'b'),
  task('t6', 'p', { trashed_at: 3 }),
]

test('underArea walks parents', () => {
  expect([...underArea(rows, 'a')].sort()).toEqual(['a', 'g', 'g2', 'p', 't1', 't2', 't3', 't4', 't6'])
})

test('areaStats counts open, completed this week, overdue and active goals', () => {
  expect(areaStats(rows, 'a', '2026-02-01', 700_000_000)).toEqual({
    open: 2,
    completedThisWeek: 1,
    overdue: 1,
    activeGoals: 1,
  })
})

test('upNextIn keeps the first five under the area, incl. goal milestones', () => {
  const items = [{ id: 't5' }, { id: 't1' }, { id: 'm' }, { id: 't2' }]
  expect(upNextIn(items, rows, 'a').map((i) => i.id)).toEqual(['t1', 'm', 't2'])
  expect(upNextIn(items, rows, 'a', 1).map((i) => i.id)).toEqual(['t1'])
})

test('openCounts and roadmapsIn', () => {
  expect(openCounts(rows, ['p']).get('p')).toBe(1)
  const maps = [{ parent_id: 'p' }, { parent_id: 'g' }, { parent_id: 'b' }]
  expect(roadmapsIn(rows, maps, 'a')).toHaveLength(2)
})
