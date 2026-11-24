/** The first-run checklist shows only to a user with no Areas who hasn't dismissed it on this device. */
export const showChecklist = (areaCount: number, dismissed: boolean) => areaCount === 0 && !dismissed

const KEY = 'orbit.onboarding.dismissed'

/** Whether this device dismissed the checklist (a per-device flag). */
export function checklistDismissed(): boolean {
  try {
    return localStorage.getItem(KEY) === '1'
  } catch {
    return false
  }
}

export function dismissChecklist() {
  try {
    localStorage.setItem(KEY, '1')
  } catch {
    // Storage blocked: it will show again next load.
  }
}
