import {
  archiveItem,
  completeTask,
  createArea,
  createGoal,
  createPage,
  createProject,
  createRoadmap,
  createRoutine,
  createView,
  dayFrom,
  deleteItem,
  duplicateTask,
  lastTask,
  moveTo,
  setDue,
  setPriority,
  sync,
  tree,
  views,
  type Task,
} from './sync'
import { inbox_id } from '../../wasm/orbit_wasm'
import { readScreen, routes, taskHref, type Route } from './routes'
import { getTheme, setTheme } from './theme'
import { clearCursor, moveCursor, openCursor } from './keys'
import { POSTPONE } from './overdue'
import { postponeById } from './postpone'
import { SNOOZE_LABELS, SNOOZE_PRESETS, snoozeTaskPreset } from './notify'

/** The item an action acts on: the open Task drawer, else the current screen's Area, Project, Page or View. */
export type Target = { kind: 'task' | 'area' | 'project' | 'page' | 'view'; id: string }

export type Action = {
  id: string
  name: string
  group: string
  /** Needs a Task (`task`) or any item (`item`) in focus; hidden from the list without one. */
  needs?: 'task' | 'item'
  run: () => Promise<void> | void
}

/** Hooks the shell fills in for actions that open UI. */
export const ui = {
  newTask: () => {},
  /** Opens the capture window with `text` already typed, such as `@Home` from a widget. */
  capture: (_text: string) => {},
  openPalette: () => {},
  openHelp: () => {},
  /** Shows a short confirmation; the shell fills it in. */
  notify: (_text: string) => {},
}
let override: string | null = null
/** Marks a Task as focused (the palette's own drawer); `null` clears it. */
export const setFocusedTask = (id: string | null) => (override = id)

const drawerOpen = () =>
  typeof document !== 'undefined' &&
  [...document.querySelectorAll('[data-slot="sheet-content"][data-state="open"]')].some((e) =>
    e.textContent?.includes('Task details'),
  )

/** What is in focus now. */
export function focused(): Target | null {
  if (lastTask && drawerOpen()) return { kind: 'task', id: lastTask }
  if (override) return { kind: 'task', id: override }
  const s = readScreen()
  if (s.name === 'area' || s.name === 'project' || s.name === 'page' || s.name === 'view') {
    return { kind: s.name, id: s.id }
  }
  return null
}

const go = (hash: string) => {
  location.hash = hash
}
const toggleTheme = () => {
  const t = getTheme()
  const dark = t === 'dark' || (t === 'system' && matchMedia('(prefers-color-scheme: dark)').matches)
  setTheme(dark ? 'light' : 'dark')
}
const pri = (n: Task['priority']) => (id: string) => setPriority(id, n)

/** Copies the address-bar link to a Task (`#/task/<id>`) and confirms it. */
export async function copyTaskLink(id: string) {
  const url = new URL(taskHref(id), location.href).href
  try {
    await navigator.clipboard.writeText(url)
    ui.notify('Link copied')
  } catch {
    ui.notify('Could not copy the link')
  }
}

/** Every action, built from the current local data (Projects and Views make one entry each). */
export async function actions(): Promise<Action[]> {
  const areas = await tree()
  const all: Action[] = []
  const add = (id: string, name: string, group: string, run: Action['run'], needs?: Action['needs']) =>
    all.push({ id, name, group, run, needs })
  const here = focused()
  const placeId = (() => {
    if (here?.kind === 'area' || here?.kind === 'project') return here.id
    return areas[0]?.area.id ?? inbox_id()
  })()

  add('task.new', 'Create Task', 'Create', () => ui.newTask())
  add('area.new', 'Create Area', 'Create', async () => {
    await createArea('New area')
  })
  add('project.new', 'Create Project', 'Create', async () => {
    const area =
      areas.find((a) => a.area.id === placeId)?.area.id ??
      areas.find((a) => a.projects.some((p) => p.id === placeId))?.area.id
    if (area) await createProject(area, 'New project')
  })
  add('goal.new', 'Create Goal', 'Create', async () => {
    await createGoal(placeId, 'New goal')
    go('#/goals')
  })
  add('page.new', 'Create Page', 'Create', async () => go(`#/page/${await createPage(placeId, 'Untitled')}`))
  add('roadmap.new', 'Create Roadmap', 'Create', async () => {
    const area = areas.find((a) => a.area.id === placeId)?.area.id ?? areas[0]?.area.id
    if (area) go(`#/roadmap/${await createRoadmap(area, 'New roadmap')}`)
  })
  add('routine.new', 'Create Routine', 'Create', async () => {
    await createRoutine(placeId, {
      name: 'New routine',
      schedule: { kind: 'daily' },
      length_days: null,
      time: null,
      goal_id: null,
    })
    go('#/routines')
  })
  add('view.new', 'Create View', 'Create', async () => go(`#/view/${await createView('New view', 'task')}`))

  add(
    'task.complete',
    'Complete',
    'Task',
    () => {
      if (here) void completeTask(here.id)
    },
    'task',
  )
  add('task.duplicate', 'Duplicate task', 'Task', () => (here ? void duplicateTask(here.id) : undefined), 'task')
  add('task.copylink', 'Copy task link', 'Task', () => (here ? void copyTaskLink(here.id) : undefined), 'task')
  add('item.inbox', 'Move to Inbox', 'Task', () => (here ? moveTo(here.id, inbox_id()) : undefined), 'task')
  for (const a of areas) {
    for (const p of a.projects) {
      add(
        `item.move.${p.id}`,
        `Move to Project: ${p.name}`,
        'Task',
        () => (here ? moveTo(here.id, p.id) : undefined),
        'task',
      )
    }
  }
  add('task.today', 'Reschedule: Today', 'Task', () => (here ? setDue(here.id, dayFrom(0)) : undefined), 'task')
  add('task.tomorrow', 'Reschedule: Tomorrow', 'Task', () => (here ? setDue(here.id, dayFrom(1)) : undefined), 'task')
  add('task.cleardate', 'Clear date', 'Task', () => (here ? setDue(here.id, null) : undefined), 'task')
  // Postpone choices (the drawer's buttons too). "Tomorrow" stays above as Reschedule: Tomorrow, for the y key.
  for (const [choice, text] of POSTPONE) {
    if (choice === 'tomorrow') continue
    add(
      `postpone.${choice}`,
      choice === 'none' ? 'Postpone: No date' : `Postpone: ${text}`,
      'Task',
      () => (here ? postponeById(here.id, choice) : undefined),
      'task',
    )
  }
  // Snooze the reminder (the drawer's buttons too): it fires again later, the due date stays.
  for (const preset of SNOOZE_PRESETS) {
    add(
      `snooze.${preset}`,
      `Snooze reminder: ${SNOOZE_LABELS[preset]}`,
      'Task',
      async () => {
        if (!here) return
        await snoozeTaskPreset(here.id, preset)
        ui.notify(`Reminder snoozed until ${SNOOZE_LABELS[preset]}`)
      },
      'task',
    )
  }
  for (const n of [1, 2, 3, 4] as const) {
    add(`task.p${n}`, `Priority ${n}`, 'Task', () => (here ? pri(n)(here.id) : undefined), 'task')
  }
  add('item.delete', 'Delete', 'Item', () => (here ? deleteItem(here.id) : undefined), 'item')
  add('item.archive', 'Archive', 'Item', () => (here ? archiveItem(here.id) : undefined), 'item')

  for (const r of Object.keys(routes) as Route[]) {
    add(`go.${r}`, `Go to ${routes[r].label}`, 'Go to', () => go(`#/${r}`))
  }
  for (const v of await views()) {
    if (v.id.startsWith('00000000-0000-7000-8000-0000000001') && v.hidden) continue
    add(`go.view.${v.id}`, `Go to View: ${v.name}`, 'Go to', () => go(`#/view/${v.id}`))
  }
  for (const a of areas) {
    add(`go.area.${a.area.id}`, `Go to Area: ${a.area.name}`, 'Go to', () => go(`#/area/${a.area.id}`))
    for (const p of a.projects) {
      add(`go.project.${p.id}`, `Go to Project: ${p.name}`, 'Go to', () => go(`#/project/${p.id}`))
    }
  }

  add('theme.toggle', 'Toggle theme', 'App', toggleTheme)
  add('sync.now', 'Sync now', 'App', () => sync())
  add('search.open', 'Search', 'App', () => ui.openPalette())
  add('help.open', 'Keyboard shortcuts', 'App', () => ui.openHelp())
  add('cursor.next', 'Next item', 'List', () => moveCursor(1, setFocusedTask))
  add('cursor.prev', 'Previous item', 'List', () => moveCursor(-1, setFocusedTask))
  add('cursor.open', 'Open item', 'List', () => openCursor())
  return all.filter((a) => !a.needs || (a.needs === 'task' ? here?.kind === 'task' : !!here))
}

/** Subsequence match score of `q` in `name` (higher is better), or null. */
function score(name: string, q: string): number | null {
  const n = name.toLowerCase()
  let at = 0
  let s = 0
  let last = -2
  for (const c of q) {
    const i = n.indexOf(c, at)
    if (i < 0) return null
    s += (i === last + 1 ? 8 : 1) + (i === 0 || n[i - 1] === ' ' || n[i - 1] === ':' ? 6 : 0)
    last = i
    at = i + 1
  }
  return s - n.length * 0.1 + (n.includes(q) ? 20 : 0)
}

/** Actions whose name fuzzy-matches `text` (empty: all), best first. */
export function filterActions(list: Action[], text: string): Action[] {
  const q = text.trim().toLowerCase().replace(/\s+/g, '')
  if (!q) return list
  return list
    .flatMap((a) => {
      const s = score(a.name, q)
      return s === null ? [] : [{ a, s }]
    })
    .sort((x, y) => y.s - x.s)
    .map((x) => x.a)
}

/** Runs the action with this id (the keyboard's way in); a row the action removed drops the cursor. */
export async function runAction(id: string) {
  await (await actions()).find((a) => a.id === id)?.run()
  setTimeout(() => {
    if (override && !document.querySelector('[data-task-row][data-cursor]')) clearCursor(setFocusedTask)
  }, 200)
}
