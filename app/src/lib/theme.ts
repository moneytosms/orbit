// Orbit Light / Orbit Dark / System, painted as `data-theme` on <html>.
// shortcut: kept per browser in localStorage; moves to synced settings with the Appearance screen (#16).
export type Theme = 'light' | 'dark' | 'system'

const media = matchMedia('(prefers-color-scheme: dark)')
let current: Theme = 'system'
try {
  current = (localStorage.getItem('theme') as Theme | null) ?? 'system'
} catch {
  // Storage blocked: follow the system.
}

function paint() {
  const dark = current === 'dark' || (current === 'system' && media.matches)
  document.documentElement.dataset.theme = dark ? 'dark' : 'light'
}
media.addEventListener('change', paint)
paint()

export const getTheme = () => current
export function setTheme(t: Theme) {
  current = t
  try {
    localStorage.setItem('theme', t)
  } catch {
    // Not remembered; still applied for this visit.
  }
  paint()
}
