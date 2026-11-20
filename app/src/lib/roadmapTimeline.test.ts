import { expect, test } from 'vitest'
import { layoutTimeline } from './roadmapTimeline'
import type { PhaseSummary } from './sync'

type Item = PhaseSummary['items'][number]
const item = (o: Partial<Item>): Item => ({
  id: 'i',
  kind: 'task',
  title: 'T',
  progress: 0,
  links: [],
  date: null,
  start: null,
  end: null,
  outside: false,
  ...o,
})
const phase = (o: Partial<PhaseSummary>): PhaseSummary => ({
  id: 'p',
  name: 'P',
  start: null,
  end: null,
  completed: false,
  progress: 0,
  current: false,
  gate_id: null,
  gate_title: null,
  locked: false,
  items: [],
  ...o,
})

test('dated Phases are bands on a shared axis and the Today line sits on it', () => {
  const t = layoutTimeline(
    [
      phase({ id: 'a', start: '2026-01-01', end: '2026-01-10' }),
      phase({ id: 'b', start: '2026-01-11', end: '2026-01-20' }),
    ],
    '2026-01-11',
  )
  expect(t.bands[0].x).toBe(0)
  expect(t.bands[0].w).toBeCloseTo(50)
  expect(t.bands[1].x).toBeCloseTo(50)
  expect(t.todayX).toBeCloseTo(50)
  expect(t.bands.every((b) => !b.dashed)).toBe(true)
})

test('undated Phases follow the last dated one, 7 days wide and dashed', () => {
  const t = layoutTimeline(
    [phase({ id: 'a', start: '2026-01-01', end: '2026-01-07' }), phase({ id: 'u1' }), phase({ id: 'u2' })],
    '2026-01-01',
  )
  expect(t.bands[1]).toMatchObject({ dashed: true })
  expect(t.bands[1].x).toBeCloseTo(t.bands[0].x + t.bands[0].w)
  expect(t.bands[1].w).toBeCloseTo(t.bands[0].w)
  expect(t.bands[2].x).toBeCloseTo(t.bands[1].x + t.bands[1].w)
  expect(t.last).toBe('2026-01-21')
})

test('items become bars, diamonds and ticks, and carry the outside flag', () => {
  const t = layoutTimeline(
    [
      phase({
        start: '2026-01-01',
        end: '2026-01-10',
        items: [
          item({ id: 'g', kind: 'goal', start: '2026-01-02', end: '2026-01-05' }),
          item({ id: 'm', kind: 'milestone', date: '2026-01-06' }),
          item({ id: 't', kind: 'task', date: '2026-01-20', outside: true }),
        ],
      }),
    ],
    '2026-01-01',
  )
  const [g, m, k] = t.bands[0].items
  expect([g.shape, m.shape, k.shape]).toEqual(['bar', 'diamond', 'tick'])
  expect(g.w).toBeGreaterThan(0)
  expect(m.x).toBeGreaterThan(g.x)
  expect(k.outside).toBe(true)
  expect(t.bands[0].lanes).toBe(3)
})
