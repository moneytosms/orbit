import {
  apply,
  backlinks,
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
  parse_filter,
  print_filter,
  checkin_id,
  local_day,
  placement_id,
  roadmap_summary,
  routine_stats,
  whats_next,
  reminder_schedule,
  stardust_summary,
  badge_shelf,
  routine_days,
  parse_mermaid,
} from '../../wasm/orbit_wasm'
import { req, tx } from './db'
import { uploadPending } from './blobs'
import type { LinkItem, LinkTarget } from './Mention'
import { defaultLayout } from './dashboard'

export type Due = { date: string; time?: string | null }
export type Recurrence = { rule: string; from_completion: boolean }
/** An offset before the due (timed) or deadline, or an absolute local `YYYY-MM-DDTHH:MM`. */
export type Trigger = { kind: 'before'; field: 'due' | 'deadline'; minutes: number } | { kind: 'at'; at: string }
export type Reminder = Trigger & { id: string; spent: boolean }
/** One reminder to show; `late` is true once its time has passed. */
export type Fire = {
  task_id: string | null
  routine_id: string | null
  reminder_id: string | null
  at: string
  late: boolean
  title: string
}
export type Task = {
  kind: 'task'
  id: string
  parent_id: string
  title: string
  completed_at: number | null
  order: string
  description: string
  /** Ids this body links to, kept by core on every write. */
  links: string[]
  /** Hashes of the attachments this description cites. */
  blobs?: string[]
  due: Due | null
  deadline: string | null
  duration_min: number | null
  priority: 1 | 2 | 3 | 4
  label_ids: string[]
  recurrence: Recurrence | null
  goal_id: string | null
  created_at: number | null
  trashed_at: number | null
  pinned_on?: string | null
  not_today_on?: string | null
  reminders: Reminder[]
}
export type Area = {
  kind: 'area'
  id: string
  name: string
  /** An emoji, or "". */
  icon: string
  /** "", "#rrggbb" or "blob:<hash>". */
  cover: string
  order: string
  trashed_at: number | null
  archived_at: number | null
}
export type Project = {
  kind: 'project'
  id: string
  parent_id: string
  name: string
  icon: string
  cover: string
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
  next_deadline_days?: number | null
  next_due_days?: number | null
  next_undated_p1?: boolean | null
  default_reminder?: boolean | null
  target_daily?: number | null
  target_weekly?: number | null
  days_off?: number[] | null
  vacation?: boolean | null
  week_start?: number | null
  dashboard?: DashboardWidget[] | null
  vacation_from?: string | null
  vacations?: [string, string][] | null
  gamification?: boolean | null
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
  pinned_on?: string | null
  not_today_on?: string | null
}
export type Page = {
  kind: 'page'
  id: string
  parent_id: string
  title: string
  body: string
  links: string[]
  blobs?: string[]
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
  entity: 'task' | 'project' | 'goal' | 'page' | 'plan' | 'roadmap' | 'phase'
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
  reminder?: string | null
  pauses: { from: string; to: string | null }[]
  order: string
  created_at: number | null
  trashed_at: number | null
  archived_at: number | null
}
export type Roadmap = {
  kind: 'roadmap'
  id: string
  parent_id: string
  name: string
  order: string
  archived_at: number | null
  trashed_at: number | null
  created_at: number | null
}
export type Phase = {
  kind: 'phase'
  id: string
  roadmap_id: string
  name: string
  order: string
  start: string | null
  end: string | null
  gate_id: string | null
  completed_at: number | null
  archived_at: number | null
  trashed_at: number | null
}
export type Placement = {
  kind: 'placement'
  id: string
  roadmap_id: string
  item_id: string
  phase_id: string | null
  links: string[]
}
export type PhaseSummary = {
  id: string
  name: string
  start: string | null
  end: string | null
  completed: boolean
  progress: number
  current: boolean
  gate_id: string | null
  gate_title: string | null
  locked: boolean
  items: {
    id: string
    kind: 'goal' | 'project' | 'milestone' | 'task'
    title: string
    progress: number
    links: string[]
    date?: string | null
    start?: string | null
    end?: string | null
    outside?: boolean
  }[]
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
/** A Badge earned once (or once per repeat key); never revoked. */
export type Award = { kind: 'award'; id: string; badge: string; key: string; earned_at: number; points: number }
export type Row =
  | Area
  | Project
  | Section
  | Task
  | Label
  | Settings
  | Completion
  | Goal
  | Milestone
  | Page
  | View
  | Routine
  | Checkin
  | Roadmap
  | Phase
  | Placement
  | Award
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
// Milestones and Phases hang off `goal_id` / `roadmap_id` instead of `parent_id`.
const ownerOf = (r: Row) =>
  r.kind === 'milestone' ? r.goal_id : r.kind === 'phase' ? r.roadmap_id : 'parent_id' in r ? r.parent_id : null
const appendOrder = async (parentId: string | null) =>
  orderAfter(
    (await allRows())
      .flatMap((r) => ('order' in r && ownerOf(r) === parentId ? [r.order] : []))
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
export const updateProject = (
  id: string,
  patch: { goal_id?: string | null; name?: string; color?: string; icon?: string; cover?: string },
) => enqueue('project.update', { id, ...patch })
export const updateTask = (id: string, patch: TaskPatch) => enqueue('task.update', { id, ...patch })
export const getTask = (id: string) => {
  lastTask = id
  return tx(['rows'], (s) => req<Task | undefined>(s('rows').get(id)))
}
/** The Task last read by the Task drawer; the command palette acts on it while that drawer is open. */
export let lastTask: string | null = null

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

export type NextSettings = { deadline_days: number; due_days: number; undated_p1: boolean }
const NEXT_DEFAULTS: NextSettings = { deadline_days: 3, due_days: 7, undated_p1: true }
/** The What's next windows and undated-P1 toggle; unset fields read as their defaults. */
export async function getNextSettings(): Promise<NextSettings> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return {
    deadline_days: row?.next_deadline_days ?? NEXT_DEFAULTS.deadline_days,
    due_days: row?.next_due_days ?? NEXT_DEFAULTS.due_days,
    undated_p1: row?.next_undated_p1 ?? NEXT_DEFAULTS.undated_p1,
  }
}
export const setNextSettings = (p: Partial<NextSettings>) =>
  enqueue('settings.update', {
    next_deadline_days: p.deadline_days,
    next_due_days: p.due_days,
    next_undated_p1: p.undated_p1,
  })

export type NextItem = {
  id: string
  kind: 'task' | 'milestone'
  title: string
  tier: number
  reason: string
  breadcrumb: string[]
}
/** Pins an item to the top of today's What's next (or unpins); clears itself tomorrow. */
export const pinItem = (id: string, on: boolean) => enqueue('item.pin', { id, date: on ? localToday() : null })
/** Hides an item from today's What's next (or brings it back); clears itself tomorrow. */
export const notToday = (id: string, on: boolean) => enqueue('item.not_today', { id, date: on ? localToday() : null })
/** The ranked What's next list for this device's local day and time. */
export async function whatsNext(): Promise<NextItem[]> {
  const d = new Date()
  const cfg = await getNextSettings()
  return JSON.parse(
    whats_next(
      JSON.stringify(await allRows()),
      localToday(),
      `${pad(d.getHours())}:${pad(d.getMinutes())}`,
      JSON.stringify(cfg),
    ),
  )
}
export const deleteItem = (id: string) => enqueue('item.delete', { id })
export const restoreItem = (id: string) => enqueue('item.restore', { id })
export const archiveItem = (id: string) => enqueue('item.archive', { id })
export const unarchiveItem = (id: string) => enqueue('item.unarchive', { id })
export type Trashed = Exclude<Row, Completion | Label | Settings | View | Checkin | Placement | Award>
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
export const VIEW_HORIZON = '00000000-0000-7000-8000-000000000105'

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
export const updateView = (id: string, patch: ViewPatch & { kind?: View['entity'] }) =>
  enqueue('view.update', { id, ...patch })
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
    kind: 'place' | 'goal' | 'page' | 'label' | 'priority' | 'date' | 'reminder'
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
  reminders?: Trigger[]
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
  for (const t of p.reminders ?? []) await addReminder(id, t)
  return id
}

const COMPLETES = ['task.complete', 'goal.complete', 'milestone.update']

/** Applies a command to its row via core; throws the error code if core rejects it. */
async function predict(rows: IDBObjectStore, cmd: Command) {
  const key = cmd.type.startsWith('reminder.') ? cmd.args.task_id : cmd.args.id
  const existing = await req(rows.get((key as string) ?? SETTINGS))
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
  await emitRows([(type.startsWith('reminder.') ? args.task_id : args.id) as string | undefined])
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
    await archiveEndedRoutines()
  })()
    .catch((e) => console.warn('sync failed', e))
    .finally(() => (running = undefined)))
}

async function flush() {
  for (;;) {
    // Files go up before the commands that cite them.
    await uploadPending((code) => rejected(code))
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
    let prior: string[] = []
    await tx(['rows', 'outbox', 'meta'], async (s) => {
      for (const cmd of commands) {
        const status = body.statuses[cmd.uuid]
        if (!status) continue // not answered: keep it queued
        // The file isn't on the server yet: keep the command and its guess, and retry next sync.
        if (status.status === 'error' && status.code === 'missing_blob') continue
        answered++
        if (status.status === 'error') {
          // Undo the guess; the delta brings back the server's row if it has one.
          s('rows').delete((cmd.args.id as string) ?? SETTINGS)
          if (COMPLETES.includes(cmd.type)) s('rows').delete(completion_id(cmd.args.id as string, cmd.uuid))
          codes.push(status.code)
        }
        s('outbox').delete(cmd.uuid)
      }
      if (body.full) {
        prior = (await req<IDBValidKey[]>(s('rows').getAllKeys())) as string[]
        s('rows').clear()
      }
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
    const queued = await tx(['outbox'], (s) => req<Command[]>(s('outbox').getAll()))
    await emitRows(
      [...body.delta.map((r) => r.id), ...commands, ...queued].map((x) =>
        typeof x === 'string' ? x : ((x as Command).args.task_id ?? (x as Command).args.id),
      ) as string[],
      prior,
    )
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

/** Sets a Task's priority (1 highest, 4 none). */
export const setPriority = (id: string, priority: Task['priority']) => updateTask(id, { priority })
/** Sets a Task's due date (`YYYY-MM-DD`), or clears it with null. */
export const setDue = (id: string, date: string | null) => updateTask(id, { due: date ? { date } : null })
/** Replaces a Task's labels. */
export const setLabels = (id: string, label_ids: string[]) => updateTask(id, { label_ids })
/** This device's local date shifted by `days`, as `YYYY-MM-DD` (ignores Day start). */
export function dayFrom(days = 0): string {
  const d = new Date()
  d.setDate(d.getDate() + days)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}
/** Sections directly under a Project or Area, in order. */
export async function sectionsOf(parentId: string): Promise<Section[]> {
  return (await visibleRows())
    .filter((r): r is Section => r.kind === 'section' && r.parent_id === parentId)
    .sort(byOrder)
}
/** Name, kind and parent of every visible Area, Project, Section, Goal and Task, by id. */
export async function nameIndex(): Promise<Map<string, { kind: string; name: string; parent_id: string | null }>> {
  const out = new Map<string, { kind: string; name: string; parent_id: string | null }>()
  for (const r of await visibleRows()) {
    if (r.kind === 'task') out.set(r.id, { kind: r.kind, name: r.title, parent_id: r.parent_id })
    else if (r.kind === 'area' || r.kind === 'goal') out.set(r.id, { kind: r.kind, name: r.name, parent_id: null })
    else if (r.kind === 'project' || r.kind === 'section')
      out.set(r.id, { kind: r.kind, name: r.name, parent_id: r.parent_id })
  }
  return out
}

/** A typed-filter parse error: `start..end` are char offsets of the bad token. */
export type FilterError = { start: number; end: number; message: string }
const knownLabels = async () => JSON.stringify({ labels: (await labels()).map((l) => ({ id: l.id, name: l.name })) })
/** Typed filter text to a filter tree; throws a `FilterError` pointing at the bad token. */
export async function parseFilter(text: string): Promise<Node> {
  try {
    return JSON.parse(parse_filter(text, await knownLabels(), localToday()))
  } catch (e) {
    throw JSON.parse(e instanceof Error ? e.message : String(e)) as FilterError
  }
}
/** A filter tree as typed filter text. */
export async function printFilter(node: Node): Promise<string> {
  return print_filter(JSON.stringify(node), await knownLabels())
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
/** Moves Routines past their length to Archived, history kept; already archived ones are not listed. */
async function archiveEndedRoutines() {
  for (const r of await routines()) if ((await routineStats(r.id)).ended) await archiveItem(r.id)
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
  /** `HH:MM` daily reminder on scheduled days. */
  reminder?: string | null
}
/** Creates a Routine under an Area or Project, starting on the Day-start date; returns its id. */
export async function createRoutine(parentId: string, input: RoutineInput): Promise<string> {
  const id = newId()
  const { length_days, time, goal_id, reminder, ...rest } = input
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
  if (reminder) await enqueue('routine.update', { id, reminder })
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

/** Visible Roadmaps (not archived or trashed), in display order. */
export async function roadmaps(): Promise<Roadmap[]> {
  return (await visibleRows()).filter((r): r is Roadmap => r.kind === 'roadmap').sort(byOrder)
}
export async function getRoadmap(id: string): Promise<Roadmap | undefined> {
  return (await visibleRows()).find((r): r is Roadmap => r.kind === 'roadmap' && r.id === id)
}
/** Creates a Roadmap under an Area, Project or Goal; returns its id. */
export async function createRoadmap(parentId: string, name: string): Promise<string> {
  const id = newId()
  await enqueue('roadmap.create', { id, parent_id: parentId, name, order: await appendOrder(parentId) })
  return id
}
export const renameRoadmap = (id: string, name: string) => enqueue('roadmap.update', { id, name })
/** Adds a Phase at the end of a Roadmap; returns its id. */
export async function createPhase(
  roadmapId: string,
  name: string,
  start: string | null = null,
  end: string | null = null,
): Promise<string> {
  const id = newId()
  await enqueue('phase.create', { id, roadmap_id: roadmapId, name, order: await appendOrder(roadmapId), start, end })
  return id
}
export const updatePhase = (
  id: string,
  patch: { name?: string; order?: string; start?: string | null; end?: string | null; gate_id?: string | null },
) => enqueue('phase.update', { id, ...patch })
export const setPhaseDone = (id: string, done: boolean) => enqueue(done ? 'phase.complete' : 'phase.uncomplete', { id })
/** Puts an item in a Phase of a Roadmap without moving it; null takes it out. One Phase per item per Roadmap. */
export const placeItem = (roadmapId: string, itemId: string, phaseId: string | null) =>
  enqueue('placement.set', {
    id: placement_id(roadmapId, itemId),
    roadmap_id: roadmapId,
    item_id: itemId,
    phase_id: phaseId,
  })
/** Phases of a Roadmap with progress, the current flag and their items, from core. */
export async function roadmapSummary(id: string, today?: string): Promise<PhaseSummary[]> {
  const rows = await visibleRows()
  return JSON.parse(roadmap_summary(JSON.stringify(rows), id, today ?? (await localDay()))).phases
}
/** Goals, Projects, Milestones and open Tasks that a Phase can hold, with their titles. */
export async function placeable(): Promise<{ id: string; kind: string; title: string }[]> {
  return (await visibleRows()).flatMap((r): { id: string; kind: string; title: string }[] =>
    r.kind === 'goal' || r.kind === 'project'
      ? [{ id: r.id, kind: r.kind, title: r.name }]
      : r.kind === 'milestone' || (r.kind === 'task' && r.completed_at === null)
        ? [{ id: r.id, kind: r.kind, title: r.title }]
        : [],
  )
}

type Any = { kind: string; id: string; parent_id?: string; trashed_at?: number | null; archived_at?: number | null }
const titleOf = (r: Any) => {
  const x = r as unknown as { title?: string; name?: string }
  return x.title ?? x.name ?? ''
}
/** The screen that shows `row`: its own, or its parent's for items without one. */
function hrefFor(row: Any, byId: Map<string, Any>): string | null {
  for (let r: Any | undefined = row, i = 0; r && i < 32; i++) {
    if (r.kind === 'page') return `#/page/${r.id}`
    if (r.kind === 'project') return `#/project/${r.id}`
    if (r.kind === 'area') return r.id === inbox_id() ? '#/inbox' : `#/area/${r.id}`
    if (r.kind === 'goal' || r.kind === 'milestone') return '#/goals'
    if (r.kind === 'roadmap') return '#/roadmaps'
    r = r.parent_id ? byId.get(r.parent_id) : undefined
  }
  return null
}

/** What a link points at now: its live title and whether it is trashed, archived or gone for good. */
export async function linkTarget(id: string): Promise<LinkTarget | null> {
  const rows = (await allRows()) as unknown as Any[]
  const byId = new Map(rows.map((r) => [r.id, r]))
  const row = byId.get(id)
  if (!row || row.kind === 'purged') return { title: '', kind: row?.kind ?? 'purged', state: 'purged', href: null }
  let state: LinkTarget['state'] = 'ok'
  for (let r: Any | undefined = row, i = 0; r && i < 32; i++) {
    if (r.trashed_at != null) {
      state = 'trashed'
      break
    }
    if (r.archived_at != null) state = 'archived'
    r = r.parent_id ? byId.get(r.parent_id) : undefined
  }
  return { title: titleOf(row), kind: row.kind, state, href: hrefFor(row, byId) }
}

/** Live items matching `query` for the link menus: `[[` offers Pages, `@` everything else linkable. */
export async function searchLinks(char: string, query: string, limit = 8): Promise<LinkItem[]> {
  const q = query.trim().toLowerCase()
  const other = new Set(['area', 'project', 'section', 'task', 'goal', 'milestone', 'routine', 'roadmap'])
  const inbox = inbox_id()
  const rows = (await visibleRows()) as unknown as Any[]
  return rows
    .filter((r) => (char === '[[' ? r.kind === 'page' : other.has(r.kind) && r.id !== inbox))
    .map((r) => ({ id: r.id, label: titleOf(r), kind: r.kind }))
    .filter((i) => i.label && i.label.toLowerCase().includes(q))
    .slice(0, limit)
}

/** Pages and Tasks whose body links to `id`, never from the trash, ordered by kind then title. */
export async function backlinksOf(
  id: string,
): Promise<{ id: string; title: string; kind: string; href: string | null }[]> {
  const rows = await allRows()
  const ids: string[] = backlinks(JSON.stringify(rows), id)
  const byId = new Map((rows as unknown as Any[]).map((r) => [r.id, r]))
  return ids
    .flatMap((i) => {
      const r = byId.get(i)
      return r ? [{ id: i, title: titleOf(r), kind: r.kind, href: hrefFor(r, byId) }] : []
    })
    .sort((a, b) => a.kind.localeCompare(b.kind) || a.title.localeCompare(b.title))
}

/** The Project or Area a Task sits in, where a Page made from its description would go; null in the Inbox. */
export async function pageOwner(taskId: string): Promise<string | null> {
  const byId = new Map((await allRows()).map((r) => [r.id, r as Any]))
  for (let r = byId.get(taskId), i = 0; r && i < 32; i++) {
    if (r.kind === 'project' || r.kind === 'area') return r.id === inbox_id() ? null : r.id
    r = r.parent_id ? byId.get(r.parent_id) : undefined
  }
  return null
}

/** Milestones and linked-Task due ticks per Goal id, for the Timeline. A Task counts towards its own Goal link, else its Project's. */
export async function timelineData(
  goalIds: string[],
): Promise<Map<string, { milestones: Milestone[]; ticks: { id: string; title: string; date: string }[] }>> {
  const rows = await visibleRows()
  const byId = new Map(rows.map((r) => [r.id, r]))
  const out = new Map(
    goalIds.map((id) => [
      id,
      { milestones: [] as Milestone[], ticks: [] as { id: string; title: string; date: string }[] },
    ]),
  )
  for (const r of rows) {
    if (r.kind === 'milestone') out.get(r.goal_id)?.milestones.push(r)
    if (r.kind !== 'task' || !r.due) continue
    let goal = r.goal_id
    let row: Row | undefined = r
    for (let i = 0; !goal && row && i < 32; i++) {
      if (row.kind === 'project') goal = row.goal_id
      row = 'parent_id' in row ? byId.get(row.parent_id) : undefined
    }
    if (goal) out.get(goal)?.ticks.push({ id: r.id, title: r.title, date: r.due.date })
  }
  for (const v of out.values()) v.milestones.sort(byOrder)
  return out
}

/** Adds a reminder to a Task. */
export const addReminder = (taskId: string, trigger: Trigger) =>
  enqueue('reminder.add', { task_id: taskId, reminder: { id: newId(), ...trigger, spent: false } })
export const removeReminder = (taskId: string, id: string) => enqueue('reminder.remove', { task_id: taskId, id })
/** Marks an absolute reminder as fired, so it never shows again. */
export const spendReminder = (taskId: string, id: string) => enqueue('reminder.spend', { task_id: taskId, id })
/** Adds a one-off absolute reminder at `at` (`YYYY-MM-DDTHH:MM`). */
export const snoozeTask = (taskId: string, at: string) => enqueue('reminder.snooze', { task_id: taskId, at })
/** Whether a Task with a timed due gets a reminder at its due time by default (on unless turned off). */
export async function getDefaultReminder(): Promise<boolean> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return row?.default_reminder !== false
}
export const setDefaultReminder = (default_reminder: boolean) => enqueue('settings.update', { default_reminder })
/** Whether Stardust, Levels, the ring, Badges, Patches and their toasts show (on unless turned off). */
export async function getGamification(): Promise<boolean> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return row?.gamification !== false
}
export const setGamification = (gamification: boolean) => enqueue('settings.update', { gamification })
/** `YYYY-MM-DDTHH:MM` in the synced timezone at `now`. */
export async function localMinute(now = Date.now()): Promise<string> {
  const settings = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return new Date(now + zoneOffset(settings?.timezone ?? '', now) * 60_000).toISOString().slice(0, 16)
}
/** The reminders due or coming up (under an hour late ones included), soonest first, from core. */
export async function reminderSchedule(now = Date.now()): Promise<Fire[]> {
  const default_reminder = await getDefaultReminder()
  return JSON.parse(
    reminder_schedule(JSON.stringify(await allRows()), await localMinute(now), JSON.stringify({ default_reminder })),
  )
}

let rowsChanged = (_rows: Row[], _removed: string[]) => {}
/** Called with the rows that now exist and the ids that no longer do, after each local write and sync. */
export const onRowsChanged = (fn: (rows: Row[], removed: string[]) => void) => (rowsChanged = fn)
/** Every stored row, for the first load of the search index. */
export const rowsSnapshot = () => allRows()
async function emitRows(ids: (string | undefined)[], gone: string[] = []) {
  const want = [...new Set(ids.filter((i): i is string => !!i))]
  const found = await tx(['rows'], (s) => Promise.all(want.map((i) => req<Row | undefined>(s('rows').get(i)))))
  const rows = found.filter((r): r is Row => !!r)
  const have = new Set(rows.map((r) => r.id))
  rowsChanged(rows, [...want.filter((i) => !have.has(i)), ...gone.filter((i) => !have.has(i) && !want.includes(i))])
}

/** Runs a typed-filter tree over every Task, as `runView` does, with this device's local date. */
export async function runFilter(node: Node): Promise<Row[]> {
  const view = {
    id: '00000000-0000-7000-8000-0000000001ff',
    name: 'Filter',
    icon: '',
    entity: 'task',
    scope: null,
    filter: node,
    sorts: [],
    group_by: null,
    layout: 'list',
    fields: [],
    show_completed: false,
    pinned: false,
    hidden: false,
    order: '',
    trashed_at: null,
    builtin: null,
  }
  const rows = await allRows()
  const byId = new Map(rows.map((r) => [r.id, r]))
  const groups: ViewGroup[] = JSON.parse(evaluate_view(JSON.stringify(view), JSON.stringify(rows), localToday()))
  return groups.flatMap((g) => g.ids.flatMap((i) => byId.get(i) ?? []))
}

/** Renames an Area or sets its colour, icon or cover. */
export const updateArea = (id: string, patch: { name?: string; color?: string; icon?: string; cover?: string }) =>
  enqueue('area.update', { id, ...patch })
/** Every row that is not trashed or archived (nor under something that is). */
export const visibleSnapshot = () => visibleRows()

export type StardustLevel = { n: number; name: string; floor: number; next: number | null }
export type StardustTargets = {
  today_done: number
  week_done: number
  daily: number
  weekly: number
  streak: number
  vacation: boolean
  days_off: number[]
}
export type Stardust = {
  total: number
  level: StardustLevel
  history: { day: string; points: number }[]
  targets: StardustTargets
}
/** Stardust total, Level, 30-day history and target progress, derived from completion records. */
export async function stardust(now = Date.now()): Promise<Stardust> {
  const settings = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  const off = zoneOffset(settings?.timezone ?? '', now)
  const today = local_day(now, off, settings?.day_start || '00:00')
  return JSON.parse(stardust_summary(JSON.stringify(await allRows()), today, off, settings?.day_start || '00:00'))
}
export const setTargets = (t: {
  target_daily?: number
  target_weekly?: number
  days_off?: number[]
  vacation?: boolean
}) => enqueue('settings.update', t)

export type ShelfItem = {
  id: string
  name: string
  tier: string
  points: number
  repeatable: boolean
  condition: string
  count: number
}
export type Shelf = { badges: ShelfItem[]; patches: number }
/** Every catalog Badge with how often it was earned, plus the Patch count, from core. */
export async function badgeShelf(): Promise<Shelf> {
  return JSON.parse(badge_shelf(JSON.stringify(await allRows())))
}

const LEVEL_KEY = 'orbit.level'
const AWARDS_KEY = 'orbit.awards'
/** Whether more Awards were earned than announced before; the first run only records. Same rule as `levelUp`. */
export function awardUp(n: number): boolean {
  const seen = Number(localStorage.getItem(AWARDS_KEY) ?? 0)
  if (n === seen) return false
  localStorage.setItem(AWARDS_KEY, String(Math.max(n, seen)))
  return seen > 0 && n > seen
}
/** The Level to announce, once: set when the stored Level is lower; the first run only records. */
export function levelUp(n: number): boolean {
  const seen = Number(localStorage.getItem(LEVEL_KEY) ?? 0)
  if (n === seen) return false
  localStorage.setItem(LEVEL_KEY, String(Math.max(n, seen)))
  return seen > 0 && n > seen
}

/** The Calendar's first weekday (0 Sunday, 1 Monday); unset reads as Monday. */
export async function getWeekStart(): Promise<number> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return row?.week_start ?? 1
}
export const setWeekStart = (week_start: number) => enqueue('settings.update', { week_start })

/** Built-in Calendar View id, mirroring `VIEW_CALENDAR` in crates/orbit-core/src/view.rs. */
export const VIEW_CALENDAR = '00000000-0000-7000-8000-000000000106'

/** Moves a Task to `date`, at `time` (`HH:MM`) when given; a Task dropped on the all-day row loses its time. */
export const moveTask = (id: string, date: string, time?: string | null) =>
  updateTask(id, { due: time ? { date, time } : { date } })

/** What the Calendar shows besides Tasks, between `from` and `to` (`YYYY-MM-DD`, inclusive): Routine days with their check-in state, Goals and Milestones. */
export async function calendarData(from: string, to: string) {
  const rows = await visibleRows()
  const all = await allRows()
  const states = new Map<string, CheckState>()
  for (const r of all) if (r.kind === 'checkin' && !r.cleared) states.set(`${r.routine_id}|${r.date}`, r.state)
  const routines = [] as { routine: Routine; days: { date: string; state: CheckState | 'due' }[] }[]
  for (const r of rows) {
    if (r.kind !== 'routine' || r.archived_at !== null) continue
    const dates: string[] = JSON.parse(routine_days(JSON.stringify(all), r.id, from, to))
    routines.push({ routine: r, days: dates.map((date) => ({ date, state: states.get(`${r.id}|${date}`) ?? 'due' })) })
  }
  return {
    routines,
    goals: rows.filter((r): r is Goal => r.kind === 'goal' && r.archived_at === null),
    milestones: rows.filter((r): r is Milestone => r.kind === 'milestone'),
  }
}

/** A Home dashboard widget; the layout is one synced Settings field. */
export type DashboardWidget = {
  id: string
  kind: string
  view_id?: string | null
  x: number
  y: number
  w: number
  h: number
  hidden: boolean
}
/** The dashboard layout; the default one while the setting is unset. */
export async function getDashboard(): Promise<DashboardWidget[]> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  return row?.dashboard ?? defaultLayout()
}
export const setDashboard = (dashboard: DashboardWidget[]) => enqueue('settings.update', { dashboard })

/** Turns vacation on from today, or off, keeping the finished range for the target streak. */
export async function setVacation(on: boolean): Promise<void> {
  const row = await tx(['rows'], (s) => req<Settings | undefined>(s('rows').get(SETTINGS)))
  const today = await localDay()
  if (on) return void (await enqueue('settings.update', { vacation: true, vacation_from: today }))
  const from = row?.vacation_from
  const yesterday = new Date(Date.parse(`${today}T00:00:00Z`) - 86_400_000).toISOString().slice(0, 10)
  const patch: { vacation: boolean; vacation_from: string; vacations?: [string, string][] } = {
    vacation: false,
    vacation_from: '',
  }
  if (from && from <= yesterday) patch.vacations = [...(row?.vacations ?? []), [from, yesterday]]
  await enqueue('settings.update', patch)
}

/** Gates a Phase behind a Milestone (it stays locked until that is done); null removes the gate. */
export const setPhaseGate = (id: string, gateId: string | null) => updatePhase(id, { gate_id: gateId })
/** Replaces the outgoing step links of an item placed in `phaseId`; links point at other placed items. */
export const setLinks = (roadmapId: string, itemId: string, phaseId: string, links: string[]) =>
  enqueue('placement.set', {
    id: placement_id(roadmapId, itemId),
    roadmap_id: roadmapId,
    item_id: itemId,
    phase_id: phaseId,
    links,
  })
/** Every visible Milestone with its Goal, for the Phase gate picker. */
export async function gateChoices(): Promise<{ id: string; title: string; done: boolean }[]> {
  return (await visibleRows())
    .filter((r): r is Milestone => r.kind === 'milestone')
    .map((m) => ({ id: m.id, title: m.title, done: m.done }))
}

export type MermaidItem = {
  key: string
  title: string
  start: string
  end: string
  milestone: boolean
  done: boolean
  links: string[]
}
export type MermaidPhase = { name: string; start: string | null; end: string | null; items: MermaidItem[] }
export type MermaidPlan = { title: string | null; phases: MermaidPhase[] }
export type MermaidResult = { ok: MermaidPlan } | { error: { line: number; message: string } }
/** Parses Mermaid gantt or flowchart text in core; returns the plan or the error line. */
export const parseMermaid = (text: string): MermaidResult => JSON.parse(parse_mermaid(text))
/** Creates Phases and items from a parsed plan inside a Roadmap, placing each item in its Phase. */
export async function importMermaid(roadmapId: string, plan: MermaidPlan): Promise<void> {
  const rows = await visibleRows()
  const roadmap = rows.find((r): r is Roadmap => r.kind === 'roadmap' && r.id === roadmapId)
  if (!roadmap) throw new Error('Roadmap not found')
  const owner = rows.find((r) => r.id === roadmap.parent_id)
  const goal = owner?.kind === 'goal' ? owner : undefined
  let area = roadmap.parent_id
  if (goal) {
    let cur: Row | undefined = goal
    while (cur?.kind === 'goal') cur = rows.find((r) => r.id === (cur as Goal).parent_id)
    area = cur?.id ?? goal.parent_id
  }
  const ids = new Map<string, string>()
  const placed: { id: string; phase: string; links: string[] }[] = []
  for (const phase of plan.phases) {
    const phaseId = await createPhase(roadmapId, phase.name, phase.start, phase.end)
    for (const item of phase.items) {
      let id: string
      if (item.milestone && goal) id = await createMilestone(goal.id, item.title, item.end || item.start)
      else {
        id = await createTask(item.title, area)
        const date = item.end || item.start
        // Flowchart nodes carry no dates, so they import undated.
        if (date || goal)
          await updateTask(id, { ...(date ? { due: { date } } : {}), ...(goal ? { goal_id: goal.id } : {}) })
        if (item.done) await completeTask(id)
      }
      await placeItem(roadmapId, id, phaseId)
      ids.set(item.key, id)
      placed.push({ id, phase: phaseId, links: item.links })
    }
  }
  for (const p of placed) {
    const links = p.links.flatMap((k) => ids.get(k) ?? [])
    if (links.length) await setLinks(roadmapId, p.id, p.phase, links)
  }
}
