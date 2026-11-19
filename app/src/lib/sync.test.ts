import 'fake-indexeddb/auto'
import { readFileSync } from 'node:fs'
import { beforeAll, beforeEach, expect, test, vi } from 'vitest'
import { initSync } from '../../wasm/orbit_wasm'
import { login } from './auth'
import { completeTask, createTask, inboxTasks, onRejected, onSignedOut, sync } from './sync'

const INBOX = '00000000-0000-7000-8000-000000000001'
type Body = { v: number; token?: number; commands: { uuid: string; type: string; args: any }[] }

let online: boolean
let requests: Body[]
let delta: object[]
let token = 0
let gate: Promise<void> | undefined
let httpStatus = 200
let rejectWith: string | undefined

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
  const res = { statuses, delta, token: ++token }
  delta = []
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
