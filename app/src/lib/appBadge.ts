import { glance } from './dashboard'
import { isDesktopTauri } from './platform'
import { localDay, visibleSnapshot } from './sync'

/** localStorage key for the "Show due count on the app icon" switch; on only when stored as `1`. */
const KEY = 'orbit.appBadge'

type BadgingNavigator = Navigator & {
  setAppBadge?: (count?: number) => Promise<void>
  clearAppBadge?: () => Promise<void>
}

/** The number the app icon shows: open Tasks due today or overdue. */
export const dueBadgeCount = (g: { dueToday: number; overdue: number }) => g.dueToday + g.overdue

/** Whether the stored switch value turns the badge on. Anything but `1`, including a missing value, is off. */
export const badgeOn = (raw: string | null) => raw === '1'

/** Whether the browser offers the Badging API here. The desktop app has its own icon and never uses it. */
export function badgeSupported(): boolean {
  if (typeof navigator === 'undefined' || isDesktopTauri()) return false
  return typeof (navigator as BadgingNavigator).setAppBadge === 'function'
}

export function badgePreference(): boolean {
  try {
    return badgeOn(localStorage.getItem(KEY))
  } catch {
    return false
  }
}

export function setBadgePreference(on: boolean): void {
  try {
    localStorage.setItem(KEY, on ? '1' : '0')
  } catch {
    // Storage is blocked: the switch simply does not persist.
  }
}

/** Shows `count` on the app icon, or clears it when `count` is 0. A no-op where the Badging API is missing. */
export async function showBadge(count: number): Promise<void> {
  if (typeof navigator === 'undefined' || isDesktopTauri()) return
  const nav = navigator as BadgingNavigator
  try {
    if (count > 0 && typeof nav.setAppBadge === 'function') await nav.setAppBadge(count)
    else if (typeof nav.clearAppBadge === 'function') await nav.clearAppBadge()
  } catch {
    // The platform can refuse the badge (permissions, unsupported OS); the icon just keeps what it had.
  }
}

/** Recounts from the local replica and sets the icon badge; clears it while the switch is off. */
export async function refreshAppBadge(): Promise<void> {
  if (!badgeSupported()) return
  if (!badgePreference()) return showBadge(0)
  try {
    const today = await localDay()
    await showBadge(dueBadgeCount(glance(await visibleSnapshot(), today)))
  } catch {
    // The replica is not readable yet; the next refresh tries again.
  }
}
