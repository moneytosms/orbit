import type { GraphEdge, GraphNode, Phase } from './sync'

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
