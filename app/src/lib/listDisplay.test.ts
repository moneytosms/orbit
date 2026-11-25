import { describe, expect, it } from 'vitest'
import { dateBucket, groupItems, isCustom, sortItems } from './listDisplay'
import type { Task } from './sync'

const t = (id: string, p: Partial<Task> = {}) => ({
  task: { id, title: id, priority: 4, due: null, label_ids: [], created_at: 0, ...p } as Task,
})
const ids = (xs: { task: Task }[]) => xs.map((x) => x.task.id)

describe('sortItems', () => {
  const items = [
    t('b', { priority: 2, due: { date: '2026-03-05', time: null }, created_at: 3 }),
    t('a', { priority: 1, created_at: 1 }),
    t('c', { priority: 2, due: { date: '2026-03-01', time: null }, created_at: 2 }),
  ]
  it('keeps manual order', () => expect(ids(sortItems(items, 'manual'))).toEqual(['b', 'a', 'c']))
  it('sorts by name, priority, date (undated last) and added', () => {
    expect(ids(sortItems(items, 'name'))).toEqual(['a', 'b', 'c'])
    expect(ids(sortItems(items, 'priority'))).toEqual(['a', 'b', 'c'])
    expect(ids(sortItems(items, 'date'))).toEqual(['c', 'b', 'a'])
    expect(ids(sortItems(items, 'added'))).toEqual(['a', 'c', 'b'])
  })
})

describe('groupItems', () => {
  const today = '2026-03-02'
  it('buckets by date relative to today', () => {
    expect(dateBucket('2026-03-01', today)[1]).toBe('Overdue')
    expect(dateBucket('2026-03-03', today)[1]).toBe('Tomorrow')
    expect(dateBucket('2026-03-09', today)[1]).toBe('Next 7 days')
    expect(dateBucket('2026-03-10', today)[1]).toBe('Later')
    expect(dateBucket('2026-12-31', '2026-12-31')[1]).toBe('Today')
  })
  it('groups by date in heading order and drops empty groups', () => {
    const g = groupItems(
      [
        t('n'),
        t('o', { due: { date: '2026-02-01', time: null } }),
        t('x', { due: { date: '2026-03-02', time: null } }),
      ],
      { group: 'date', sort: 'manual' },
      today,
    )
    expect(g.map((s) => s.title)).toEqual(['Overdue', 'Today', 'No date'])
  })
  it('groups by priority', () => {
    const g = groupItems([t('a', { priority: 3 }), t('b', { priority: 1 })], { group: 'priority', sort: 'name' }, today)
    expect(g.map((s) => s.title)).toEqual(['Priority 1', 'Priority 3'])
  })
  it('groups by label, repeating multi-label tasks and ending with No label', () => {
    const names = new Map([
      ['l1', 'Home'],
      ['l2', 'Work'],
    ])
    const g = groupItems(
      [t('a', { label_ids: ['l2', 'l1'] }), t('b'), t('c', { label_ids: ['l2'] })],
      { group: 'label', sort: 'manual' },
      today,
      names,
    )
    expect(g.map((s) => [s.title, ids(s.items)])).toEqual([
      ['Home', ['a']],
      ['Work', ['a', 'c']],
      ['No label', ['b']],
    ])
  })
  it('flags custom displays', () => {
    expect(isCustom({ group: 'none', sort: 'manual' })).toBe(false)
    expect(isCustom({ group: 'none', sort: 'name' })).toBe(true)
  })
})
