import { req, tx } from './db'

export type UpgradeRequired = {
  error: 'upgrade_required'
  client_protocol: number
  min_client_protocol: number
  upgrade: string
}
export type ServerVersion = {
  server: string
  protocol: number
  min_client_protocol: number
  min_shell_version: string
  built: string
  ui: string
}
export type UpdateState = { upgrade: UpgradeRequired | null; version: ServerVersion | null }

const KEY = 'updates'
export const CLIENT_PROTOCOL = 1
const UPGRADE_URL = 'https://github.com/moneytosms/orbit/releases/latest'
let state: UpdateState = { upgrade: null, version: null }
const listeners = new Set<(value: UpdateState) => void>()
const publish = (value: UpdateState) => {
  state = value
  listeners.forEach((fn) => fn(state))
}

export const currentUpdates = () => state
export const onUpdates = (fn: (value: UpdateState) => void) => {
  listeners.add(fn)
  fn(state)
  return () => listeners.delete(fn)
}

export async function loadUpdates() {
  state = (await tx(['meta'], (s) => req<UpdateState | undefined>(s('meta').get(KEY)))) ?? state
  publish(state)
  return state
}

export async function setUpgrade(upgrade: UpgradeRequired) {
  await loadUpdates()
  const next = { ...state, upgrade }
  await tx(['meta'], async (s) => void s('meta').put(next, KEY))
  publish(next)
}

export async function clearUpgrade() {
  await loadUpdates()
  if (!state.upgrade) return
  const next = { ...state, upgrade: null }
  await tx(['meta'], async (s) => void s('meta').put(next, KEY))
  publish(next)
}

export async function checkServerVersion(): Promise<UpdateState> {
  await loadUpdates()
  const res = await fetch('/version', { cache: 'no-store' })
  if (!res.ok) throw new Error(`version: HTTP ${res.status}`)
  const version = (await res.json()) as ServerVersion
  const next = { ...state, version }
  await tx(['meta'], async (s) => void s('meta').put(next, KEY))
  publish(next)
  if (version.min_client_protocol > CLIENT_PROTOCOL) {
    await setUpgrade({
      error: 'upgrade_required',
      client_protocol: CLIENT_PROTOCOL,
      min_client_protocol: version.min_client_protocol,
      upgrade: UPGRADE_URL,
    })
  } else if (state.upgrade && state.upgrade.min_client_protocol <= CLIENT_PROTOCOL) {
    await clearUpgrade()
  }
  return next
}

export const uiHash = __ORBIT_UI_HASH__
