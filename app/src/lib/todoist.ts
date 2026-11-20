// Todoist import: the server reads the account with a pasted token (used once, never stored),
// shows a preview of where each project goes, then commits it as normal commands.
import { api } from './auth'
import { sync } from './sync'

/** Where one Todoist project goes. `area` names the Area (or the Project's Area), `name` the Project. */
export type Plan = {
  id: string
  kind: 'area' | 'project' | 'inbox'
  area: string
  name: string
  path: string
  tasks: number
  sections: number
  imported: number
}
export type Preview = { projects: Plan[]; totals: Record<string, number> }
export type Report = {
  areas: number
  projects: number
  sections: number
  tasks: number
  labels: number
  pages: number
  reminders: number
  existing_tasks: number
  issues: string[]
  dropped: string[]
}

/** Sends the token once and waits for the preview; throws the server's failure message. */
export async function startImport(token: string, every = 500): Promise<{ job: string; preview: Preview }> {
  const { job } = await api<{ job: string }>('POST', '/import/todoist', { token })
  for (;;) {
    const p = await api<{ status: string; error?: string } & Partial<Preview>>('GET', `/import/todoist/${job}`)
    if (p.status === 'ready') return { job, preview: p as Preview }
    if (p.status === 'failed') throw new Error(p.error)
    await new Promise((r) => setTimeout(r, every))
  }
}

/** Commits the (edited) mapping, then syncs so this Device sees the new items. */
export async function commitImport(job: string, projects: Plan[]): Promise<Report> {
  const report = await api<Report>('POST', `/import/todoist/${job}/commit`, { projects })
  await sync().catch(() => {})
  return report
}
