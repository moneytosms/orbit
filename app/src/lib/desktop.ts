// Linux desktop shell glue (Tauri): the tray/`--capture` events, and handing the Rust side a
// native Device token so its background replica can fire reminders. No-op in a browser.
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { ui } from './actions'
import { createToken } from './auth'
import { sync } from './sync'

export const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

let started = false

/** The web Device's token is an HttpOnly cookie, so Rust gets its own Device via `POST /devices`. */
async function provisionToken() {
  try {
    if (await invoke<boolean>('has_token')) return
    const { device, token } = await createToken('Orbit desktop (reminders)')
    await invoke('set_token', { server: location.origin, device, token })
  } catch {
    // Signed out or offline: retried at the next start or window focus.
  }
}

export function startDesktop() {
  if (started || !inTauri()) return
  started = true
  void listen('orbit://capture', () => ui.newTask())
  void listen('orbit://sync', () => void sync())
  void provisionToken()
  window.addEventListener('focus', () => void provisionToken())
}
