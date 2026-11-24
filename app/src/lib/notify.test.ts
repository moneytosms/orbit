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

import { handleAction } from './notify'

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
