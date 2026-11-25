import { beforeEach, expect, test, vi } from 'vitest'

const sync = vi.hoisted(() => ({
  checkIn: vi.fn(),
  completeTask: vi.fn(),
  snoozeTask: vi.fn(),
  getDaySettings: vi.fn(async () => ({ day_start: '04:00' })),
  localDay: vi.fn(async () => '2026-11-24'),
  localMinute: vi.fn(async () => '2026-11-24T10:00'),
  reminderSchedule: vi.fn(async () => []),
  spendReminder: vi.fn(),
}))
vi.mock('./sync', () => sync)
vi.mock('./desktop', () => ({ inTauri: () => false, startDesktop: vi.fn() }))

import { handleAction, snoozeUntil, SNOOZE_PRESETS } from './notify'

beforeEach(() => {
  vi.clearAllMocks()
  vi.stubGlobal('location', { hash: '' })
})

test('complete calls completeTask with the task id', async () => {
  await handleAction('complete', 't1')
  expect(sync.completeTask).toHaveBeenCalledWith('t1')
  expect(sync.snoozeTask).not.toHaveBeenCalled()
})

test('snooze10 and snooze60 snooze to now plus the minutes', async () => {
  await handleAction('snooze10', 't1')
  expect(sync.snoozeTask).toHaveBeenLastCalledWith('t1', '2026-11-24T10:10')
  await handleAction('snooze60', 't2')
  expect(sync.snoozeTask).toHaveBeenLastCalledWith('t2', '2026-11-24T11:00')
})

test('tomorrow snoozes to the next day at Day start', async () => {
  await handleAction('tomorrow', 't1')
  expect(sync.snoozeTask).toHaveBeenCalledWith('t1', '2026-11-25T04:00')
})

test('checkin marks the routine done for today', async () => {
  await handleAction('checkin', undefined, 'r1')
  expect(sync.checkIn).toHaveBeenCalledWith('r1', '2026-11-24', 'done')
})

test('actions without their id do nothing', async () => {
  await handleAction('complete')
  await handleAction('snooze10')
  await handleAction('checkin', 't1')
  expect(sync.completeTask).not.toHaveBeenCalled()
  expect(sync.snoozeTask).not.toHaveBeenCalled()
  expect(sync.checkIn).not.toHaveBeenCalled()
})

test('an unknown action mutates nothing; with a task it opens the inbox', async () => {
  await handleAction('bogus', 't1')
  expect(sync.completeTask).not.toHaveBeenCalled()
  expect(sync.snoozeTask).not.toHaveBeenCalled()
  expect(location.hash).toBe('#/inbox')
})

test('snooze presets measure from now', () => {
  const now = '2026-11-24T23:50'
  expect(snoozeUntil('10m', now)).toBe('2026-11-25T00:00')
  expect(snoozeUntil('30m', now)).toBe('2026-11-25T00:20')
  expect(snoozeUntil('1h', '2026-11-24T10:15')).toBe('2026-11-24T11:15')
  expect(snoozeUntil('3h', '2026-11-24T10:15')).toBe('2026-11-24T13:15')
})

test('tomorrow 09:00 is the next calendar day, across month and year ends', () => {
  expect(snoozeUntil('tomorrow9', '2026-11-24T18:00')).toBe('2026-11-25T09:00')
  expect(snoozeUntil('tomorrow9', '2026-11-30T08:00')).toBe('2026-12-01T09:00')
  expect(snoozeUntil('tomorrow9', '2026-12-31T23:59')).toBe('2027-01-01T09:00')
})

test('next Monday 09:00 is strictly after today', () => {
  // 2026-11-25 is a Wednesday, so the next Monday is 2026-11-30.
  expect(snoozeUntil('monday9', '2026-11-25T08:00')).toBe('2026-11-30T09:00')
  // From a Monday it is the Monday a week on, not today.
  expect(snoozeUntil('monday9', '2026-11-30T08:00')).toBe('2026-12-07T09:00')
  // Sunday to Monday is the next day.
  expect(snoozeUntil('monday9', '2026-11-29T20:00')).toBe('2026-11-30T09:00')
  // Across a year end.
  expect(snoozeUntil('monday9', '2026-12-31T12:00')).toBe('2027-01-04T09:00')
})

test('every preset yields a local minute string', () => {
  for (const p of SNOOZE_PRESETS) expect(snoozeUntil(p, '2026-11-24T10:00')).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/)
})
