import { apply, inbox_id, new_id, next_hlc } from '../../wasm/orbit_wasm'
import { req, tx } from './db'

export type Task = { kind: 'task'; id: string; area_id: string; title: string; completed_at: number | null }
type Row = Task | { kind: 'area'; id: string; name: string }
type Hlc = { wall: number; counter: number; device: string }
type Command = { uuid: string; type: string; args: { id: string; title?: string }; hlc: Hlc }

const newId = () => new_id(Date.now(), crypto.getRandomValues(new Uint8Array(10)))
const byHlc = (a: Command, b: Command) => a.hlc.wall - b.hlc.wall || a.hlc.counter - b.hlc.counter
let changed = () => {}

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
    // shortcut: random per-browser Device id until login (#34) issues one.
    const prev: Hlc = (await req(s('meta').get('hlc'))) ?? { wall: 0, counter: 0, device: newId() }
    const hlc: Hlc = JSON.parse(next_hlc(JSON.stringify(prev), Date.now()))
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
    if (!res.ok) throw new Error(`sync: HTTP ${res.status}`)
    const body: { statuses: Record<string, unknown>; delta: Row[]; token: number } = await res.json()
    await tx(['rows', 'outbox', 'meta'], async (s) => {
      // shortcut: a rejected command is just dropped; restoring its row and a toast come with #35.
      for (const uuid of Object.keys(body.statuses)) s('outbox').delete(uuid)
      for (const row of body.delta) s('rows').put(row)
      s('meta').put(body.token, 'token')
      // Re-predict anything queued meanwhile on top of the server's rows.
      for (const cmd of (await req<Command[]>(s('outbox').getAll())).sort(byHlc)) {
        await predict(s('rows'), cmd).catch(() => {})
      }
    })
    changed()
    if (commands.length < 100) return
  }
}

/** Syncs on open, on focus, 2 s after writes and every 60 s while visible. */
export function start(onChange: () => void) {
  changed = onChange
  sync()
  addEventListener('focus', () => sync())
  setInterval(() => document.visibilityState === 'visible' && sync(), 60_000)
}
