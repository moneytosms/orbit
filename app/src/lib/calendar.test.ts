import { describe, expect, it } from 'vitest'
import { addMinutes, feedToEvents, minutesBetween, toEvents } from './calendar'
import type { Task } from './sync'

const task = (p: Partial<Task>): Task =>
  ({
    kind: 'task',
    id: 't1',
    title: 'Write',
    completed_at: null,
    trashed_at: null,
    due: null,
    deadline: null,
    duration_min: null,
    ...p,
  }) as Task

describe('toEvents', () => {
  it('sizes a timed Task by duration, defaulting to 30 minutes', () => {
    const [a, b] = toEvents(
      [
        task({ id: 'a', due: { date: '2026-03-05', time: '14:00' } }),
        task({ id: 'b', due: { date: '2026-03-05', time: '23:30' }, duration_min: 90 }),
      ],
      [],
      [],
      [],
    )
    expect(a).toMatchObject({ start: '2026-03-05T14:00', end: '2026-03-05T14:30', allDay: false })
    expect(b.end).toBe('2026-03-06T01:00')
  })
  it('puts an untimed due Task and a deadline on the all-day row', () => {
    const ev = toEvents([task({ due: { date: '2026-03-05' }, deadline: '2026-03-09' })], [], [], [])
    expect(ev.map((e) => [e.extendedProps.kind, e.start, e.allDay, e.editable])).toEqual([
      ['task', '2026-03-05', true, true],
      ['deadline', '2026-03-09', true, false],
    ])
  })
  it('skips completed Tasks', () => {
    expect(toEvents([task({ completed_at: 1, due: { date: '2026-03-05' } })], [], [], [])).toEqual([])
  })
  it('marks Routine days done, skipped or due', () => {
    const ev = toEvents(
      [],
      [
        {
          routine: { id: 'r', name: 'Run', time: null },
          days: [
            { date: '2026-03-05', state: 'done' },
            { date: '2026-03-06', state: 'skipped' },
            { date: '2026-03-07', state: 'due' },
          ],
        },
      ],
      [],
      [],
    )
    expect(ev.map((e) => e.classNames[1])).toEqual(['cal-routine-done', 'cal-routine-skipped', 'cal-routine-due'])
  })
  it('shows Goal target dates and Milestones, not finished Goals', () => {
    const ev = toEvents(
      [],
      [],
      [
        { id: 'g', name: 'Ship', target_date: '2026-04-01', completed_at: null },
        { id: 'h', name: 'Done', target_date: '2026-04-02', completed_at: 5 },
        { id: 'i', name: 'None', target_date: null, completed_at: null },
      ],
      [
        { id: 'm', title: 'Beta', date: '2026-03-20', done: false },
        { id: 'n', title: 'Undated', date: null, done: false },
      ],
    )
    expect(ev.map((e) => e.id)).toEqual(['goal:g', 'milestone:m'])
  })
})

describe('minutes', () => {
  it('round-trips', () => {
    expect(minutesBetween('2026-03-05T14:00', addMinutes('2026-03-05T14:00', 45))).toBe(45)
  })
})

describe('feedToEvents', () => {
  it('maps all-day events to UTC dates and timed ones to instants, read-only', () => {
    const [d, t] = feedToEvents([
      { uid: 'a', summary: 'Holiday', start_ms: Date.UTC(2026, 2, 5), end_ms: Date.UTC(2026, 2, 7), all_day: true },
      { uid: 'b', summary: '', start_ms: Date.UTC(2026, 2, 5, 14), end_ms: Date.UTC(2026, 2, 5, 15), all_day: false },
    ])
    expect(d).toMatchObject({ start: '2026-03-05', end: '2026-03-07', allDay: true, editable: false })
    expect(t).toMatchObject({ start: '2026-03-05T14:00:00.000Z', title: '(no title)', allDay: false })
    expect(d.classNames).toEqual(['cal-ical'])
    expect(d.extendedProps).toEqual({ kind: 'ical', id: 'a' })
  })
})
