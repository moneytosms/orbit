import 'fake-indexeddb/auto'
import { readFileSync } from 'node:fs'
import { beforeAll, beforeEach, expect, test, vi } from 'vitest'
import { initSync, parse_quick_add } from '../../wasm/orbit_wasm'
import { login } from './auth'
import { req, tx } from './db'
import { addFile } from './blobs'
import {
  completeTask,
  createArea,
  createGoal,
  createLabel,
  createMilestone,
  createPage,
  createProject,
  createTask,
  createView,
  archived,
  archiveItem,
  deleteItem,
  restoreItem,
  trashed,
  unarchiveItem,
  editPageBody,
  getPage,
  getSettings,
  getTask,
  runView,
  setPriority,
  setDue,
  dayFrom,
  pinnedViews,
  reorderViews,
  resetView,
  updateView,
  views,
  goalProgress,
  labels,
  moveItem,
  moveTo,
  onRejected,
  onSignedOut,
  orderBetween,
  pagesOf,
  quickAdd,
  reminderSchedule,
  renamePage,
  setMilestoneDone,
  sync,
  tasksIn,
  tree,
  updateSettings,
  updateTask,
  VIEW_SLIPPING,
  VIEW_HORIZON,
  timelineData,
  type Task,
  parseFilter,
  printFilter,
  checkIn,
  clearCheckIn,
  createRoutine,
  localDay,
  routineStats,
  routines,
  whatsNext,
  pinItem,
  notToday,
  createRoadmap,
  createPhase,
  placeItem,
  setPhaseDone,
  roadmapSummary,
  setPhaseGate,
  setLinks,
  gateChoices,
  updateArea,
} from './sync'

const INBOX = '00000000-0000-7000-8000-000000000001'
const inboxTasks = () => tasksIn(INBOX).then((r) => r.map((x) => x.task))
type Body = { v: number; token?: number; commands: { uuid: string; type: string; args: any }[] }

let online: boolean
let requests: Body[]
let delta: object[]
let token = 0
let gate: Promise<void> | undefined
let httpStatus = 200
let rejectWith: string | undefined
let full = false
let order: string[] = []

// Stand-in server: accepts every command and returns whatever delta the test sets.
const DEVICE = '01900000-0000-7000-8000-0000000000d1'
let logins: object[] = []

vi.stubGlobal('fetch', async (url: string, init: RequestInit) => {
  if (!online) throw new TypeError('Failed to fetch')
  if (url === '/auth/login') {
    const body = JSON.parse(init.body as string)
    logins.push(body)
    if (body.password !== 'right') return new Response(JSON.stringify({ error: 'credentials' }), { status: 401 })
    return new Response(JSON.stringify({ device: DEVICE }))
  }
  if (url.startsWith('/blobs/')) {
    order.push(`PUT ${url}`)
    return new Response('', { status: 201 })
  }
  order.push('POST /sync')
  const body: Body = JSON.parse(init.body as string)
  requests.push(body)
  await gate
  if (httpStatus !== 200) return new Response('', { status: httpStatus })
  const status = rejectWith ? { status: 'error', code: rejectWith } : { status: 'ok' }
  const statuses = Object.fromEntries(body.commands.map((c) => [c.uuid, status]))
  const res = { statuses, delta, token: ++token, full }
  delta = []
  full = false
  return new Response(JSON.stringify(res))
})

beforeAll(() => {
  initSync({ module: readFileSync(new URL('../../wasm/orbit_wasm_bg.wasm', import.meta.url)) })
})

beforeEach(() => {
  online = true
  requests = []
  delta = []
  httpStatus = 200
  rejectWith = undefined
})

test('a Task created offline shows at once and is sent once back online', async () => {
  online = false
  await createTask('Buy milk')
  expect((await inboxTasks()).map((t) => t.title)).toEqual(['Buy milk'])
  await sync()

  online = true
  await sync()
  expect(requests).toHaveLength(1)
  expect(requests[0].token).toBeUndefined()
  expect(requests[0].commands).toMatchObject([{ type: 'task.create', args: { title: 'Buy milk' } }])
  const issued = token

  await sync()
  expect(requests[1]).toMatchObject({ v: 1, token: issued, commands: [] })
  expect((await inboxTasks()).map((t) => t.title)).toEqual(['Buy milk'])
})

test('completing a Task hides it at once and queues task.complete', async () => {
  const [milk] = await inboxTasks()
  await completeTask(milk.id)
  expect(await inboxTasks()).toEqual([])
  await sync()
  expect(requests[0].commands).toMatchObject([{ type: 'task.complete', args: { id: milk.id } }])
})

test("another Device's create and complete arrive in the delta", async () => {
  const id = '01900000-0000-7000-8000-0000000000aa'
  const row = { kind: 'task', id, parent_id: INBOX, title: 'From phone', completed_at: null, order: '' }
  delta = [row]
  await sync()
  expect((await inboxTasks()).map((t) => t.title)).toEqual(['From phone'])

  delta = [{ ...row, completed_at: 5000 }]
  await sync()
  expect(await inboxTasks()).toEqual([])
})

test('a write made while a sync is in flight is sent before that sync settles', async () => {
  let release!: () => void
  gate = new Promise((r) => (release = r))
  const inFlight = sync()
  await vi.waitFor(() => expect(requests).toHaveLength(1))
  await createTask('Late')
  void sync()
  gate = undefined
  release()
  await inFlight
  expect(requests.at(-1)!.commands).toMatchObject([{ type: 'task.create', args: { title: 'Late' } }])
})

test('a rejected command is dropped, its guess undone and the rejection reported', async () => {
  const codes: string[] = []
  onRejected((code) => codes.push(code))
  await sync()
  const before = (await inboxTasks()).map((t) => t.title)
  await createTask('Rejected')
  rejectWith = 'exists'
  await sync()
  expect(codes).toEqual(['exists'])
  expect((await inboxTasks()).map((t) => t.title)).toEqual(before)
  rejectWith = undefined
  await sync()
  expect(requests.at(-1)!.commands).toEqual([])
})

test('a rejected completion drops its guessed completion record', async () => {
  const id = await createTask('Rejected done')
  await sync()
  await completeTask(id)
  const completions = () =>
    tx(['rows'], (s) => req<{ kind: string; item_id?: string }[]>(s('rows').getAll())).then((rows) =>
      rows.filter((r) => r.kind === 'completion' && r.item_id === id),
    )
  expect(await completions()).toHaveLength(1)
  rejectWith = 'archived'
  await sync()
  expect(await completions()).toEqual([])
})

test('an old protocol version (426) keeps the outbox for after an update', async () => {
  await createTask('Kept')
  httpStatus = 426
  await sync()
  httpStatus = 200
  await sync()
  expect(requests.at(-1)!.commands).toMatchObject([{ type: 'task.create', args: { title: 'Kept' } }])
})

test('a 401 keeps the outbox and reports it; logging in replays it as the new Device', async () => {
  const pending: number[] = []
  onSignedOut((n) => pending.push(n))
  await createTask('While signed out')
  httpStatus = 401
  await sync()
  expect(pending).toEqual([1])

  httpStatus = 200
  await expect(login('wrong')).rejects.toThrow('credentials')
  await login('right')
  expect(logins.at(-1)).toMatchObject({ platform: 'web' })
  expect(requests.at(-1)!.commands).toMatchObject([{ type: 'task.create', args: { title: 'While signed out' } }])

  await createTask('After login')
  await sync()
  expect(requests.at(-1)!.commands).toMatchObject([{ hlc: { device: DEVICE } }])
})

test('tree() lists Areas and their Projects in display order', async () => {
  await createArea('Home')
  await createArea('Work')
  const home = (await tree()).find((t) => t.area.name === 'Home')!.area
  await createProject(home.id, 'Kitchen')
  await createProject(home.id, 'Garage')
  const shape = (await tree()).map((t) => [t.area.name, t.projects.map((p) => p.name)])
  expect(shape).toEqual([
    ['Home', ['Kitchen', 'Garage']],
    ['Work', []],
  ])
  expect((await tree()).some((t) => t.area.id === INBOX)).toBe(false)
})

test('moveItem places a Task at the predicted order at once', async () => {
  for (const title of ['X1', 'X2', 'X3']) await createTask(title)
  const tasks = await inboxTasks()
  const [x1, , x3] = ['X1', 'X2', 'X3'].map((title) => tasks.find((t) => t.title === title)!)
  await moveItem(x3.id, INBOX, orderBetween(undefined, x1.order))
  const titles = (await inboxTasks()).map((t) => t.title).filter((t) => t.startsWith('X'))
  expect(titles).toEqual(['X3', 'X1', 'X2'])
})

test('moveTo reparents a Task to the end of another parent, subtasks following', async () => {
  await createArea('Errands')
  const area = (await tree()).find((t) => t.area.name === 'Errands')!.area
  await createTask('Already there', area.id)
  const id = await createTask('Y1')
  await createTask('Y1 sub', id)
  await moveTo(id, area.id)
  expect((await inboxTasks()).some((t) => t.title === 'Y1')).toBe(false)
  const rows = (await tasksIn(area.id)).map((r) => [r.task.title, r.depth])
  expect(rows).toEqual([
    ['Already there', 0],
    ['Y1', 0],
    ['Y1 sub', 1],
  ])
  await sync()
  const sent = requests.flatMap((r) => r.commands).filter((c) => c.type === 'item.move')
  expect(sent.at(-1)?.args).toMatchObject({ id, parent_id: area.id })
})

test('completing a parent completes its open subtasks locally at the same moment', async () => {
  await createTask('Parent')
  const parent = (await inboxTasks()).find((t) => t.title === 'Parent')!
  await createTask('Child', parent.id)
  const child = (await tasksIn(parent.id))[0].task
  await completeTask(parent.id)
  const row = (id: string) => tx(['rows'], (s) => req<Task>(s('rows').get(id)))
  const p = await row(parent.id)
  const c = await row(child.id)
  expect(p.completed_at).not.toBeNull()
  expect(c.completed_at).toBe(p.completed_at)
})

test('updateTask sends only the changed field and the row shows it at once', async () => {
  await createTask('Pay rent')
  const task = (await inboxTasks()).find((t) => t.title === 'Pay rent')!
  await updateTask(task.id, { priority: 1 })
  expect((await getTask(task.id))!.priority).toBe(1)
  await sync()
  const sent = requests.flatMap((r) => r.commands).filter((c) => c.type === 'task.update' && c.args.id === task.id)
  expect(sent.map((c) => c.args)).toEqual([{ id: task.id, priority: 1 }])
})

test('updateTask with null clears an optional field', async () => {
  await createTask('Dentist')
  const task = (await inboxTasks()).find((t) => t.title === 'Dentist')!
  await updateTask(task.id, { due: { date: '2026-11-25', time: '09:00' }, duration_min: 30 })
  expect(await getTask(task.id)).toMatchObject({ due: { date: '2026-11-25', time: '09:00' }, duration_min: 30 })
  await updateTask(task.id, { due: null, duration_min: null })
  expect(await getTask(task.id)).toMatchObject({ due: null, duration_min: null })
  await sync()
  const sent = requests.flatMap((r) => r.commands).filter((c) => c.type === 'task.update' && c.args.id === task.id)
  expect(sent.at(-1)!.args).toEqual({ id: task.id, due: null, duration_min: null })
})

test('createLabel adds a flat Label that a Task can carry, listed by name', async () => {
  await createTask('Groceries')
  const task = (await inboxTasks()).find((t) => t.title === 'Groceries')!
  const zed = await createLabel('zz-errand')
  const amy = await createLabel('aa-home')
  expect((await labels()).map((l) => l.name).filter((n) => n.endsWith('errand') || n.endsWith('home'))).toEqual([
    'aa-home',
    'zz-errand',
  ])
  await updateTask(task.id, { label_ids: [zed, amy] })
  expect((await getTask(task.id))!.label_ids).toEqual([zed, amy])
  await updateTask(task.id, { label_ids: [amy] })
  expect((await getTask(task.id))!.label_ids).toEqual([amy])
})

test('updateSettings creates the settings row, then changes only the fields given', async () => {
  expect(await getSettings()).toBeNull()
  await updateSettings({ theme: 'dark', accent: '#c2552f' })
  expect(await getSettings()).toEqual({ theme: 'dark', accent: '#c2552f', start_view: '' })
  await updateSettings({ theme: 'light' })
  expect(await getSettings()).toEqual({ theme: 'light', accent: '#c2552f', start_view: '' })
  // Accent reset syncs as '' and reads back as cleared, not missing.
  await updateSettings({ accent: '' })
  expect(await getSettings()).toEqual({ theme: 'light', accent: '', start_view: '' })
  await sync()
  const sent = requests.flatMap((r) => r.commands).filter((c) => c.type === 'settings.update')
  expect(sent.map((c) => c.args)).toEqual([{ theme: 'dark', accent: '#c2552f' }, { theme: 'light' }, { accent: '' }])
})

const parseLine = (text: string, labelNames: string[] = [], goalNames: string[] = []) =>
  JSON.parse(
    parse_quick_add(
      text,
      '2026-11-19T10:00',
      JSON.stringify({ areas: [], goals: goalNames, pages: [], labels: labelNames }),
    ),
  )

test('quickAdd applies priority, due and a known label in one update', async () => {
  const errand = await createLabel('qa-errand')
  const id = await quickAdd(parseLine('Pay rent p1 #qa-errand tomorrow', ['qa-errand']), INBOX)
  expect(await getTask(id)).toMatchObject({
    title: 'Pay rent',
    priority: 1,
    due: { date: '2026-11-20' },
    label_ids: [errand],
  })
  await sync()
  const sent = requests.flatMap((r) => r.commands).filter((c) => c.type === 'task.update' && c.args.id === id)
  expect(sent).toMatchObject([{ args: { id, priority: 1, label_ids: [errand] } }])
})

test('quickAdd turns "every 3 days" into a recurrence with a first due', async () => {
  const id = await quickAdd(parseLine('water plants every 3 days'), INBOX)
  expect(await getTask(id)).toMatchObject({
    title: 'water plants',
    recurrence: { rule: '3 days', from_completion: false },
    due: { date: '2026-11-19' },
  })
})

test('quickAdd turns "!30m" into an offset reminder that fires before the due', async () => {
  const id = await quickAdd(parseLine('call mom tomorrow 9am !30m'), INBOX)
  const task = (await getTask(id))!
  expect(task.title).toBe('call mom')
  expect(task.reminders).toMatchObject([{ kind: 'before', field: 'due', minutes: 30, spent: false }])
  const fires = await reminderSchedule(new Date(2026, 10, 20, 8, 20).getTime())
  expect(fires.filter((f) => f.task_id === id)).toMatchObject([{ at: '2026-11-20T08:30', late: false }])
})

test('quickAdd keeps an unknown $goal name in the title', async () => {
  const id = await quickAdd(parseLine('Read book $qa-goal'), INBOX)
  expect((await getTask(id))!.title).toBe('Read book $qa-goal')
})

test('quickAdd links a known $goal and strips the token', async () => {
  await createArea('qa-goal-area')
  const areaId = (await tree()).find((t) => t.area.name === 'qa-goal-area')!.area.id
  const goal = await createGoal(areaId, 'Get Fit')
  const id = await quickAdd(parseLine('run 5k $Get Fit', [], ['Get Fit']), INBOX)
  expect(await getTask(id)).toMatchObject({ title: 'run 5k', goal_id: goal })
})

test('a goal counts a done milestone and reports behind pace against its dates', async () => {
  const a = (await tree()).find((t) => t.area.name === 'qa-goal-area')!.area.id
  const g = await createGoal(a, 'Pace', { start: '2026-01-01', target_date: '2099-12-31' })
  const m = await createMilestone(g, 'Half way')
  await createMilestone(g, 'Done')
  expect((await goalProgress(g)).progress).toBe(0)
  await setMilestoneDone(m, true)
  const after = await goalProgress(g)
  expect(after.progress).toBe(0.5)
  expect(after.behind).toBe(false)
})

test('quickAdd creates an unknown #label on save', async () => {
  const id = await quickAdd(parseLine('Sort mail #qa-new'), INBOX)
  const label = (await labels()).find((l) => l.name === 'qa-new')
  expect(label).toBeDefined()
  expect((await getTask(id))!.label_ids).toEqual([label!.id])
})

test('a Page edit predicts the new body and bumps the rev; a stale base_rev keeps the body', async () => {
  const a = (await tree()).find((t) => t.area.name === 'qa-goal-area')!.area.id
  const id = await createPage(a, 'Notes')
  expect(await getPage(id)).toMatchObject({ title: 'Notes', body: '', rev: 0 })
  await renamePage(id, 'Meeting notes')
  await editPageBody(id, '# Hi', 0)
  expect(await getPage(id)).toMatchObject({ title: 'Meeting notes', body: '# Hi', rev: 1 })
  await editPageBody(id, 'stale', 0)
  expect(await getPage(id)).toMatchObject({ body: '# Hi', rev: 1 })
  expect((await pagesOf(a)).map((p) => p.id)).toEqual([id])
  await sync()
  expect(requests.at(-1)!.commands.at(-1)).toMatchObject({
    type: 'page.edit_body',
    args: { id, body: 'stale', base_rev: 0 },
  })
})

test('delete hides a Task and its subtasks, Trash lists it and restore brings it back', async () => {
  await sync()
  const parent = await createTask('Trash me')
  const child = await createTask('Child', parent)
  await sync()
  await deleteItem(parent)
  expect((await inboxTasks()).map((t) => t.title)).not.toContain('Trash me')
  expect((await tasksIn(parent)).map((r) => r.task.id)).not.toContain(child)
  expect((await trashed()).map((r) => r.id)).toEqual([parent])
  await sync()
  expect(requests.at(-1)!.commands).toMatchObject([{ type: 'item.delete', args: { id: parent } }])
  await restoreItem(parent)
  expect((await inboxTasks()).map((t) => t.title)).toContain('Trash me')
  expect(await trashed()).toEqual([])
  await sync()
})

test('archive hides an Area with its Projects; unarchive restores them', async () => {
  await createArea('Shelf')
  const area = (await tree()).find((t) => t.area.name === 'Shelf')!.area
  await createProject(area.id, 'Inside')
  await archiveItem(area.id)
  expect((await tree()).map((t) => t.area.name)).not.toContain('Shelf')
  expect((await archived()).map((r) => r.id)).toEqual([area.id])
  await unarchiveItem(area.id)
  expect((await tree()).map((t) => t.area.name)).toContain('Shelf')
  expect(await archived()).toEqual([])
  await sync()
})

test('a purged row in the delta deletes the local row', async () => {
  const id = await createTask('Doomed')
  await sync()
  delta = [{ kind: 'purged', id, purged_at: 1 }]
  await sync()
  expect(await getTask(id)).toBeUndefined()
})

test('a full delta clears local rows but keeps the outbox', async () => {
  const old = await createTask('Cleared')
  await sync()
  const other = '01900000-0000-7000-8000-0000000000bb'
  full = true
  delta = [{ kind: 'task', id: other, parent_id: INBOX, title: 'Server only', completed_at: null, order: '' }]
  let release!: () => void
  gate = new Promise((r) => (release = r))
  const sent = requests.length
  const inFlight = sync()
  await vi.waitFor(() => expect(requests.length).toBeGreaterThan(sent))
  const kept = await createTask('Queued meanwhile')
  gate = undefined
  release()
  await inFlight
  expect(await getTask(other)).toBeDefined()
  expect(await getTask(old)).toBeUndefined()
  expect(await getTask(kept)).toBeDefined()
})

test('runView on the built-in Upcoming returns a Task due tomorrow, grouped by day', async () => {
  const d = new Date(Date.now() + 86_400_000)
  const tomorrow = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  const id = await createTask('Due tomorrow')
  await updateTask(id, { due: { date: tomorrow } })
  const upcoming = '00000000-0000-7000-8000-000000000102'
  const { groups } = await runView(upcoming)
  const hit = groups.find((g) => g.rows.some((r) => r.id === id))
  expect(hit?.key).toBe(tomorrow)
})

test('hiding a builtin takes it out of pinnedViews; resetView brings it back', async () => {
  expect((await pinnedViews()).map((v) => v.id)).toContain(VIEW_SLIPPING)
  await updateView(VIEW_SLIPPING, { hidden: true })
  expect((await pinnedViews()).map((v) => v.id)).not.toContain(VIEW_SLIPPING)
  expect((await views()).find((v) => v.id === VIEW_SLIPPING)?.hidden).toBe(true)
  await resetView(VIEW_SLIPPING)
  expect((await pinnedViews()).map((v) => v.id)).toContain(VIEW_SLIPPING)
  expect((await views()).find((v) => v.id === VIEW_SLIPPING)?.hidden).toBe(false)
})

test('reorderViews moves a view before another, then last', async () => {
  const first = await createView('Reorder A', 'task')
  const second = await createView('Reorder B', 'task')
  await reorderViews(second, first)
  let ids = (await views()).map((v) => v.id)
  expect(ids.indexOf(second)).toBe(ids.indexOf(first) - 1)

  await reorderViews(second, null)
  ids = (await views()).map((v) => v.id)
  expect(ids.at(-1)).toBe(second)
})

test('a cross-column priority drop updates the Task and the view regroups', async () => {
  const id = await createTask('Drag me')
  const view = await createView('By priority', 'task')
  await updateView(view, { group_by: 'priority' })
  const keyOf = async () => (await runView(view)).groups.find((g) => g.rows.some((r) => r.id === id))?.key
  expect(await keyOf()).toBe('4')
  await setPriority(id, 1)
  expect((await getTask(id))?.priority).toBe(1)
  expect(await keyOf()).toBe('1')
  await setDue(id, dayFrom(1))
  expect((await getTask(id))?.due?.date).toBe(dayFrom(1))
  await setDue(id, null)
  expect((await getTask(id))?.due).toBeNull()
})

test('typed filters round-trip through the tree and point at bad tokens', async () => {
  const career = await createLabel('Career')
  const text = 'p1 & #Career & due before: next week'
  const tree = await parseFilter(text)
  expect(tree).toEqual({
    type: 'group',
    all: true,
    children: [
      { type: 'cond', path: 'priority', op: 'is', value: 1 },
      { type: 'cond', path: 'label', op: 'is', value: career },
      { type: 'cond', path: 'due', op: 'before', value: '+1w' },
    ],
  })
  expect(await printFilter(tree)).toBe(text)
  await expect(parseFilter('p1 & #Nope')).rejects.toMatchObject({ start: 5, end: 10 })
})

const shiftDay = (day: string, by: number) => {
  const d = new Date(`${day}T00:00:00Z`)
  d.setUTCDate(d.getUTCDate() + by)
  return d.toISOString().slice(0, 10)
}

test('a Routine ticked today counts a streak; clearing the tick undoes it', async () => {
  await createArea('Health')
  const [{ area: a }] = (await tree()).filter((x) => x.area.name === 'Health')
  const id = await createRoutine(a.id, {
    name: 'Meds',
    schedule: { kind: 'daily' },
    length_days: null,
    time: '09:00',
    goal_id: null,
  })
  expect((await routines()).map((r) => r.id)).toContain(id)
  const today = await localDay()
  expect((await routineStats(id)).days.at(-1)).toEqual([today, 'pending'])
  await checkIn(id, today, 'done')
  let stats = await routineStats(id)
  expect(stats.current_streak).toBe(1)
  expect(stats.days.at(-1)).toEqual([today, 'done'])
  await clearCheckIn(id, today)
  stats = await routineStats(id)
  expect(stats.current_streak).toBe(0)
  expect(stats.days.at(-1)).toEqual([today, 'pending'])
})

test('a skipped day keeps the streak and is left out of the completion rate', async () => {
  const [{ area }] = (await tree()).filter((x) => x.area.name === 'Health')
  const id = await createRoutine(area.id, {
    name: 'Stretch',
    schedule: { kind: 'daily' },
    length_days: null,
    time: null,
    goal_id: null,
  })
  const today = await localDay()
  // Backdate the start so there are earlier days to tick.
  await tx(['rows'], async (s) => {
    const row = await req<any>(s('rows').get(id))
    s('rows').put({ ...row, start: shiftDay(today, -3) })
  })
  await checkIn(id, shiftDay(today, -3), 'done')
  await checkIn(id, shiftDay(today, -2), 'skipped')
  await checkIn(id, shiftDay(today, -1), 'done')
  await checkIn(id, today, 'done')
  const stats = await routineStats(id)
  expect(stats.current_streak).toBe(3)
  expect(stats.best_streak).toBe(3)
  expect(stats.completion_rate).toBe(1)
  expect(stats.days.map((d) => d[1])).toEqual(['done', 'skipped', 'done', 'done'])
})

test("pin moves a task to the top of today's What's next; not_today hides it", async () => {
  const a = await createTask('Next A')
  const b = await createTask('Next B')
  await updateTask(a, { due: { date: dayFrom(0) }, priority: 1 })
  await updateTask(b, { due: { date: dayFrom(0) }, priority: 3 })
  let ids = (await whatsNext()).map((i) => i.id)
  expect(ids.indexOf(a)).toBeLessThan(ids.indexOf(b))

  await pinItem(b, true)
  const list = await whatsNext()
  expect(list[0].id).toBe(b)
  expect(list[0].tier).toBe(1)

  await notToday(b, true)
  ids = (await whatsNext()).map((i) => i.id)
  expect(ids).not.toContain(b)
  expect(ids).toContain(a)

  await notToday(b, false)
  expect((await whatsNext()).map((i) => i.id)).toContain(b)
})

import { backlinksOf, linkTarget, searchLinks } from './sync'

test('a Page body with an orbit link predicts the link index and a Backlink on the target', async () => {
  await createArea('lk-area')
  const area = (await tree()).find((t) => t.area.name === 'lk-area')!.area.id
  const target = await createPage(area, 'Link target')
  const source = await createPage(area, 'Link source')
  await editPageBody(source, `see [Link target](orbit:${target}) and [x](orbit:nope)`, 0)
  expect((await getPage(source))!.links).toEqual([target])
  expect((await backlinksOf(target)).map((b) => b.id)).toEqual([source])
  expect(await linkTarget(target)).toMatchObject({ title: 'Link target', kind: 'page', state: 'ok' })
  expect((await searchLinks('[[', 'link tar')).map((i) => i.id)).toEqual([target])
  expect((await searchLinks('@', 'link tar')).map((i) => i.id)).toEqual([])
})

test('a link target shows its live title and trashed or archived state; trashed sources drop out', async () => {
  const area = (await tree()).find((t) => t.area.name === 'lk-area')!.area.id
  const target = await createPage(area, 'Old name')
  const source = await createPage(area, 'Src2')
  await editPageBody(source, `[Old name](orbit:${target})`, 0)
  await renamePage(target, 'New name')
  expect((await linkTarget(target))!.title).toBe('New name')
  await deleteItem(target)
  expect((await linkTarget(target))!.state).toBe('trashed')
  await restoreItem(target)
  await deleteItem(source)
  expect(await backlinksOf(target)).toEqual([])
  await restoreItem(source)
  await createProject(area, 'lk-proj')
  const proj = (await tree()).flatMap((t) => t.projects).find((p) => p.name === 'lk-proj')!.id
  const inProj = await createPage(proj, 'In project')
  await archiveItem(proj)
  expect((await linkTarget(inProj))!.state).toBe('archived')
  expect(await linkTarget('00000000-0000-7000-8000-00000000dead')).toMatchObject({ state: 'purged' })
})

test('Horizon builtin runs and timelineData returns a milestone and a task tick', async () => {
  const horizon = (await views()).find((v) => v.id === VIEW_HORIZON)
  expect(horizon?.layout).toBe('timeline')
  expect(horizon?.entity).toBe('goal')
  await createArea('tl-area')
  const areaId = (await tree()).find((t) => t.area.name === 'tl-area')!.area.id
  const g = await createGoal(areaId, 'Horizon goal', { start: '2026-01-01', target_date: '2026-06-30' })
  await createMilestone(g, 'Halfway', '2026-03-01')
  const t = await createTask('Linked', INBOX)
  await updateTask(t, { goal_id: g, due: { date: '2026-02-02' } })
  const { groups } = await runView(VIEW_HORIZON)
  expect(groups.flatMap((x) => x.rows).some((r) => r.id === g)).toBe(true)
  const data = (await timelineData([g])).get(g)!
  expect(data.milestones.map((m) => m.title)).toEqual(['Halfway'])
  expect(data.ticks).toEqual([{ id: t, title: 'Linked', date: '2026-02-02' }])
})

test('an attachment added offline is uploaded before the command that cites it', async () => {
  const area = (await tree()).find((t) => t.area.name === 'lk-area')!.area.id
  const page = await createPage(area, 'With file')
  await sync()
  online = false
  const { hash, markdown } = await addFile(new File(['hello'], 'note.txt', { type: 'text/plain' }))
  expect(markdown).toBe(`[note.txt](blob:${hash})`)
  await editPageBody(page, `see ${markdown}`, 0)
  online = true
  order = []
  await sync()
  expect(order[0]).toBe(`PUT /blobs/${hash}`)
  expect(order.indexOf('POST /sync')).toBeGreaterThan(0)
  expect(requests.at(-1)!.commands).toMatchObject([{ type: 'page.edit_body' }])
  expect((await getPage(page))!.blobs).toEqual([hash])
})

test('a file over the limit is refused with a clear message', async () => {
  const big = new File([new Uint8Array(25 * 1024 * 1024 + 1)], 'big.bin')
  await expect(addFile(big)).rejects.toThrow(/at most 25 MB/)
})

test('a Roadmap Phase holds items from any Area, moves them by reference and completes', async () => {
  await createArea('Work')
  await createArea('Home')
  const areas = await tree()
  const work = areas.find((x) => x.area.name === 'Work')!.area
  const home = areas.find((x) => x.area.name === 'Home')!.area
  const road = await createRoadmap(work.id, 'Plan')
  const one = await createPhase(road, 'One')
  const two = await createPhase(road, 'Two')
  const task = await createTask('Elsewhere', home.id)
  await placeItem(road, task, one)
  await placeItem(road, task, two)
  let phases = await roadmapSummary(road, '2026-01-01')
  expect(phases.map((p) => p.items.length)).toEqual([0, 1])
  expect(phases[0].current).toBe(true)
  expect((await tasksIn(home.id)).map((t) => t.task.id)).toContain(task)
  await completeTask(task)
  await setPhaseDone(two, true)
  phases = await roadmapSummary(road, '2026-01-01')
  expect(phases[1].progress).toBe(1)
  expect(phases[1].completed).toBe(true)
  await setPhaseDone(two, false)
  expect((await roadmapSummary(road, '2026-01-01'))[1].completed).toBe(false)
})

test('the rows hook reports a local write', async () => {
  const { onRowsChanged } = await import('./sync')
  const seen: string[] = []
  onRowsChanged((rows) => seen.push(...rows.map((r) => r.id)))
  const t = await createTask('hooked', INBOX)
  expect(seen).toContain(t)
  onRowsChanged(() => {})
})

test('an Area renames through area.update', async () => {
  await createArea('Before')
  const id = (await tree()).find((a) => a.area.name === 'Before')!.area.id
  await updateArea(id, { name: 'After' })
  expect((await tree()).find((a) => a.area.id === id)?.area.name).toBe('After')
})

test('Stardust counts a P1 Task at 80 and a Milestone at 250; un-completing a Milestone removes it', async () => {
  const { stardust, setMilestoneDone, createMilestone, setTargets } = await import('./sync')
  const base = (await stardust()).total
  const t = await createTask('stardust task', INBOX)
  await setPriority(t, 1)
  await completeTask(t)
  expect((await stardust()).total).toBe(base + 80)
  const area = (await tree())[0].area.id
  const g = await createGoal(area, 'stardust goal')
  const m = await createMilestone(g, 'stardust milestone')
  await setMilestoneDone(m, true)
  expect((await stardust()).total).toBe(base + 330)
  await setMilestoneDone(m, false)
  expect((await stardust()).total).toBe(base + 80)
  await setTargets({ target_daily: 7 })
  const s = await stardust()
  expect(s.targets.daily).toBe(7)
  expect(s.history).toHaveLength(30)
})

test('week start defaults to Monday and syncs as a setting', async () => {
  const { getWeekStart, setWeekStart } = await import('./sync')
  expect(await getWeekStart()).toBe(1)
  await setWeekStart(0)
  expect(await getWeekStart()).toBe(0)
})

test('the Calendar view is built in, and calendarData lists Routine days with check-in state', async () => {
  const { views, VIEW_CALENDAR, calendarData, createRoutine, checkIn, moveTask, getTask, localDay, tree, createArea } =
    await import('./sync')
  expect((await views()).find((v) => v.id === VIEW_CALENDAR)?.layout).toBe('calendar')
  await createArea('Calendar')
  const area = (await tree()).find((x) => x.area.name === 'Calendar')!.area
  const id = await createRoutine(area.id, {
    name: 'Stretch',
    schedule: { kind: 'daily' },
    length_days: null,
    time: null,
    goal_id: null,
  })
  const today = await localDay()
  await checkIn(id, today, 'done')
  const { routines } = await calendarData(today, today)
  const mine = routines.find((r) => r.routine.id === id)!
  expect(mine.days).toEqual([{ date: today, state: 'done' }])
  const t = await createTask('drag me', INBOX)
  await moveTask(t, '2026-01-05', '14:00')
  expect((await getTask(t))?.due).toEqual({ date: '2026-01-05', time: '14:00' })
})

test('the dashboard layout is the default until set, then synced as one field', async () => {
  const { getDashboard, setDashboard } = await import('./sync')
  const d = await getDashboard()
  expect(d.map((w) => w.kind)).toContain('glance')
  await setDashboard(d.map((w) => (w.kind === 'glance' ? { ...w, hidden: true } : w)))
  expect((await getDashboard()).find((w) => w.kind === 'glance')?.hidden).toBe(true)
})

test('turning vacation off keeps no range when it began today, and clears vacation_from', async () => {
  const { setVacation, localDay } = await import('./sync')
  const settings = () =>
    tx(['rows'], (s) => req<Record<string, unknown> | undefined>(s('rows').get('00000000-0000-7000-8000-000000000002')))
  await setVacation(true)
  let row = await settings()
  expect(row?.vacation).toBe(true)
  expect(row?.vacation_from).toBe(await localDay())
  await setVacation(false)
  row = await settings()
  expect(row?.vacation).toBe(false)
  expect(row?.vacation_from ?? null).toBeNull()
  expect(row?.vacations ?? []).toEqual([])
})

test('a Phase gated by an undone Milestone is locked and never current, and steps link', async () => {
  await createArea('Work')
  const work = (await tree()).find((x) => x.area.name === 'Work')!.area
  const road = await createRoadmap(work.id, 'Plan')
  const one = await createPhase(road, 'One')
  const two = await createPhase(road, 'Two')
  const goal = await createGoal(work.id, 'Ship')
  const ms = await createMilestone(goal, 'Beta')
  const a = await createTask('A', work.id)
  const b = await createTask('B', work.id)
  await placeItem(road, a, one)
  await placeItem(road, b, two)
  await setPhaseGate(one, ms)
  expect((await gateChoices()).map((m) => m.id)).toContain(ms)
  let phases = await roadmapSummary(road, '2026-01-01')
  expect(phases[0]).toMatchObject({ locked: true, current: false, gate_title: 'Beta' })
  expect(phases[0].items.length).toBe(1)
  expect(phases[1].current).toBe(true)
  await setMilestoneDone(ms, true)
  phases = await roadmapSummary(road, '2026-01-01')
  expect(phases[0]).toMatchObject({ locked: false, current: true })
  await setLinks(road, a, one, [b])
  phases = await roadmapSummary(road, '2026-01-01')
  expect(phases[0].items[0].links).toEqual([b])
  await setPhaseGate(one, null)
  expect((await roadmapSummary(road, '2026-01-01'))[0].gate_id).toBeNull()
})

test('a Mermaid gantt imports as Phases with placed items', async () => {
  const { importMermaid, parseMermaid } = await import('./sync')
  await createArea('Mermaid')
  const area = (await tree()).find((x) => x.area.name === 'Mermaid')!.area
  const road = await createRoadmap(area.id, 'Imported')
  const parsed = parseMermaid(
    'gantt\n  dateFormat YYYY-MM-DD\n  section Build\n  Design :a, 2026-03-01, 2026-03-05\n  Code :b, after a, 3d\n  section Ship\n  Release :c, 2026-03-10, 2026-03-12',
  )
  if (!('ok' in parsed)) throw new Error('parse failed')
  await importMermaid(road, parsed.ok)
  const phases = await roadmapSummary(road, '2026-01-01')
  expect(phases.map((p) => p.name)).toEqual(['Build', 'Ship'])
  expect(phases.map((p) => p.items.length)).toEqual([2, 1])
  expect(phases[0].items.map((i) => i.title)).toEqual(['Design', 'Code'])
  expect(phases[0].items[0].links).toHaveLength(1)
})

test('a Mermaid flowchart imports undated Tasks with links', async () => {
  const { importMermaid, parseMermaid } = await import('./sync')
  await createArea('Flow')
  const area = (await tree()).find((x) => x.area.name === 'Flow')!.area
  const road = await createRoadmap(area.id, 'Flow import')
  const parsed = parseMermaid('flowchart LR\n  subgraph Plan\n    A[Sketch] --> B[Build]\n  end')
  if (!('ok' in parsed)) throw new Error('parse failed')
  await importMermaid(road, parsed.ok)
  const [plan] = await roadmapSummary(road, '2026-01-01')
  expect(plan.items.map((i) => i.title)).toEqual(['Sketch', 'Build'])
  expect(plan.items[0].links).toEqual([plan.items[1].id])
  expect((await getTask(plan.items[0].id))?.due ?? null).toBeNull()
})
