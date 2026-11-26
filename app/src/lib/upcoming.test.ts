import { describe, expect, it } from 'vitest'
import {
  bucketByDay,
  buildStrip,
  hasPrevWeek,
  laterGroups,
  limitGroups,
  moveSelection,
  shiftWeek,
  stripStart,
} from './upcoming'
import type { Task } from './sync'

const today = '2026-11-26'
const task = (id: string, date: string | null, over: Partial<Task> = {}): Task =>
  ({
    kind: 'task',
    id,
    title: id,
    parent_id: 'p',
    completed_at: null,
    trashed_at: null,
    priority: 4,
    due: date ? { date } : null,
    ...over,
  }) as Task

describe('upcoming', () => {
  it('starts the strip on today with 7 consecutive days', () => {
    const s = buildStrip(today, today, [])
    expect(s.map((d) => d.date)).toEqual([
      '2026-11-26',
      '2026-11-27',
      '2026-11-28',
      '2026-11-29',
      '2026-11-30',
      '2026-12-01',
      '2026-12-02',
    ])
    expect(s[0]).toMatchObject({ today: true, weekday: 'Thu', day: 26 })
    expect(s.filter((d) => d.today)).toHaveLength(1)
  })

  it('buckets tasks to their day, sorted by time then priority', () => {
    const tasks = [
      task('late', '2026-11-27', { due: { date: '2026-11-27', time: '15:00' } }),
      task('early', '2026-11-27', { due: { date: '2026-11-27', time: '09:00' }, priority: 3 }),
      task('allday', '2026-11-27'),
      task('urgent', '2026-11-27', { due: { date: '2026-11-27', time: '09:00' }, priority: 1 }),
      task('other', '2026-11-28'),
    ]
    const b = bucketByDay(tasks, ['2026-11-27', '2026-11-28'])
    expect(b.get('2026-11-27')!.map((t) => t.id)).toEqual(['allday', 'urgent', 'early', 'late'])
    expect(b.get('2026-11-28')!.map((t) => t.id)).toEqual(['other'])
  })

  it('excludes overdue, done, trashed and undated tasks from the days', () => {
    const tasks = [
      task('over', '2026-11-20'),
      task('done', '2026-11-27', { completed_at: 1 }),
      task('gone', '2026-11-27', { trashed_at: 1 }),
      task('none', null),
      task('ok', '2026-11-27'),
    ]
    const s = buildStrip(today, today, tasks)
    expect(s.reduce((n, d) => n + d.count, 0)).toBe(1)
    expect(s[1].count).toBe(1)
  })

  it('navigates by week, never before today', () => {
    expect(shiftWeek(today, today, -1)).toBe(today)
    const next = shiftWeek(today, today, 1)
    expect(next).toBe('2026-12-03')
    expect(stripStart(today, next)).toBe('2026-12-03')
    expect(buildStrip(today, next, [])[0].today).toBe(false)
    expect(hasPrevWeek(today, next)).toBe(true)
    expect(hasPrevWeek(today, today)).toBe(false)
    expect(shiftWeek(today, next, -1)).toBe('2026-11-26')
  })

  it('arrow moves cross into the next week and stop at today', () => {
    expect(moveSelection(today, today, -1)).toBe(today)
    expect(stripStart(today, moveSelection(today, '2026-12-02', 1))).toBe('2026-12-03')
  })

  it('lists tasks after the 7 days under date groups, limited', () => {
    const tasks = [
      task('in', '2026-12-02'),
      task('b', '2026-12-10'),
      task('a', '2026-12-03'),
      task('c', '2026-12-10', { priority: 1 }),
    ]
    const g = laterGroups(tasks, today)
    expect(g.map((x) => x.date)).toEqual(['2026-12-03', '2026-12-10'])
    expect(g[1].tasks.map((t) => t.id)).toEqual(['c', 'b'])
    expect(limitGroups(g, 2).map((x) => x.tasks.length)).toEqual([1, 1])
  })
})
