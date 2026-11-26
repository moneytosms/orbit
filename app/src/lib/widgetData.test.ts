import { expect, test } from 'vitest'
import { dueToday, overdueTasks, recentlyDone, share, topLabels, weekProgress } from './widgetData'
import type { Row } from './sync'

const DAY = 24 * 3600 * 1000
const NOW = Date.UTC(2026, 10, 26, 12)
const t = (id: string, parent: string, extra: Record<string, unknown> = {}) =>
  ({
    kind: 'task',
    id,
    parent_id: parent,
    title: id,
    completed_at: null,
    order: id,
    trashed_at: null,
    due: null,
    ...extra,
  }) as unknown as Row
const area = (id: string) => ({ kind: 'area', id, name: id }) as unknown as Row
const rows: Row[] = [
  area('A'),
  area('B'),
  t('today', 'A', { due: { date: '2026-11-26' } }),
  t('late', 'A', { due: { date: '2026-11-20' } }),
  t('later', 'A', { due: { date: '2026-12-01' } }),
  t('other', 'B', { due: { date: '2026-11-26' } }),
  t('gone', 'A', { due: { date: '2026-11-26' }, trashed_at: 1 }),
  t('d1', 'A', { completed_at: NOW - DAY }),
  t('d2', 'B', { completed_at: NOW - 2 * DAY }),
  t('old', 'A', { completed_at: NOW - 30 * DAY }),
]

test('due today and overdue, optionally scoped', () => {
  expect(dueToday(rows, '2026-11-26').map((x) => x.id)).toEqual(['other', 'today'])
  expect(dueToday(rows, '2026-11-26', 'A').map((x) => x.id)).toEqual(['today'])
  expect(overdueTasks(rows, '2026-11-26').map((x) => x.id)).toEqual(['late'])
  expect(overdueTasks(rows, '2026-11-26', 'B')).toEqual([])
})

test('recently done is newest first, capped and scoped', () => {
  expect(recentlyDone(rows).map((x) => x.id)).toEqual(['d1', 'd2', 'old'])
  expect(recentlyDone(rows, 1).map((x) => x.id)).toEqual(['d1'])
  expect(recentlyDone(rows, 8, 'B').map((x) => x.id)).toEqual(['d2'])
})

test('week progress counts the last 7 days against what is still due', () => {
  expect(weekProgress(rows, '2026-11-26', NOW)).toEqual({ done: 2, total: 5 })
  expect(weekProgress(rows, '2026-11-26', NOW, 'B')).toEqual({ done: 1, total: 2 })
  expect(share({ done: 0, total: 0 })).toBe(0)
  expect(share({ done: 2, total: 5 })).toBeCloseTo(0.4)
})

test('top labels drop unused ones and sort by count then name', () => {
  const u = [
    { label: { name: 'b' }, tasks: 2 },
    { label: { name: 'a' }, tasks: 2 },
    { label: { name: 'c' }, tasks: 5 },
    { label: { name: 'z' }, tasks: 0 },
  ]
  expect(topLabels(u).map((x) => x.label.name)).toEqual(['c', 'a', 'b'])
  expect(topLabels(u, 1)).toHaveLength(1)
})
