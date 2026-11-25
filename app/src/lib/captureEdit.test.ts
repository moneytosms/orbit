import { beforeAll, expect, test } from 'vitest'
import { readFileSync } from 'node:fs'
import { initSync, parse_quick_add } from '../../wasm/orbit_wasm'
import { dateText, dueLabel, monthGrid, setToken, shiftMonth, toggleToken } from './captureEdit'
import type { Parsed } from './sync'

const NOW = '2026-11-19T10:00'
const known = { areas: ['Home', 'Work'], goals: ['Get Fit'], pages: [], labels: ['errand', 'deep work'] }

beforeAll(() => {
  initSync({ module: readFileSync(new URL('../../wasm/orbit_wasm_bg.wasm', import.meta.url)) })
})

const parse = (s: string): Parsed => JSON.parse(parse_quick_add(s, NOW, JSON.stringify(known)))
const edit = (s: string, f: (p: Parsed) => string) => f(parse(s))

test('priority replaces the typed one and clears', () => {
  expect(edit('fix bug p3 now', (p) => setToken('fix bug p3 now', p.tokens, 'priority', 'p1'))).toBe('fix bug p1 now')
  expect(edit('fix bug p3', (p) => setToken('fix bug p3', p.tokens, 'priority', ''))).toBe('fix bug')
  expect(edit('fix bug', (p) => setToken('fix bug', p.tokens, 'priority', 'p2'))).toBe('fix bug p2')
})

test('a picked date replaces the typed phrase in place', () => {
  const s = 'call mom tomorrow 6pm p1'
  const out = edit(s, (p) => setToken(s, p.tokens, 'date', dateText('2026-12-01', '09:30')))
  expect(out).toBe('call mom 2026-12-01 09:30 p1')
  const again = parse(out)
  expect(again.title).toBe('call mom')
  expect(again.due).toEqual({ date: '2026-12-01', time: '09:30' })
})

test('removing a date leaves the rest of the line', () => {
  const s = 'pay rent in 3 days #errand'
  expect(edit(s, (p) => setToken(s, p.tokens, 'date', ''))).toBe('pay rent #errand')
})

test('offsets survive non-ASCII text before the token', () => {
  const s = 'café ☕ fri p3'
  expect(edit(s, (p) => setToken(s, p.tokens, 'priority', 'p1'))).toBe('café ☕ fri p1')
})

test('destination swaps the @ token', () => {
  const s = 'fix tap @home'
  expect(edit(s, (p) => setToken(s, p.tokens, 'place', '@Work'))).toBe('fix tap @Work')
  expect(edit(s, (p) => setToken(s, p.tokens, 'place', ''))).toBe('fix tap')
})

test('labels toggle on and off, including multi-word names', () => {
  let s = 'plan sprint'
  s = edit(s, (p) => toggleToken(s, p.tokens, 'label', 'deep work', '#deep work'))
  expect(s).toBe('plan sprint #deep work')
  expect(parse(s).labels).toEqual(['deep work'])
  s = edit(s, (p) => toggleToken(s, p.tokens, 'label', 'deep work', '#deep work'))
  expect(s).toBe('plan sprint')
})

test('reminders toggle by minutes', () => {
  let s = 'call mom 6pm'
  s = edit(s, (p) => toggleToken(s, p.tokens, 'reminder', '30', '!30m'))
  expect(s).toBe('call mom 6pm !30m')
  s = edit(s, (p) => toggleToken(s, p.tokens, 'reminder', '30', '!30m'))
  expect(s).toBe('call mom 6pm')
})

test('due label', () => {
  expect(dueLabel({ date: '2026-11-20', time: '18:00' })).toBe('Fri 20 Nov, 18:00')
  expect(dueLabel({ date: '2026-11-20', time: null })).toBe('Fri 20 Nov')
})

test('month grid starts on Monday and covers six weeks', () => {
  const g = monthGrid(2026, 11)
  expect(g).toHaveLength(42)
  expect(g[0]).toEqual({ date: '2026-10-26', inMonth: false })
  expect(g.find((c) => c.date === '2026-11-01')?.inMonth).toBe(true)
  expect(g.filter((c) => c.inMonth)).toHaveLength(30)
})

test('shiftMonth wraps years', () => {
  expect(shiftMonth(2026, 12, 1)).toEqual({ y: 2027, m: 1 })
  expect(shiftMonth(2026, 1, -1)).toEqual({ y: 2025, m: 12 })
  expect(shiftMonth(2026, 3, 0)).toEqual({ y: 2026, m: 3 })
})
