import { describe, expect, it } from 'vitest'
import { dayKey, dayLabel, groupCompleted, matchesWords } from './completed'

// Local-time stamps, so the day grouping is the same on every machine.
const at = (y: number, m: number, d: number, h = 12, min = 0) => new Date(y, m - 1, d, h, min).getTime()
const t = (id: string, completed_at: number | null, p: { title?: string; description?: string } = {}) => ({
  id,
  title: p.title ?? id,
  description: p.description ?? '',
  completed_at,
})

describe('dayKey and dayLabel', () => {
  it('reads the local day of a timestamp', () => {
    expect(dayKey(at(2026, 3, 2, 23, 30))).toBe('2026-03-02')
  })

  it('names today and yesterday, and dates anything else', () => {
    expect(dayLabel('2026-11-25', '2026-11-25')).toBe('Today')
    expect(dayLabel('2026-11-24', '2026-11-25')).toBe('Yesterday')
    expect(dayLabel('2026-11-20', '2026-11-25')).toMatch(/20 Nov 2026/)
  })
})

describe('matchesWords', () => {
  it('needs every word, in any order, in the title or description', () => {
    const task = { title: 'Pay the rent', description: 'bank transfer' }
    expect(matchesWords(task, 'rent pay')).toBe(true)
    expect(matchesWords(task, 'TRANSFER rent')).toBe(true)
    expect(matchesWords(task, 'rent car')).toBe(false)
  })

  it('matches everything when the query is blank', () => {
    expect(matchesWords({ title: 'x', description: '' }, '   ')).toBe(true)
  })
})

describe('groupCompleted', () => {
  const today = '2026-11-25'
  const tasks = [
    t('old', at(2026, 11, 20, 9)),
    t('early', at(2026, 11, 25, 8)),
    t('late', at(2026, 11, 25, 17)),
    t('yday', at(2026, 11, 24, 10), { title: 'Yesterday task' }),
    t('open', null),
  ]

  it('groups by completion day, newest day and newest task first, skipping open tasks', () => {
    const groups = groupCompleted(tasks, '', today)
    expect(groups.map((g) => g.day)).toEqual(['2026-11-25', '2026-11-24', '2026-11-20'])
    expect(groups.map((g) => g.label)).toEqual(['Today', 'Yesterday', expect.stringMatching(/20 Nov 2026/)])
    expect(groups[0].items.map((x) => x.id)).toEqual(['late', 'early'])
  })

  it('filters by the search words and drops days left empty', () => {
    const groups = groupCompleted(tasks, 'yesterday', today)
    expect(groups).toHaveLength(1)
    expect(groups[0].items.map((x) => x.id)).toEqual(['yday'])
  })

  it('returns nothing when no completed task matches', () => {
    expect(groupCompleted(tasks, 'nope', today)).toEqual([])
    expect(groupCompleted([], '', today)).toEqual([])
  })
})
