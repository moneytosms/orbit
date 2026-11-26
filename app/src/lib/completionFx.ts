// The small reward for finishing a task: the row gives a quick shake, a handful of stardust specks
// drift up from its circle, and a coarse-pointer device (a phone) gets a light tick. All of it is
// skipped when motion is off, or set to follow a system that asks for less.

/** Whether animation is allowed under the Settings motion choice and the system preference. */
export function motionAllowed(): boolean {
  if (typeof document === 'undefined') return false
  const mode = document.documentElement.dataset.motion
  if (mode === 'off') return false
  if (mode === 'system' || mode === undefined) return !matchMedia?.('(prefers-reduced-motion: reduce)').matches
  return true
}

/** A light tick on touch devices. The shell needs the VIBRATE permission on Android. */
export function tick(ms = 8) {
  if (typeof navigator === 'undefined' || !matchMedia?.('(pointer: coarse)').matches || !('vibrate' in navigator))
    return
  try {
    navigator.vibrate(ms)
  } catch {
    /* unsupported by this browser */
  }
}

const SPECKS = 12

/** Shakes the task's row and releases stardust from its circle; nothing happens if the row is gone. */
export function completionFx(taskId: string) {
  tick()
  if (!motionAllowed()) return
  const row = document.querySelector<HTMLElement>(`[data-task-row="${CSS.escape(taskId)}"]`)
  if (!row) return
  row.classList.remove('task-shake')
  void row.offsetWidth // restart the animation if it is already running
  row.classList.add('task-shake')
  setTimeout(() => row.classList.remove('task-shake'), 400)

  const from = (row.querySelector('button') ?? row).getBoundingClientRect()
  const layer = document.createElement('div')
  layer.setAttribute('aria-hidden', 'true')
  layer.className = 'stardust'
  layer.style.left = `${from.left + from.width / 2}px`
  layer.style.top = `${from.top + from.height / 2}px`
  for (let i = 0; i < SPECKS; i++) {
    const speck = document.createElement('i')
    const angle = (Math.PI * 2 * i) / SPECKS + Math.random() * 0.5
    const reach = 18 + Math.random() * 22
    speck.style.setProperty('--dx', `${Math.cos(angle) * reach}px`)
    speck.style.setProperty('--dy', `${Math.sin(angle) * reach - 10}px`)
    speck.style.setProperty('--d', `${520 + Math.random() * 200}ms`)
    speck.style.setProperty('--s', `${3 + Math.random() * 3}px`)
    layer.append(speck)
  }
  document.body.append(layer)
  setTimeout(() => layer.remove(), 800)
}
