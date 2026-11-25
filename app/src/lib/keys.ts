/** The keymap: one row per shortcut, each mapped to an action id in the registry (actions.ts). */
export type Key = {
  /** One key (`j`), a modified key (`ctrl+k`) or a two-key chord (`g i`). */
  keys: string
  action: string
  label: string
  group: string
}

export const keymap: Key[] = [
  { keys: 'q', action: 'task.new', label: 'Add task', group: 'General' },
  { keys: '/', action: 'search.open', label: 'Search', group: 'General' },
  { keys: 'ctrl+k', action: 'search.open', label: 'Search and commands', group: 'General' },
  { keys: '?', action: 'help.open', label: 'Show shortcuts', group: 'General' },
  { keys: 'j', action: 'cursor.next', label: 'Next item', group: 'List' },
  { keys: 'k', action: 'cursor.prev', label: 'Previous item', group: 'List' },
  { keys: 'Enter', action: 'cursor.open', label: 'Open item', group: 'List' },
  { keys: 'c', action: 'cursor.open', label: 'Open task and comments', group: 'Task' },
  { keys: 'e', action: 'task.complete', label: 'Complete task', group: 'Task' },
  { keys: '1', action: 'task.p1', label: 'Priority 1', group: 'Task' },
  { keys: '2', action: 'task.p2', label: 'Priority 2', group: 'Task' },
  { keys: '3', action: 'task.p3', label: 'Priority 3', group: 'Task' },
  { keys: '4', action: 'task.p4', label: 'Priority 4', group: 'Task' },
  { keys: 't', action: 'task.today', label: 'Schedule today', group: 'Task' },
  { keys: 'T', action: 'task.cleardate', label: 'Clear date', group: 'Task' },
  { keys: 'y', action: 'task.tomorrow', label: 'Schedule tomorrow', group: 'Task' },
  { keys: 'l', action: 'task.copylink', label: 'Copy task link', group: 'Task' },
  { keys: 'g h', action: 'go.home', label: 'Go to Home', group: 'Go to' },
  { keys: 'g i', action: 'go.inbox', label: 'Go to Inbox', group: 'Go to' },
  { keys: 'g u', action: 'go.upcoming', label: 'Go to Upcoming', group: 'Go to' },
  { keys: 'g g', action: 'go.goals', label: 'Go to Goals', group: 'Go to' },
  { keys: 'g r', action: 'go.routines', label: 'Go to Routines', group: 'Go to' },
  { keys: 'g m', action: 'go.roadmaps', label: 'Go to Roadmaps', group: 'Go to' },
  { keys: 'g s', action: 'go.settings', label: 'Go to Settings', group: 'Go to' },
  { keys: 'g p', action: 'go.profile', label: 'Go to Profile', group: 'Go to' },
]

export const CHORD_MS = 1000

/** The slice of a KeyboardEvent the matcher reads. */
export type KeyInput = {
  key: string
  ctrlKey?: boolean
  metaKey?: boolean
  altKey?: boolean
  /** True when focus is in an input, textarea, select or contenteditable. */
  typing: boolean
}

export type Matcher = {
  /** The action id this key press triggers, or null. */
  press: (e: KeyInput, now: number) => string | null
}

/** Whether `el` is somewhere text is typed. */
export const isTyping = (el: EventTarget | null) =>
  el instanceof Element && !!el.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])')

/** Matches presses against `map`: single keys, Ctrl/Cmd+key, and 2-key chords within CHORD_MS. */
export function matcher(map: Key[] = keymap): Matcher {
  let prefix: { key: string; at: number } | null = null
  const chords = new Set(map.filter((k) => k.keys.includes(' ')).map((k) => k.keys.split(' ')[0]))
  return {
    press(e, now) {
      const mod = !!(e.ctrlKey || e.metaKey)
      const pending = prefix && now - prefix.at <= CHORD_MS ? prefix.key : null
      prefix = null
      if (e.key === 'Shift' || e.key === 'Control' || e.key === 'Meta' || e.key === 'Alt') {
        prefix = pending ? { key: pending, at: now } : null
        return null
      }
      if (mod) {
        if (e.altKey) return null
        return map.find((k) => k.keys === `ctrl+${e.key.toLowerCase()}`)?.action ?? null
      }
      if (e.altKey) return null
      if (e.typing) return null
      if (pending) return map.find((k) => k.keys === `${pending} ${e.key}`)?.action ?? null
      if (chords.has(e.key)) {
        prefix = { key: e.key, at: now }
        return null
      }
      return map.find((k) => k.keys === e.key)?.action ?? null
    },
  }
}

/** Display form of a keymap entry: `ctrl+k` as `Ctrl K`, chords as `g then i`. */
export const keyLabel = (keys: string) =>
  keys.includes(' ')
    ? keys.split(' ').join(' then ')
    : keys.startsWith('ctrl+')
      ? `Ctrl ${keys.slice(5).toUpperCase()}`
      : /^[A-Z]$/.test(keys)
        ? `Shift ${keys}`
        : keys

/** The keymap grouped for the cheat sheet, in table order. */
export function groupedKeys(map: Key[] = keymap): { group: string; keys: Key[] }[] {
  const out: { group: string; keys: Key[] }[] = []
  for (const k of map) {
    const g = out.find((x) => x.group === k.group)
    if (g) g.keys.push(k)
    else out.push({ group: k.group, keys: [k] })
  }
  return out
}

// The list cursor: rows mark themselves with `data-task-row="<id>"` and their open control with `data-open`.
const ROWS = '[data-task-row]'
const rowEls = () => [...document.querySelectorAll<HTMLElement>(ROWS)].filter((e) => e.offsetParent !== null)

/** The row the cursor is on, if still on screen. */
export const cursorRow = () => document.querySelector<HTMLElement>(`${ROWS}[data-cursor]`)

/** Moves the cursor one row (first row when none) and reports the row's task id. */
export function moveCursor(dir: 1 | -1, set: (id: string | null) => void) {
  const rows = rowEls()
  if (!rows.length) return set(null)
  const at = rows.findIndex((r) => r.hasAttribute('data-cursor'))
  const next = rows[at < 0 ? (dir === 1 ? 0 : rows.length - 1) : Math.min(rows.length - 1, Math.max(0, at + dir))]
  for (const r of document.querySelectorAll(`${ROWS}[data-cursor]`)) r.removeAttribute('data-cursor')
  next.setAttribute('data-cursor', 'true')
  next.scrollIntoView?.({ block: 'nearest' })
  next.querySelector<HTMLElement>('[data-open]')?.focus({ preventScroll: true })
  set(next.dataset.taskRow ?? null)
}

/** Opens the row under the cursor. */
export const openCursor = () => cursorRow()?.querySelector<HTMLElement>('[data-open]')?.click()

/** Drops the cursor (rows gone or the screen changed). */
export function clearCursor(set: (id: string | null) => void) {
  for (const r of document.querySelectorAll(`${ROWS}[data-cursor]`)) r.removeAttribute('data-cursor')
  set(null)
}
