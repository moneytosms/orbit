import { afterEach, expect, test, vi } from 'vitest'
import { badgeOn, badgeSupported, dueBadgeCount, showBadge } from './appBadge'

afterEach(() => vi.unstubAllGlobals())

test('the badge counts open Tasks due today or overdue', () => {
  expect(dueBadgeCount({ dueToday: 2, overdue: 3 })).toBe(5)
  expect(dueBadgeCount({ dueToday: 0, overdue: 0 })).toBe(0)
})

test('the switch is on only when stored as 1', () => {
  expect(badgeOn('1')).toBe(true)
  expect(badgeOn('0')).toBe(false)
  expect(badgeOn(null)).toBe(false)
  expect(badgeOn('true')).toBe(false)
})

test('showBadge sets the count, clears it at zero, and is a no-op without the Badging API', async () => {
  const setAppBadge = vi.fn(async () => {})
  const clearAppBadge = vi.fn(async () => {})
  vi.stubGlobal('navigator', { setAppBadge, clearAppBadge })
  expect(badgeSupported()).toBe(true)
  await showBadge(4)
  expect(setAppBadge).toHaveBeenCalledWith(4)
  await showBadge(0)
  expect(clearAppBadge).toHaveBeenCalledTimes(1)

  vi.stubGlobal('navigator', {})
  expect(badgeSupported()).toBe(false)
  await expect(showBadge(4)).resolves.toBeUndefined()
})

test('a refused badge is swallowed so the refresh never throws', async () => {
  vi.stubGlobal('navigator', {
    setAppBadge: vi.fn(async () => {
      throw new Error('denied')
    }),
  })
  await expect(showBadge(2)).resolves.toBeUndefined()
})
