import { beforeAll, expect, test } from 'vitest'
import { readFileSync } from 'node:fs'
import { initSync, parse_quick_add } from '../../wasm/orbit_wasm'
import {
  dateText,
  deadlineLabel,
  deadlineText,
  dueLabel,
  durationLabel,
  durationText,
  monthGrid,
  setToken,
  shiftMonth,
  toggleToken,
} from './captureEdit'
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

test('a picked deadline appends braces, then replaces `by` and brace forms in place', () => {
  const s = 'send report p2'
  const added = edit(s, (p) => setToken(s, p.tokens, 'deadline', deadlineText('2026-12-05')))
  expect(added).toBe('send report p2 {2026-12-05}')
  expect(parse(added)).toMatchObject({ title: 'send report', deadline: '2026-12-05', priority: 2, due: null })

  const by = 'send report by friday p2'
  const swapped = edit(by, (p) => setToken(by, p.tokens, 'deadline', deadlineText('2026-12-09')))
  expect(swapped).toBe('send report {2026-12-09} p2')
  expect(parse(swapped).deadline).toBe('2026-12-09')

  const braces = 'send {march 30} report'
  expect(edit(braces, (p) => setToken(braces, p.tokens, 'deadline', deadlineText('tomorrow')))).toBe(
    'send {tomorrow} report',
  )
  expect(edit(by, (p) => setToken(by, p.tokens, 'deadline', ''))).toBe('send report p2')
})

test('a deadline pick leaves the due date alone', () => {
  const s = 'pay rent tomorrow'
  const out = edit(s, (p) => setToken(s, p.tokens, 'deadline', deadlineText('end of month')))
  expect(parse(out)).toMatchObject({ title: 'pay rent', deadline: '2026-11-30', due: { date: '2026-11-20' } })
})

test('a picked duration appends, replaces `for` and ranges, and clears', () => {
  const s = 'write essay'
  const added = edit(s, (p) => setToken(s, p.tokens, 'duration', durationText(90)))
  expect(added).toBe('write essay for 1h30m')
  expect(parse(added)).toMatchObject({ title: 'write essay', duration_min: 90 })

  const f = 'write essay for 30 min tomorrow'
  const out = edit(f, (p) => setToken(f, p.tokens, 'duration', durationText(60)))
  expect(out).toBe('write essay for 1h tomorrow')
  expect(parse(out)).toMatchObject({ duration_min: 60, due: { date: '2026-11-20' } })

  const r = 'standup tomorrow 3-4pm'
  expect(edit(r, (p) => setToken(r, p.tokens, 'duration', durationText(15)))).toBe('standup tomorrow for 15m')
  expect(edit(f, (p) => setToken(f, p.tokens, 'duration', ''))).toBe('write essay tomorrow')
})

test('duration text round-trips through the parser and labels read well', () => {
  for (const m of [1, 15, 45, 60, 75, 120, 1440]) {
    expect(parse(`x ${durationText(m)}`).duration_min).toBe(m)
  }
  expect(durationText(15)).toBe('for 15m')
  expect(durationText(60)).toBe('for 1h')
  expect(durationText(90)).toBe('for 1h30m')
  expect(durationLabel(90)).toBe('1h 30m')
  expect(durationLabel(45)).toBe('45m')
  expect(durationLabel(120)).toBe('2h')
  expect(deadlineLabel('2026-12-05')).toBe('Sat 5 Dec')
})

test('deadline and duration tokens carry byte offsets after non-ASCII text', () => {
  const s = 'café ☕ by friday for 1h'
  expect(edit(s, (p) => setToken(s, p.tokens, 'deadline', '{2026-12-05}'))).toBe('café ☕ {2026-12-05} for 1h')
  expect(edit(s, (p) => setToken(s, p.tokens, 'duration', 'for 2h'))).toBe('café ☕ by friday for 2h')
})
