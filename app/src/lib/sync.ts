import {
  apply,
  builtin_views,
  completion_for,
  completion_id,
  evaluate_view,
  goal_progress,
  hidden_ids,
  inbox_id,
  new_id,
  next_hlc,
  observe_hlc,
  order_between,
  checkin_id,
  local_day,
  routine_stats,
} from '../../wasm/orbit_wasm'
import { req, tx } from './db'

export type Due = { date: string; time?: string | null }
export type Recurrence = { rule: string; from_completion: boolean }
export type Task = {
  kind: 'task'
  id: string
  parent_id: string
  title: string
  completed_at: number | null
  order: string
  description: string
  due: Due | null
  deadline: string | null
  duration_min: number | null
  priority: 1 | 2 | 3 | 4
  label_ids: string[]
  recurrence: Recurrence | null
  goal_id: string | null
  created_at: number | null
  trashed_at: number | null
}
export type Area = {
  kind: 'area'
  id: string
  name: string
  order: string
  trashed_at: number | null
  archived_at: number | null
}
export type Project = {
  kind: 'project'
  id: string
  parent_id: string
  name: string
  order: string
  goal_id: string | null
  touched_at: number | null
  trashed_at: number | null
  archived_at: number | null
}
export type Section = {
  kind: 'section'
  id: string
  parent_id: string
  name: string
  order: string
  trashed_at: number | null
}
export type Label = { kind: 'label'; id: string; name: string }
export type Settings = {
  kind: 'settings'
  id: string
  theme: string
  accent: string
  start_view: string
  timezone?: string
  day_start?: string
}
/** The fields of a Task that `task.update` can change; absent = unchanged, `null` clears. */
export type TaskPatch = Partial<
  Pick<
    Task,
    'title' | 'description' | 'due' | 'deadline' | 'duration_min' | 'priority' | 'label_ids' | 'recurrence' | 'goal_id'
  >
>
export type Entry = { amount: number; date: string; note: string }
export type Progress = { kind: 'count' } | { kind: 'target'; target: number; entries: Entry[] } | { kind: 'manual' }
export type Goal = {
  kind: 'goal'
  id: string
  /** An Area, or a Goal (then this is a sub-goal). */
  parent_id: string
  name: string
  done_condition: string
  progress: Progress
  start: string | null
  target_date: string | null
  completed_at: number | null
  order: string
  trashed_at: number | null
  archived_at: number | null
}
export type Milestone = {
  kind: 'milestone'
  id: string
  goal_id: string
  title: string
  date: string | null
  done: boolean
  order: string
  trashed_at: number | null
}
export type Page = {
  kind: 'page'
  id: string
  parent_id: string
  title: string
  body: string
  rev: number
  order: string
  trashed_at: number | null
}
export type Cond = { type: 'cond'; path: string; op: string; value?: unknown }
export type Node = { type: 'group'; all: boolean; children: Node[] } | Cond
export type Sort = { field: string; desc: boolean }
/** A saved View. `entity` is what it lists: core stores it as `entity` because `kind` tags the Row. */
export type View = {
  kind: 'view'
  id: string
  name: string
  icon: string
  entity: 'task' | 'project' | 'goal' | 'page' | 'plan'
  scope: string | null
  filter: Node
  sorts: Sort[]
  group_by: string | null
  layout: string
  fields: string[]
  show_completed: boolean
  pinned: boolean
  hidden: boolean
  order: string
  trashed_at: number | null
  builtin: string | null
}
export type ViewGroup = { key: string; ids: string[] }
export type Schedule =
  { kind: 'daily' } | { kind: 'weekdays' } | { kind: 'every'; n: number } | { kind: 'per_week'; n: number }
export type Routine = {
  kind: 'routine'
  id: string
  parent_id: string
  name: string
  schedule: Schedule
  start: string
  length_days: number | null
  time: string | null
  goal_id: string | null
  pauses: { from: string; to: string | null }[]
  order: string
  created_at: number | null
  trashed_at: number | null
  archived_at: number | null
}
export type CheckState = 'done' | 'skipped'
export type Checkin = {
  kind: 'checkin'
  id: string
  routine_id: string
  date: string
  state: CheckState
  cleared: boolean
}
export type DayState = 'done' | 'skipped' | 'missed' | 'paused' | 'off' | 'pending'
export type RoutineStats = {
  current_streak: number
  best_streak: number
  completion_rate: number | null
  days: [string, DayState][]
  ended: boolean
  paused: boolean
}
type Completion = { kind: 'completion'; id: string; item_id: string; at: number }
type Row =
  Area | Project | Section | Task | Label | Settings | Completion | Goal | Milestone | Page | View | Routine | Checkin
type Ordered = { id: string; order: string }
type Hlc = { wall: number; counter: number; device: string }
type Command = { uuid: string; type: string; args: Record<string, unknown>; hlc: Hlc }

// Matches core `SETTINGS`: the singleton settings row has a fixed id.
const SETTINGS = '00000000-0000-7000-8000-000000000002'
const newId = () => new_id(Date.now(), crypto.getRandomValues(new Uint8Array(10)))
const byHlc = (a: Command, b: Command) => a.hlc.wall - b.hlc.wall || a.hlc.counter - b.hlc.counter
const byOrder = (a: Ordered, b: Ordered) => (a.order < b.order ? -1 : a.order > b.order ? 1 : a.id < b.id ? -1 : 1)
let changed = () => {}
let rejected = (_code: string) => {}

let signedOut = (_pending: number) => {}

/** Called with the error code of each command the server refused. */
export const onRejected = (fn: (code: string) => void) => (rejected = fn)
/** Called with the outbox size when the server says this Device is signed out (401). */
export const onSignedOut = (fn: (pending: number) => void) => (signedOut = fn)

/** This Device's last clock reading. Before the first login the Device id is a random placeholder. */
const clock = async (meta: IDBObjectStore): Promise<Hlc> =>
  (await req(meta.get('hlc'))) ?? { wall: 0, counter: 0, device: newId() }

const allRows = () => tx(['rows'], (s) => req<Row[]>(s('rows').getAll()))
/** Rows that are not trashed, archived or under such a row. */
const visibleRows = async () => {
  const rows = await allRows()
  const hidden = new Set<string>(hidden_ids(JSON.stringify(rows)))
  return rows.filter((r) => !hidden.has(r.id))
}

/** Areas except the Inbox, each with its Projects, both in display order. */
export async function tree(): Promise<{ area: Area; projects: Project[] }[]> {
  const rows = await visibleRows()
  const inbox = inbox_id()
  const areas = rows.filter((r): r is Area => r.kind === 'area' && r.id !== inbox).sort(byOrder)
  const projects = rows.filter((r): r is Project => r.kind === 'project')
  return areas.map((area) => ({ area, projects: projects.filter((p) => p.parent_id === area.id).sort(byOrder) }))
}

/** Open Tasks under `parentId`, each followed depth-first by its open subtasks. Depth caps at 3. */
export async function tasksIn(parentId: string): Promise<{ task: Task; depth: number }[]> {
  const open = (await visibleRows()).filter((r): r is Task => r.kind === 'task' && r.completed_at === null)
  const out: { task: Task; depth: number }[] = []
  const seen = new Set<string>()
  const walk = (parent: string, depth: number) => {
    for (const task of open.filter((t) => t.parent_id === parent).sort(byOrder)) {
      if (seen.has(task.id)) continue
      seen.add(task.id)
      out.push({ task, depth })
      walk(task.id, Math.min(depth + 1, 3))
    }
  }
  walk(parentId, 0)
  return out
}

export const orderAfter = (last?: string) => order_between(last, undefined)
export const orderBetween = (a?: string, b?: string) => order_between(a, b)

/** Order after the last child of `parentId` (null: top level). Ids are UUIDv7 but orders decide. */
const appendOrder = async (parentId: string | null) =>
  orderAfter(
    (await allRows())
      .flatMap((r) => ('order' in r && ('parent_id' in r ? r.parent_id : null) === parentId ? [r.order] : []))
      .sort()
      .at(-1),
  )

export const createArea = async (name: string) =>
  enqueue('area.create', { id: newId(), name, order: await appendOrder(null) })
export const createProject = async (areaId: string, name: string) =>
  enqueue('project.create', { id: newId(), parent_id: areaId, name, order: await appendOrder(areaId) })
export const createSection = async (parentId: string, name: string) =>
  enqueue('section.create', { id: newId(), parent_id: parentId, name, order: await appendOrder(parentId) })
/** Without `parentId` the Task lands in the Inbox. Returns the new Task's id. */
export const createTask = async (title: string, parentId?: string) => {
  const id = newId()
  await enqueue('task.create', {
    id,
    title,
    parent_id: parentId,
    order: await appendOrder(parentId ?? inbox_id()),
  })
  return id
}
const pad = (n: number) => String(n).padStart(2, '0')
const localToday = () => {
  const d = new Date()
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}
/** Completes a Task; a recurring one moves to its next due instead, counted from this device's local date. */
export const completeTask = (id: string) => enqueue('task.complete', { id, today: localToday() })
export const updateProject = (id: string, patch: { goal_id: string | null }) =>
  enqueue('project.update', { id, ...patch })
export const updateTask = (id: string, patch: TaskPatch) => enqueue('task.update', { id, ...patch })
export const getTask = (id: string) => tx(['rows'], (s) => req<Task | undefined>(s('rows').get(id)))

/** Creates a flat, global Label and returns its id. */
export async function createLabel(name: string): Promise<string> {
  const id = newId()
  await enqueue('label.create', { id, name })
  return id
}
/** All Labels, sorted by name. */
export async function labels(): Promise<Label[]> {
  return (await allRows()).filter((r): r is Label => r.kind === 'label').sort((a, b) => a.name.localeCompare(b.name))
}

/** The Settings row's appearance, or null before any Device set one. An empty field is unset. */
export async function getSettings(): Promise<{ theme: string; accent: string; start_view: string } | null> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return row ? { theme: row.theme, accent: row.accent, start_view: row.start_view ?? '' } : null
}
/** The synced timezone (empty: this device's) and Day start. */
export async function getDaySettings(): Promise<{ timezone: string; day_start: string }> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return { timezone: row?.timezone ?? '', day_start: row?.day_start || '00:00' }
}
export const updateSettings = (p: { theme?: string; accent?: string }) => enqueue('settings.update', p)
export const setStartView = (v: string) => enqueue('settings.update', { start_view: v })
export const deleteItem = (id: string) => enqueue('item.delete', { id })
export const restoreItem = (id: string) => enqueue('item.restore', { id })
export const archiveItem = (id: string) => enqueue('item.archive', { id })
export const unarchiveItem = (id: string) => enqueue('item.unarchive', { id })
export type Trashed = Exclude<Row, Completion | Label | Settings | View | Checkin>
/** Items deleted by hand (their own `trashed_at`), newest first. */
export async function trashed(): Promise<Trashed[]> {
  return (await allRows())
    .filter((r): r is Trashed => 'trashed_at' in r && r.trashed_at != null)
    .sort((a, b) => (b.trashed_at ?? 0) - (a.trashed_at ?? 0))
}
/** Areas, Projects and Goals archived by hand and not trashed. */
export async function archived(): Promise<(Area | Project | Goal)[]> {
  return (await allRows())
    .filter(
      (r): r is Area | Project | Goal =>
        (r.kind === 'area' || r.kind === 'project' || r.kind === 'goal') &&
        r.archived_at != null &&
        r.trashed_at == null,
    )
    .sort(byOrder)
}
export const moveItem = (id: string, parentId: string | null, order: string) =>
  enqueue('item.move', { id, parent_id: parentId, order })
/** Reparents `id` to the end of `parentId`'s children. */
export const moveTo = async (id: string, parentId: string) => moveItem(id, parentId, await appendOrder(parentId))

/** Creates a Goal under an Area (or a sub-goal under a Goal) and returns its id. */
export async function createGoal(
  parentId: string,
  name: string,
  opts: { progress?: Progress; start?: string; target_date?: string; done_condition?: string } = {},
): Promise<string> {
  const id = newId()
  await enqueue('goal.create', { id, parent_id: parentId, name, order: await appendOrder(parentId), ...opts })
  return id
}
export const updateGoal = (
  id: string,
  patch: {
    name?: string
    done_condition?: string
    progress?: Progress
    start?: string | null
    target_date?: string | null
  },
) => enqueue('goal.update', { id, ...patch })
export const completeGoal = (id: string) => enqueue('goal.complete', { id })
/** Adds a dated amount to a Target Goal. */
export const addEntry = (id: string, amount: number, note = '') =>
  enqueue('goal.add_entry', { id, amount, date: localToday(), note })
export async function createMilestone(goalId: string, title: string, date?: string): Promise<string> {
  const id = newId()
  await enqueue('milestone.create', { id, goal_id: goalId, title, date, order: await appendOrder(goalId) })
  return id
}
export const setMilestoneDone = (id: string, done: boolean) => enqueue('milestone.update', { id, done })

/** Creates a Page under an Area, Project or Goal and returns its id. */
export async function createPage(parentId: string, title: string): Promise<string> {
  const id = newId()
  await enqueue('page.create', { id, parent_id: parentId, title, order: await appendOrder(parentId) })
  return id
}
export const renamePage = (id: string, title: string) => enqueue('page.rename', { id, title })
/** Saves a body edited from `baseRev`, the rev the editor loaded; a stale one comes back as a conflict copy. */
export const editPageBody = (id: string, body: string, baseRev: number) =>
  enqueue('page.edit_body', { id, body, base_rev: baseRev })
export const getPage = (id: string) => tx(['rows'], (s) => req<Page | undefined>(s('rows').get(id)))
/** Pages under an Area, Project or Goal, in display order. */
export async function pagesOf(parentId: string): Promise<Page[]> {
  return (await visibleRows()).filter((r): r is Page => r.kind === 'page' && r.parent_id === parentId).sort(byOrder)
}
/** All Pages, for quick-add `%page` matching. */
export async function pages(): Promise<Page[]> {
  return (await visibleRows()).filter((r): r is Page => r.kind === 'page').sort(byOrder)
}

/** Active (not completed) Goals, in display order. */
export async function goals(): Promise<Goal[]> {
  return (await visibleRows()).filter((r): r is Goal => r.kind === 'goal' && r.completed_at === null).sort(byOrder)
}
/** Every visible Goal, completed or not, in display order. */
export async function allGoals(): Promise<Goal[]> {
  return (await visibleRows()).filter((r): r is Goal => r.kind === 'goal').sort(byOrder)
}
export async function milestonesOf(goalId: string): Promise<Milestone[]> {
  return (await visibleRows())
    .filter((r): r is Milestone => r.kind === 'milestone' && r.goal_id === goalId)
    .sort(byOrder)
}
/** The Goal a Task inherits from its Project (through any parent Tasks), if any. */
export async function inheritedGoal(taskId: string): Promise<string | null> {
  const rows = await allRows()
  const byId = new Map(rows.map((r) => [r.id, r]))
  let row = byId.get(taskId)
  for (let i = 0; row && i < 32; i++) {
    if (row.kind === 'project') return row.goal_id
    row = 'parent_id' in row ? byId.get(row.parent_id) : undefined
  }
  return null
}
/** Progress 0..1 and whether the Goal is behind its straight-line pace, from core. */
export async function goalProgress(goalId: string): Promise<{ progress: number; behind: boolean }> {
  return JSON.parse(goal_progress(JSON.stringify(await allRows()), goalId, localToday()))
}

/** Built-in View ids, mirroring `VIEW_INBOX`..`VIEW_RECENT` in crates/orbit-core/src/view.rs. */
export const VIEW_INBOX = '00000000-0000-7000-8000-000000000101'
export const VIEW_UPCOMING = '00000000-0000-7000-8000-000000000102'
export const VIEW_SLIPPING = '00000000-0000-7000-8000-000000000103'
export const VIEW_RECENT = '00000000-0000-7000-8000-000000000104'

/** Built-in Views merged with stored rows (a stored row replaces its built-in), not trashed, in order. */
export async function views(): Promise<View[]> {
  const stored = (await allRows()).filter((r): r is View => r.kind === 'view')
  const ids = new Set(stored.map((v) => v.id))
  const builtins: View[] = JSON.parse(builtin_views())
  return [...builtins.filter((v) => !ids.has(v.id)), ...stored].filter((v) => v.trashed_at == null).sort(byOrder)
}
export async function getView(id: string): Promise<View | undefined> {
  return (await views()).find((v) => v.id === id)
}
/** Runs a View over every row, from core, with this device's local date. Rows keep the View's order. */
export async function runView(id: string): Promise<{ groups: { key: string; rows: Row[] }[] }> {
  const view = await getView(id)
  if (!view) return { groups: [] }
  const rows = await allRows()
  const byId = new Map(rows.map((r) => [r.id, r]))
  const groups: ViewGroup[] = JSON.parse(evaluate_view(JSON.stringify(view), JSON.stringify(rows), localToday()))
  return { groups: groups.map((g) => ({ key: g.key, rows: g.ids.flatMap((i) => byId.get(i) ?? []) })) }
}
/** Creates a custom View listing `kind`, last in order, and returns its id. */
export async function createView(name: string, kind: View['entity']): Promise<string> {
  const id = newId()
  const last = (await views())
    .map((v) => v.order)
    .sort()
    .at(-1)
  await enqueue('view.create', { id, name, kind, order: orderAfter(last) })
  return id
}
export type ViewPatch = Partial<
  Pick<
    View,
    | 'name'
    | 'icon'
    | 'scope'
    | 'filter'
    | 'sorts'
    | 'group_by'
    | 'layout'
    | 'fields'
    | 'show_completed'
    | 'pinned'
    | 'hidden'
  >
> & { kind?: View['entity'] }
export const updateView = (id: string, patch: ViewPatch) => enqueue('view.update', { id, ...patch })
/** Puts a built-in View back to its defaults. */
export const resetView = (id: string) => enqueue('view.reset', { id })
export const deleteView = (id: string) => enqueue('view.delete', { id })
/** Pinned, visible Views in order, for the sidebar. */
export async function pinnedViews(): Promise<View[]> {
  return (await views()).filter((v) => v.pinned && !v.hidden)
}
/** Moves a View before `beforeId`, or last when null. */
export async function reorderViews(id: string, beforeId: string | null) {
  const rest = (await views()).filter((v) => v.id !== id)
  const i = beforeId === null ? rest.length : rest.findIndex((v) => v.id === beforeId)
  if (i < 0) return
  await enqueue('view.update', { id, order: order_between(rest[i - 1]?.order, rest[i]?.order) })
}

/** A quick-add line as parsed by core `parse::parse` (`parse_quick_add`). */
export type Parsed = {
  title: string
  tokens: {
    kind: 'place' | 'goal' | 'page' | 'label' | 'priority' | 'date'
    text: string
    value: string
    start: number
    end: number
    known: boolean
  }[]
  priority: number | null
  due: Due | null
  recurrence?: Recurrence | null
  place: string | null
  goal: string | null
  page: string | null
  labels: string[]
}

/** The title a quick-add line saves: the parsed title, then the text of unknown `@ $ %` names in input order. */
export const quickTitle = (p: Parsed) =>
  [p.title, ...p.tokens.filter((t) => !t.known && t.kind !== 'label').map((t) => t.text)]
    .filter((s) => s !== '')
    .join(' ')

/**
 * Saves a parsed quick-add line. The Task goes under the named Area or Project, else `fallbackParentId`;
 * unknown `#labels` are created; priority, due and labels go in one `task.update`. Returns the new id.
 */
export async function quickAdd(p: Parsed, fallbackParentId: string): Promise<string> {
  const name = p.place?.toLowerCase()
  const areas = await tree()
  const place =
    name &&
    (areas.find((a) => a.area.name.toLowerCase() === name)?.area.id ??
      areas.flatMap((a) => a.projects).find((pr) => pr.name.toLowerCase() === name)?.id)
  const goalName = p.goal?.toLowerCase()
  const goalId = goalName && (await goals()).find((g) => g.name.toLowerCase() === goalName)?.id
  const all = await labels()
  const ids = new Set<string>()
  for (const l of p.labels) {
    const hit = all.find((x) => x.name.toLowerCase() === l.toLowerCase())
    if (hit) ids.add(hit.id)
  }
  for (const t of p.tokens.filter((t) => !t.known && t.kind === 'label')) {
    const hit = all.find((x) => x.name.toLowerCase() === t.value.toLowerCase())
    if (hit) ids.add(hit.id)
    else {
      const id = await createLabel(t.value)
      all.push({ kind: 'label', id, name: t.value })
      ids.add(id)
    }
  }
  const id = await createTask(quickTitle(p), place || fallbackParentId)
  const patch: TaskPatch = {}
  if (p.priority != null) patch.priority = p.priority as Task['priority']
  if (p.due) patch.due = p.due
  if (p.recurrence) patch.recurrence = p.recurrence
  if (goalId) patch.goal_id = goalId
  if (ids.size) patch.label_ids = [...ids]
  if (Object.keys(patch).length) await updateTask(id, patch)
  return id
}

const COMPLETES = ['task.complete', 'goal.complete', 'milestone.update']

/** Applies a command to its row via core; throws the error code if core rejects it. */
async function predict(rows: IDBObjectStore, cmd: Command) {
  const existing = await req(rows.get((cmd.args.id as string) ?? SETTINGS))
  const from = existing ? JSON.stringify(existing) : undefined
  rows.put(JSON.parse(apply(from, JSON.stringify(cmd))))
  // A completion also logs a record, as the server does.
  if (COMPLETES.includes(cmd.type)) {
    const log = completion_for(from, JSON.stringify(cmd))
    if (log) rows.put(JSON.parse(log))
  }
}

/** Like the server: completing a Task completes its open descendants at the same moment. */
async function cascade(rows: IDBObjectStore, id: string) {
  const all = (await req<Row[]>(rows.getAll())).filter((r): r is Task => r.kind === 'task')
  const at = all.find((t) => t.id === id)?.completed_at
  if (at == null) return
  let parents = [id]
  while (parents.length) {
    const kids = all.filter((t) => t.completed_at === null && parents.includes(t.parent_id))
    for (const t of kids) rows.put({ ...t, completed_at: at })
    parents = kids.map((t) => t.id)
  }
}

async function enqueue(type: string, args: Command['args']) {
  await tx(['rows', 'outbox', 'meta'], async (s) => {
    const hlc: Hlc = JSON.parse(next_hlc(JSON.stringify(await clock(s('meta'))), Date.now()))
    const cmd = { uuid: newId(), type, args, hlc }
    await predict(s('rows'), cmd)
    if (type === 'task.complete') await cascade(s('rows'), args.id as string)
    s('meta').put(hlc, 'hlc')
    s('outbox').put(cmd)
  })
  changed()
  clearTimeout(debounce)
  debounce = setTimeout(sync, 2000)
}
let debounce: ReturnType<typeof setTimeout>

let running: Promise<void> | undefined
let again = false
/** Sends the outbox and merges the delta. Network errors leave the outbox queued. */
export function sync(): Promise<void> {
  // A call during a flush runs one more flush, so writes made meanwhile aren't stranded.
  if (running) again = true
  return (running ??= (async () => {
    do {
      again = false
      await flush()
    } while (again)
  })()
    .catch((e) => console.warn('sync failed', e))
    .finally(() => (running = undefined)))
}

async function flush() {
  for (;;) {
    const [outbox, token] = await tx(['outbox', 'meta'], (s) =>
      Promise.all([req<Command[]>(s('outbox').getAll()), req<number | undefined>(s('meta').get('token'))]),
    )
    const commands = outbox.sort(byHlc).slice(0, 100)
    const res = await fetch('/sync', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ v: 1, token, commands }),
    })
    if (res.status === 401) {
      // The outbox stays; logging in again sends it.
      return signedOut(await tx(['outbox'], (s) => req(s('outbox').count())))
    }
    if (!res.ok) throw new Error(`sync: HTTP ${res.status}`)
    type Status = { status: 'ok' } | { status: 'error'; code: string }
    const body: {
      statuses: Record<string, Status>
      delta: (Row | { kind: 'purged'; id: string })[]
      token: number
      full?: boolean
      clock: Hlc | null
    } = await res.json()
    const codes: string[] = []
    let answered = 0
    await tx(['rows', 'outbox', 'meta'], async (s) => {
      for (const cmd of commands) {
        const status = body.statuses[cmd.uuid]
        if (!status) continue // not answered: keep it queued
        answered++
        if (status.status === 'error') {
          // Undo the guess; the delta brings back the server's row if it has one.
          s('rows').delete((cmd.args.id as string) ?? SETTINGS)
          if (COMPLETES.includes(cmd.type)) s('rows').delete(completion_id(cmd.args.id as string, cmd.uuid))
          codes.push(status.code)
        }
        s('outbox').delete(cmd.uuid)
      }
      if (body.full) s('rows').clear()
      for (const row of body.delta) {
        if (row.kind === 'purged') s('rows').delete(row.id)
        else s('rows').put(row)
      }
      s('meta').put(body.token, 'token')
      if (body.clock) {
        const mine = JSON.stringify(await clock(s('meta')))
        s('meta').put(JSON.parse(observe_hlc(mine, JSON.stringify(body.clock), Date.now())), 'hlc')
      }
      // Re-predict anything queued meanwhile on top of the server's rows. Plain `apply`: an
      // older queued edit can show over a newer server value until it is sent.
      for (const cmd of (await req<Command[]>(s('outbox').getAll())).sort(byHlc)) {
        await predict(s('rows'), cmd).catch(() => {})
      }
    })
    codes.forEach(rejected)
    changed()
    // A partly answered batch would be resent as-is: stop rather than loop.
    if (commands.length < 100 || answered < commands.length) return
  }
}

/** Syncs on open, on focus, 2 s after writes and every 60 s while visible. */
export function start(onChange: () => void) {
  changed = onChange
  sync()
  addEventListener('focus', () => sync())
  setInterval(() => document.visibilityState === 'visible' && sync(), 60_000)
}

/** Minutes the zone `tz` ("" = this device's own) is ahead of UTC at `ms`. */
export function zoneOffset(tz: string, ms: number): number {
  if (!tz) return -new Date(ms).getTimezoneOffset()
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tz,
    hourCycle: 'h23',
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
  }).formatToParts(ms)
  const n = (t: string) => Number(parts.find((p) => p.type === t)?.value)
  const asUtc = Date.UTC(n('year'), n('month') - 1, n('day'), n('hour'), n('minute'), n('second'))
  return Math.round((asUtc - Math.floor(ms / 1000) * 1000) / 60_000)
}
/** The Day-start date now, in the synced timezone and Day start. */
export async function localDay(now = Date.now()): Promise<string> {
  const settings = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return local_day(now, zoneOffset(settings?.timezone ?? '', now), settings?.day_start || '00:00')
}
export const setTimezone = (timezone: string) => enqueue('settings.update', { timezone })
export const setDayStart = (day_start: string) => enqueue('settings.update', { day_start })

/** Visible Routines (not archived or trashed), in display order. */
export async function routines(): Promise<Routine[]> {
  return (await visibleRows()).filter((r): r is Routine => r.kind === 'routine').sort(byOrder)
}
/** Archived Routines, history kept. */
export async function archivedRoutines(): Promise<Routine[]> {
  return (await allRows())
    .filter((r): r is Routine => r.kind === 'routine' && r.archived_at != null && r.trashed_at == null)
    .sort(byOrder)
}
export type RoutineInput = {
  name: string
  schedule: Schedule
  length_days: number | null
  time: string | null
  goal_id: string | null
}
/** Creates a Routine under an Area or Project, starting on the Day-start date; returns its id. */
export async function createRoutine(parentId: string, input: RoutineInput): Promise<string> {
  const id = newId()
  const { length_days, time, goal_id, ...rest } = input
  await enqueue('routine.create', {
    id,
    parent_id: parentId,
    ...rest,
    start: await localDay(),
    length_days,
    time,
    goal_id,
    order: await appendOrder(parentId),
  })
  return id
}
export const updateRoutine = (id: string, patch: Partial<RoutineInput>) => enqueue('routine.update', { id, ...patch })
/** Ticks a Routine's day as done or skipped; the same day on two Devices is one Check-in. */
export const checkIn = (id: string, date: string, state: CheckState) =>
  enqueue('checkin.set', { id: checkin_id(id, date), routine_id: id, date, state })
export const clearCheckIn = (id: string, date: string) => enqueue('checkin.clear', { id: checkin_id(id, date) })
export const pauseRoutine = async (id: string) => enqueue('routine.pause', { id, date: await localDay() })
export const resumeRoutine = async (id: string) => enqueue('routine.resume', { id, date: await localDay() })
/** Streaks, completion rate and per-day states of a Routine up to the Day-start date, from core. */
export async function routineStats(id: string, today?: string): Promise<RoutineStats> {
  const rows = (await allRows()).filter((r) => r.id === id || (r.kind === 'checkin' && r.routine_id === id))
  return JSON.parse(routine_stats(JSON.stringify(rows), id, today ?? (await localDay())))
}
/** Check-in state per date for a Routine, cleared ones left out. */
export async function checkinsOf(id: string): Promise<Map<string, CheckState>> {
  const m = new Map<string, CheckState>()
  for (const r of await allRows()) if (r.kind === 'checkin' && r.routine_id === id && !r.cleared) m.set(r.date, r.state)
  return m
}
