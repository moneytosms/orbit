import {
  checkIn,
  completeTask,
  getDaySettings,
  localDay,
  localMinute,
  reminderSchedule,
  snoozeTask,
  spendReminder,
  type Fire,
} from './sync'
import { inTauri, startDesktop } from './desktop'

const KEY = 'orbit.notify'
const shown = new Set<string>()
let timer: ReturnType<typeof setTimeout> | undefined
let started = false

/** Whether this device shows reminder notifications (a per-device setting). */
export function notifyEnabled(): boolean {
  try {
    return (
      localStorage.getItem(KEY) === '1' && typeof Notification !== 'undefined' && Notification.permission === 'granted'
    )
  } catch {
    return false
  }
}

/** Turns notifications on or off here; turning on asks for permission. Returns the resulting state. */
export async function setNotifyEnabled(on: boolean): Promise<boolean> {
  if (on) {
    if (typeof Notification === 'undefined') return false
    if (Notification.permission !== 'granted') on = (await Notification.requestPermission()) === 'granted'
  }
  try {
    localStorage.setItem(KEY, on ? '1' : '0')
  } catch {
    // Storage blocked: the toggle just won't stick.
  }
  void refreshNotifier()
  return on
}

const fireKey = (f: Fire) => `${f.task_id ?? f.routine_id}|${f.reminder_id ?? ''}|${f.at}`
/** Local `YYYY-MM-DDTHH:MM` as epoch ms in this device's zone, close enough to schedule a timer. */
const localMs = (at: string) => new Date(at).getTime()

/** Tomorrow at Day start, as `YYYY-MM-DDTHH:MM`. */
async function tomorrowAtDayStart(): Promise<string> {
  const now = await localMinute()
  const d = new Date(`${now.slice(0, 10)}T00:00:00Z`)
  d.setUTCDate(d.getUTCDate() + 1)
  return `${d.toISOString().slice(0, 10)}T${(await getDaySettings()).day_start}`
}
const inMinutes = async (m: number) => {
  const now = new Date(`${await localMinute()}:00Z`)
  return new Date(now.getTime() + m * 60_000).toISOString().slice(0, 16)
}

/** Snooze presets offered for a Task, in menu order. */
export const SNOOZE_PRESETS = ['10m', '30m', '1h', '3h', 'tomorrow9', 'monday9'] as const
export type SnoozePreset = (typeof SNOOZE_PRESETS)[number]
/** Menu wording for each preset. */
export const SNOOZE_LABELS: Record<SnoozePreset, string> = {
  '10m': '10 min',
  '30m': '30 min',
  '1h': '1 hour',
  '3h': '3 hours',
  tomorrow9: 'Tomorrow 9:00',
  monday9: 'Monday 9:00',
}

/**
 * When a snooze preset lands, as local `YYYY-MM-DDTHH:MM`, given `now` in the same form.
 * Pure calendar arithmetic on the floating local time, so no zone or DST handling is needed.
 * `monday9` is the next Monday after today (a Monday now gives the one a week on).
 */
export function snoozeUntil(preset: SnoozePreset, now: string): string {
  const day = now.slice(0, 10)
  const addDays = (n: number) => {
    const d = new Date(`${day}T00:00:00Z`)
    d.setUTCDate(d.getUTCDate() + n)
    return d.toISOString().slice(0, 10)
  }
  const addMinutes = (m: number) => {
    const d = new Date(`${now.slice(0, 16)}:00Z`)
    return new Date(d.getTime() + m * 60_000).toISOString().slice(0, 16)
  }
  switch (preset) {
    case '10m':
      return addMinutes(10)
    case '30m':
      return addMinutes(30)
    case '1h':
      return addMinutes(60)
    case '3h':
      return addMinutes(180)
    case 'tomorrow9':
      return `${addDays(1)}T09:00`
    case 'monday9': {
      const dow = new Date(`${day}T00:00:00Z`).getUTCDay() // 0 = Sunday
      const toMonday = (1 - dow + 7) % 7 || 7
      return `${addDays(toMonday)}T09:00`
    }
  }
}

/** Snoozes a Task by a preset, measured from the device's local now. */
export async function snoozeTaskPreset(taskId: string, preset: SnoozePreset) {
  await snoozeTask(taskId, snoozeUntil(preset, await localMinute()))
  void refreshNotifier()
}

/** Runs a notification action (also used for clicks on the notification body). */
export async function handleAction(action: string, taskId?: string, routineId?: string) {
  if (action === 'checkin' && routineId) await checkIn(routineId, await localDay(), 'done')
  else if (action === 'complete' && taskId) await completeTask(taskId)
  else if (action === 'snooze10' && taskId) await snoozeTask(taskId, await inMinutes(10))
  else if (action === 'snooze60' && taskId) await snoozeTask(taskId, await inMinutes(60))
  else if (action === 'tomorrow' && taskId) await snoozeTask(taskId, await tomorrowAtDayStart())
  else if (taskId) location.hash = `#/inbox`
  void refreshNotifier()
}

async function show(f: Fire) {
  const options: NotificationOptions & { actions?: { action: string; title: string }[] } = {
    body: f.late ? `Was due ${f.at.replace('T', ' ')}` : f.at.slice(11),
    tag: fireKey(f),
    data: { task_id: f.task_id, routine_id: f.routine_id },
    ...(f.task_id
      ? {
          actions: [
            { action: 'complete', title: 'Complete' },
            { action: 'snooze10', title: 'Snooze 10 min' },
            { action: 'snooze60', title: 'Snooze 1 h' },
            { action: 'tomorrow', title: 'Tomorrow' },
          ],
        }
      : f.routine_id
        ? // No Routine snooze exists, so only Done.
          { actions: [{ action: 'checkin', title: 'Done' }] }
        : {}),
  }
  const reg = 'serviceWorker' in navigator ? await navigator.serviceWorker.getRegistration() : undefined
  if (reg && (f.task_id || f.routine_id)) await reg.showNotification(f.title, options)
  else {
    const n = new Notification(f.title, options)
    n.onclick = () => {
      n.close()
      void handleAction('open', f.task_id ?? undefined)
    }
  }
}

/** Shows every reminder that is due and schedules a timer for the next one. */
export async function refreshNotifier() {
  clearTimeout(timer)
  // The desktop app's Rust side fires reminders; the web path would duplicate them.
  if (inTauri() || !notifyEnabled()) return
  const now = await localMinute()
  const fires = await reminderSchedule()
  let next: Fire | undefined
  for (const f of fires) {
    if (f.at > now) {
      next ??= f
      continue
    }
    const k = fireKey(f)
    if (shown.has(k)) continue
    shown.add(k)
    await show(f)
    // Absolute and snoozed reminders fire once; offsets re-apply per occurrence.
    if (f.task_id && f.reminder_id) await spendReminder(f.task_id, f.reminder_id).catch(() => {})
  }
  if (next) {
    const wait = Math.min(Math.max(localMs(next.at) - Date.now(), 1000), 30_000)
    timer = setTimeout(() => void refreshNotifier(), wait)
  } else timer = setTimeout(() => void refreshNotifier(), 30_000)
}

/** Starts the notifier: action clicks from the service worker, and the periodic check. */
export function startNotifier() {
  if (started) return
  started = true
  startDesktop()
  navigator.serviceWorker?.addEventListener('message', (e) => {
    if (e.data?.type === 'orbit-notify') void handleAction(e.data.action, e.data.task_id, e.data.routine_id)
  })
  const q = new URLSearchParams(location.search)
  if (q.has('notify')) {
    void handleAction(q.get('notify')!, q.get('task') ?? undefined, q.get('routine') ?? undefined)
    history.replaceState(null, '', location.pathname + location.hash)
  }
  void refreshNotifier()
}
