// Linux desktop shell glue (Tauri): the tray/`--capture` events, and handing the Rust side a
// native Device token so its background replica can fire reminders. No-op in a browser.
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { ui } from './actions'
import { createToken } from './auth'
import { sync } from './sync'
import { inTauri } from './platform'

export { inTauri } from './platform'

let started = false

/** The web Device's token is an HttpOnly cookie, so Rust gets its own Device via `POST /devices`. */
async function provisionToken() {
  try {
    if (await invoke<boolean>('has_token')) return
    const { device, token } = await createToken(
      navigator.userAgent.includes('Android') ? 'Orbit Android (background sync)' : 'Orbit desktop (reminders)',
    )
    await invoke('set_token', { server: location.origin, device, token })
  } catch {
    // Signed out or offline: retried at the next start or window focus.
  }
}

/** A widget link: `#/new?q=<text>` opens the capture window, anything else is a hash route. */
function openLink(link: string) {
  if (link.startsWith('#/new')) {
    const q = new URLSearchParams(link.slice(link.indexOf('?') + 1)).get('q') ?? ''
    return ui.capture(link.includes('?') ? q : '')
  }
  location.hash = link
}

/** Android: a widget tap leaves a link (`#/task/<id>`) for the app to open once the window is up. */
async function takeLink(retries = 0) {
  try {
    const link = await invoke<string>('take_link')
    if (link) return openLink(link)
  } catch {
    return // Not Android, or not signed in yet.
  }
  // The window can gain focus a moment before the activity hands over the new intent.
  if (retries < 3) setTimeout(() => void takeLink(retries + 1), 400)
}

export function startDesktop() {
  if (started || !inTauri()) return
  started = true
  void listen('orbit://capture', () => ui.newTask())
  void listen('orbit://sync', () => void sync())
  window.addEventListener('orbit-link', (e) => openLink(String((e as CustomEvent).detail)))
  void provisionToken()
  const android = navigator.userAgent.includes('Android')
  if (android) void takeLink()
  window.addEventListener('focus', () => {
    void provisionToken()
    if (android) void takeLink()
  })
}
