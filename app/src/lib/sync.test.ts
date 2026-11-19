import 'fake-indexeddb/auto'
import { readFileSync } from 'node:fs'
import { beforeAll, beforeEach, expect, test, vi } from 'vitest'
import { initSync, parse_quick_add } from '../../wasm/orbit_wasm'
import { login } from './auth'
import { req, tx } from './db'
import {
  completeTask,
  createArea,
  createGoal,
  createLabel,
  createMilestone,
  createPage,
  createProject,
  createTask,
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
  goalProgress,
  labels,
  moveItem,
  moveTo,
  onRejected,
  onSignedOut,
  orderBetween,
  pagesOf,
  quickAdd,
  renamePage,
  setMilestoneDone,
  sync,
  tasksIn,
  tree,
  updateSettings,
  updateTask,
  type Task,
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
  const inFlight = sync()
  await vi.waitFor(() => expect(requests.length).toBeGreaterThan(0))
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
