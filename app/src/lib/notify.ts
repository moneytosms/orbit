import {
  completeTask,
  getDaySettings,
  localMinute,
  reminderSchedule,
  snoozeTask,
  spendReminder,
  type Fire,
} from './sync'

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

/** Runs a notification action (also used for clicks on the notification body). */
export async function handleAction(action: string, taskId?: string) {
  if (action === 'complete' && taskId) await completeTask(taskId)
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
    data: { task_id: f.task_id },
    ...(f.task_id
      ? {
          actions: [
            { action: 'complete', title: 'Complete' },
            { action: 'snooze10', title: 'Snooze 10 min' },
            { action: 'snooze60', title: 'Snooze 1 h' },
            { action: 'tomorrow', title: 'Tomorrow' },
          ],
        }
      : {}),
  }
  const reg = 'serviceWorker' in navigator ? await navigator.serviceWorker.getRegistration() : undefined
  if (reg && f.task_id) await reg.showNotification(f.title, options)
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
  if (!notifyEnabled()) return
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
  navigator.serviceWorker?.addEventListener('message', (e) => {
    if (e.data?.type === 'orbit-notify') void handleAction(e.data.action, e.data.task_id)
  })
  const q = new URLSearchParams(location.search)
  if (q.has('notify')) {
    void handleAction(q.get('notify')!, q.get('task') ?? undefined)
    history.replaceState(null, '', location.pathname + location.hash)
  }
  void refreshNotifier()
}
