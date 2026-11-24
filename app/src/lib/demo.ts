// Web demo (build with VITE_DEMO=1): a fake server inside the page, so the real app runs on the website
// with no backend. The commands apply locally as usual; the "server" only acknowledges them. Data stays in
// this browser's IndexedDB.
import { login, hasDevice } from './auth'
import {
  addEntry,
  createArea,
  createGoal,
  createProject,
  createRoutine,
  createTask,
  setDue,
  tree,
  updateTask,
} from './sync'

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } })

function fakeServer() {
  const real = window.fetch.bind(window)
  let token = 0
  window.fetch = async (input, init) => {
    const url = new URL(input instanceof Request ? input.url : String(input), location.href)
    if (url.origin !== location.origin) return real(input, init)
    switch (url.pathname) {
      case '/auth':
        return json({ setup: false })
      case '/auth/login':
        return json({ device: crypto.randomUUID() })
      case '/sync': {
        const { commands } = JSON.parse(String(init?.body))
        const statuses = Object.fromEntries(commands.map((c: { uuid: string }) => [c.uuid, { status: 'ok' }]))
        return json({ statuses, delta: [], token: ++token, clock: null })
      }
      case '/devices':
        return json([])
      case '/version':
      case '/disk':
      case '/export':
        return json({ error: 'demo' }, 404)
      default:
        return real(input, init)
    }
  }
}

const day = (offset: number) => {
  const d = new Date()
  d.setDate(d.getDate() + offset)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

async function seed() {
  await createArea('Home')
  await createArea('Career')
  const areas = await tree()
  const home = areas.find((a) => a.area.name === 'Home')!.area.id
  const career = areas.find((a) => a.area.name === 'Career')!.area.id
  await createProject(home, 'Spring cleaning')
  const project = (await tree()).find((a) => a.area.id === home)!.projects[0].id
  const t = async (title: string, parent: string | undefined, due: number | null, priority?: 1 | 2 | 3 | 4) => {
    const id = await createTask(title, parent)
    if (due !== null) await setDue(id, day(due))
    if (priority) await updateTask(id, { priority })
  }
  await t('Try the quick add: press q', undefined, 0)
  await t('Sort the garage', project, 1)
  await t('Donate old clothes', project, 3)
  await t('Pay the electricity bill', home, -1, 1)
  await t('Update the portfolio', career, 2, 2)
  await t('Book the dentist', home, 5)
  const goal = await createGoal(career, 'Read 5 books', { progress: { kind: 'target', target: 5, entries: [] } })
  await addEntry(goal, 2, 'Two down')
  await createRoutine(home, {
    name: 'Morning walk',
    icon: '🚶',
    schedule: { kind: 'daily' },
    length_days: null,
    time: '07:30',
    goal_id: null,
  })
}

function banner() {
  const bar = document.createElement('div')
  bar.textContent = 'Demo: nothing leaves your browser. Clear site data to reset.'
  bar.style.cssText =
    'position:fixed;bottom:0;left:0;right:0;z-index:9999;padding:4px 8px;text-align:center;font:12px system-ui;background:#4F7F5B;color:#fff;pointer-events:none'
  document.body.append(bar)
}

export async function startDemo() {
  fakeServer()
  if (!(await hasDevice())) await login('demo')
  if (!localStorage.getItem('orbit-demo-seeded')) {
    localStorage.setItem('orbit-demo-seeded', '1')
    await seed()
  }
  banner()
}
