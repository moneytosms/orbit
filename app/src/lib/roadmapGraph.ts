import type { GraphEdge, GraphNode, GraphNodeKind, Phase } from './sync'

/** Nodes hidden because they sit on an untaken branch of a decided decision. They are kept, only not shown. */
export function hiddenNodeIds(nodes: GraphNode[], edges: GraphEdge[]): Set<string> {
  const hidden = new Set<string>()
  for (const decision of nodes.filter((n) => n.node_kind === 'decision' && n.decided_at && n.taken_edge_id)) {
    const walk = (id: string, visited: Set<string>) => {
      if (visited.has(id) || id === decision.id) return
      visited.add(id)
      hidden.add(id)
      for (const edge of edges) if (edge.source_id === id) walk(edge.target_id, visited)
    }
    for (const edge of edges)
      if (edge.source_id === decision.id && edge.id !== decision.taken_edge_id) walk(edge.target_id, new Set())
  }
  return hidden
}

export type BandGroup = { band: Phase | null; nodes: GraphNode[] }

/** Nodes grouped by band in band order, then a final `band: null` group for nodes with no (or a missing) band. */
export function groupByBand(nodes: GraphNode[], bands: Phase[]): BandGroup[] {
  const byOrder = (a: Phase, b: Phase) => (a.order < b.order ? -1 : a.order > b.order ? 1 : a.id < b.id ? -1 : 1)
  const known = new Set(bands.map((b) => b.id))
  const groups: BandGroup[] = [...bands].sort(byOrder).map((band) => ({
    band,
    nodes: nodes.filter((n) => n.band_id === band.id),
  }))
  groups.push({ band: null, nodes: nodes.filter((n) => !n.band_id || !known.has(n.band_id)) })
  return groups
}

/** A layered layout: rank = longest path from a source, columns 320 apart, rows 120 apart, title order inside a rank. */
export function tidyLayout(nodes: GraphNode[], edges: GraphEdge[]): Map<string, { x: number; y: number }> {
  const incoming = new Map(nodes.map((n) => [n.id, 0]))
  const next = new Map<string, string[]>()
  for (const e of edges) {
    incoming.set(e.target_id, (incoming.get(e.target_id) ?? 0) + 1)
    next.set(e.source_id, [...(next.get(e.source_id) ?? []), e.target_id])
  }
  const rank = new Map<string, number>()
  const queue = nodes.filter((n) => !incoming.get(n.id)).map((n) => n.id)
  for (const n of queue) rank.set(n, 0)
  for (let i = 0; i < queue.length; i++)
    for (const to of next.get(queue[i]) ?? []) {
      const r = (rank.get(queue[i]) ?? 0) + 1
      if (r > (rank.get(to) ?? -1)) rank.set(to, r)
      incoming.set(to, (incoming.get(to) ?? 1) - 1)
      if (!incoming.get(to)) queue.push(to)
    }
  const ordered = [...nodes].sort(
    (a, b) => (rank.get(a.id) ?? 0) - (rank.get(b.id) ?? 0) || a.title.localeCompare(b.title),
  )
  const slots = new Map<number, number>()
  const out = new Map<string, { x: number; y: number }>()
  for (const n of ordered) {
    const r = rank.get(n.id) ?? 0
    const slot = slots.get(r) ?? 0
    slots.set(r, slot + 1)
    out.set(n.id, { x: 80 + r * 320, y: 80 + slot * 120 })
  }
  return out
}

export const NODE_W = 240
export const NODE_H = 64
export type ConvertTarget = { kind: GraphNodeKind; ok: boolean; reason?: string }
const CONVERTIBLE: GraphNodeKind[] = ['task', 'decision', 'note', 'heading']
const REF_KINDS: GraphNodeKind[] = ['task', 'project', 'goal', 'milestone']

/** Kinds a node can become. Linked nodes (task, project, goal, milestone) keep their item when they turn into a plain node. */
export function convertTargets(node: Pick<GraphNode, 'node_kind'>): ConvertTarget[] {
  const linked = REF_KINDS.includes(node.node_kind)
  return CONVERTIBLE.filter((kind) => kind !== node.node_kind).map((kind) =>
    kind === 'task' && linked
      ? { kind, ok: false, reason: `A ${node.node_kind} is its own item and cannot become a task.` }
      : { kind, ok: true },
  )
}

/** A free spot for a new node: right of `anchor`, else at `at`, moved down until it overlaps nothing. */
export function placeNew(
  nodes: Pick<GraphNode, 'x' | 'y'>[],
  anchor: Pick<GraphNode, 'x' | 'y'> | undefined,
  at: { x: number; y: number },
): { x: number; y: number } {
  const p = anchor ? { x: anchor.x + NODE_W + 80, y: anchor.y } : { ...at }
  const taken = (y: number) => nodes.some((n) => Math.abs(n.x - p.x) < NODE_W && Math.abs(n.y - y) < NODE_H + 16)
  while (taken(p.y)) p.y += NODE_H + 32
  return p
}
