// Login, first-run setup and Device management. The web Device's token is an HttpOnly cookie,
// so this side only keeps the Device id (its HLC device id).
import { req, tx } from './db'
import { sync } from './sync'

export type Device = {
  id: string
  name: string
  platform: string
  created: number
  last_seen: number
  current: boolean
}

/** Calls the server; throws its error code (e.g. `totp`, `credentials`, `rate_limited`) or the HTTP status. */
export async function api<T = void>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body === undefined ? {} : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (!res.ok) throw new Error((await res.json().catch(() => null))?.error ?? `HTTP ${res.status}`)
  return res.status === 204 ? (undefined as T) : res.json()
}

const find = (names: string[]) => names.find((n) => navigator.userAgent.includes(n))
const deviceName = () =>
  `${(find(['Firefox', 'Edg', 'Chrome', 'Safari']) ?? 'Browser').replace('Edg', 'Edge')} on ${find(['Android', 'iPhone', 'Windows', 'Mac', 'Linux']) ?? 'the web'}`

/** Signs in as a new Device, adopts its id for the HLC and sends whatever the outbox holds. */
async function signIn(path: string, body: object) {
  const { device } = await api<{ device: string }>('POST', path, { ...body, name: deviceName(), platform: 'web' })
  await tx(['meta'], async (s) => {
    const hlc = await req(s('meta').get('hlc'))
    s('meta').put(hlc ? { ...hlc, device } : { wall: 0, counter: 0, device }, 'hlc')
    s('meta').put(device, 'device')
  })
  await sync()
}

export const login = (password: string, totp?: string) => signIn('/auth/login', { password, totp: totp || null })
export const setup = (code: string, password: string) => signIn('/auth/setup', { code, password })
/** Whether this browser ever logged in; a revoked one finds out on its next sync. */
export const hasDevice = () => tx(['meta'], (s) => req(s('meta').get('device'))).then(Boolean)
export const deviceId = () => tx(['meta'], (s) => req<string | undefined>(s('meta').get('device')))
export const needsSetup = () => api<{ setup: boolean }>('GET', '/auth').then((r) => r.setup)

export const devices = () => api<Device[]>('GET', '/devices')
/** When the server last took a backup (ms since epoch), or null if never. */
export const backupStatus = () =>
  api<{ last_backup_ms: number | null }>('GET', '/backup/status').then((r) => r.last_backup_ms)
export const revoke = (id: string) => api('DELETE', `/devices/${id}`)
export const createToken = (name: string) => api<{ device: string; token: string }>('POST', '/devices', { name })
export const changePassword = (current: string, next: string, signOutOthers: boolean) =>
  api('POST', '/auth/password', { current, new: next, sign_out_others: signOutOthers })
export const startTotp = (password: string) => api<{ secret: string; url: string }>('POST', '/auth/totp', { password })
export const confirmTotp = (code: string) => api('POST', '/auth/totp/confirm', { code })
