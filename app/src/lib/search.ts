import { SearchIndex } from '../../wasm/orbit_wasm'
import { onRowsChanged, rowsSnapshot } from './sync'

export type Hit = { id: string; kind: string; title: string; score: number }
type Opens = Record<string, { count: number; last: number }>

const KEY = 'search.opens'

function load(): Opens {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}')
  } catch {
    // Storage blocked or damaged: start with no history.
    return {}
  }
}
function save(opens: Opens) {
  try {
    localStorage.setItem(KEY, JSON.stringify(opens))
  } catch {
    // Not remembered; still counted for this visit.
  }
}

/** One search index, with frecency kept per Device in localStorage. Rows go in through `upsert`/`remove`. */
export function createSearch(persist = true) {
  const index = new SearchIndex()
  const opens: Opens = persist ? load() : {}
  for (const [id, o] of Object.entries(opens)) index.set_opens(id, o.count, o.last)
  return {
    upsert: (row: { id: string }) => index.upsert(JSON.stringify(row)),
    remove: (id: string) => index.remove(id),
    query: (text: string, limit = 30, now = Date.now()): Hit[] => JSON.parse(index.query(text, now, limit)),
    recordOpen(id: string, now = Date.now()) {
      index.record_open(id, now)
      const o = opens[id] ?? { count: 0, last: 0 }
      opens[id] = { count: o.count + 1, last: now }
      if (persist) save(opens)
    },
  }
}

let shared: ReturnType<typeof createSearch> | undefined
let loading: Promise<void> | undefined

/** The one app-wide index: loaded from the local rows once, then fed by every local write and sync delta. */
export function search() {
  if (!shared) {
    const s = (shared = createSearch())
    const ignore = () => {}
    onRowsChanged((rows, removed) => {
      for (const r of rows) s.upsert(r)
      for (const id of removed) s.remove(id)
    })
    loading = rowsSnapshot().then((rows) => {
      for (const r of rows) s.upsert(r)
    }, ignore)
  }
  return shared
}
/** Resolves once the first load of the local rows is in the index. */
export const searchReady = async () => {
  search()
  await loading
}

/** Display names of the kinds, in the order results are grouped. */
export const KIND_ORDER = ['task', 'project', 'area', 'goal', 'milestone', 'page', 'routine', 'view', 'section']
export const KIND_LABEL: Record<string, string> = {
  task: 'Tasks',
  project: 'Projects',
  area: 'Areas',
  goal: 'Goals',
  milestone: 'Milestones',
  page: 'Pages',
  routine: 'Routines',
  view: 'Views',
  section: 'Sections',
}

/** Results grouped by kind, each group in score order; groups in `KIND_ORDER`. */
export function groupHits(hits: Hit[]): { kind: string; hits: Hit[] }[] {
  const order = (k: string) => (KIND_ORDER.includes(k) ? KIND_ORDER.indexOf(k) : KIND_ORDER.length)
  const kinds = [...new Set(hits.map((h) => h.kind))].sort((a, b) => order(a) - order(b))
  return kinds.map((kind) => ({ kind, hits: hits.filter((h) => h.kind === kind) }))
}
