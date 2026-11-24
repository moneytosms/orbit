import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { addMinutes, eventsOn, feedToEvents, minutesBetween, soon, toEvents } from './calendar'
import type { FeedEvent } from './auth'
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

const feedEvent = (p: Partial<FeedEvent>): FeedEvent => ({
  uid: 'a',
  summary: 'Holiday',
  start_ms: Date.UTC(2026, 2, 5),
  end_ms: Date.UTC(2026, 2, 7),
  all_day: true,
  start_date: '2026-03-05',
  end_date: '2026-03-07',
  feed_id: 1,
  feed_name: 'Work',
  color: 'career',
  ...p,
})

describe('feedToEvents', () => {
  it('maps all-day events to their plain dates and timed ones to instants, read-only, colored by feed', () => {
    const [d, t] = feedToEvents([
      feedEvent({}),
      feedEvent({
        uid: 'b',
        summary: '',
        all_day: false,
        start_date: null,
        end_date: null,
        start_ms: Date.UTC(2026, 2, 5, 14),
        end_ms: Date.UTC(2026, 2, 5, 15),
        color: 'home',
      }),
    ])
    expect(d).toMatchObject({ start: '2026-03-05', end: '2026-03-07', allDay: true, editable: false })
    expect(t).toMatchObject({ start: '2026-03-05T14:00:00.000Z', title: '(no title)', allDay: false })
    expect(d.classNames).toEqual(['cal-ical', 'cal-feed-career'])
    expect(t.classNames).toEqual(['cal-ical', 'cal-feed-home'])
    expect(d.extendedProps).toEqual({ kind: 'ical', id: 'a' })
  })
})

describe('feed events by local day, viewer in UTC+5:30', () => {
  const tz = process.env.TZ
  beforeAll(() => (process.env.TZ = 'Asia/Kolkata'))
  afterAll(() =>
    process.env.TZ === 'Asia/Kolkata' && tz === undefined ? delete process.env.TZ : (process.env.TZ = tz),
  )
  // 00:30 IST on 5 March is still 4 March in UTC.
  const now = new Date('2026-03-04T19:00:00Z')

  it('matches an all-day event on the viewer date, not the UTC date', () => {
    const today = feedEvent({ uid: 'today', start_date: '2026-03-05', end_date: '2026-03-06' })
    const yesterday = feedEvent({ uid: 'yday', start_date: '2026-03-04', end_date: '2026-03-05' })
    expect(eventsOn([today, yesterday], '2026-03-05').map((e) => e.uid)).toEqual(['today'])
    expect(soon([today, yesterday], now).today.map((e) => e.uid)).toEqual(['today'])
  })
  it('keeps a multi-day all-day event on each of its days and drops it on the exclusive end date', () => {
    const trip = feedEvent({ start_date: '2026-03-05', end_date: '2026-03-07' })
    expect([4, 5, 6, 7].map((d) => eventsOn([trip], `2026-03-0${d}`).length)).toEqual([0, 1, 1, 0])
  })
  it('puts a timed event on the local day of its instant', () => {
    const early = feedEvent({
      uid: 'early',
      all_day: false,
      start_date: null,
      end_date: null,
      start_ms: Date.parse('2026-03-04T19:00:00Z'),
      end_ms: Date.parse('2026-03-04T20:00:00Z'),
    })
    const nextDay = feedEvent({
      uid: 'next',
      all_day: false,
      start_date: null,
      end_date: null,
      start_ms: Date.parse('2026-03-05T19:00:00Z'),
      end_ms: Date.parse('2026-03-05T20:00:00Z'),
    })
    const s = soon([early, nextDay], now)
    expect(s.today.map((e) => e.uid)).toEqual(['early'])
    expect(s.tomorrow.map((e) => e.uid)).toEqual(['next'])
  })
})

describe('Calendar widget visibility rule', () => {
  const now = new Date(2026, 2, 5, 9)
  it('is hidden with no events (no visible feeds returns none) or only later ones', () => {
    expect(soon([], now).shown).toBe(false)
    expect(soon([feedEvent({ start_date: '2026-03-07', end_date: '2026-03-08' })], now).shown).toBe(false)
  })
  it('shows for an event today or tomorrow', () => {
    expect(soon([feedEvent({ start_date: '2026-03-06', end_date: '2026-03-07' })], now).shown).toBe(true)
    expect(soon([feedEvent({ start_date: '2026-03-04', end_date: '2026-03-06' })], now).shown).toBe(true)
  })
})
