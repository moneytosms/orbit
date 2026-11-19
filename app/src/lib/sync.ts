import { apply, inbox_id, new_id, next_hlc, observe_hlc } from '../../wasm/orbit_wasm'
import { req, tx } from './db'

export type Task = { kind: 'task'; id: string; area_id: string; title: string; completed_at: number | null }
type Row = Task | { kind: 'area'; id: string; name: string }
type Hlc = { wall: number; counter: number; device: string }
type Command = { uuid: string; type: string; args: { id: string; title?: string }; hlc: Hlc }

const newId = () => new_id(Date.now(), crypto.getRandomValues(new Uint8Array(10)))
const byHlc = (a: Command, b: Command) => a.hlc.wall - b.hlc.wall || a.hlc.counter - b.hlc.counter
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

export async function inboxTasks(): Promise<Task[]> {
  const rows: Row[] = await tx(['rows'], (s) => req(s('rows').getAll()))
  const inbox = inbox_id()
  // Ids are UUIDv7, so key order is creation order.
  return rows.filter((r): r is Task => r.kind === 'task' && r.area_id === inbox && r.completed_at === null)
}

export const createTask = (title: string) => enqueue('task.create', { id: newId(), title })
export const completeTask = (id: string) => enqueue('task.complete', { id })

/** Applies a command to its row via core; throws the error code if core rejects it. */
async function predict(rows: IDBObjectStore, cmd: Command) {
  const existing = await req(rows.get(cmd.args.id))
  rows.put(JSON.parse(apply(existing ? JSON.stringify(existing) : undefined, JSON.stringify(cmd))))
}

async function enqueue(type: string, args: Command['args']) {
  await tx(['rows', 'outbox', 'meta'], async (s) => {
    const hlc: Hlc = JSON.parse(next_hlc(JSON.stringify(await clock(s('meta'))), Date.now()))
    const cmd = { uuid: newId(), type, args, hlc }
    await predict(s('rows'), cmd)
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
    const body: { statuses: Record<string, Status>; delta: Row[]; token: number; clock: Hlc | null } =
      await res.json()
    const codes: string[] = []
    let answered = 0
    await tx(['rows', 'outbox', 'meta'], async (s) => {
      for (const cmd of commands) {
        const status = body.statuses[cmd.uuid]
        if (!status) continue // not answered: keep it queued
        answered++
        if (status.status === 'error') {
          // Undo the guess; the delta brings back the server's row if it has one.
          s('rows').delete(cmd.args.id)
          codes.push(status.code)
        }
        s('outbox').delete(cmd.uuid)
      }
      for (const row of body.delta) s('rows').put(row)
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
