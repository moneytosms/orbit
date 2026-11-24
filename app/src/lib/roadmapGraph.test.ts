import { expect, test } from 'vitest'
import { groupByBand, hiddenNodeIds, tidyLayout } from './roadmapGraph'
import type { GraphEdge, GraphNode, Phase } from './sync'

const node = (o: Partial<GraphNode> & { id: string }): GraphNode => ({
  kind: 'graph_node',
  roadmap_id: 'r',
  node_kind: 'task',
  ref_id: null,
  title: o.id,
  x: 0,
  y: 0,
  band_id: null,
  taken_edge_id: null,
  decided_at: null,
  trashed_at: null,
  ...o,
})
const edge = (id: string, source_id: string, target_id: string): GraphEdge => ({
  kind: 'graph_edge',
  id,
  roadmap_id: 'r',
  source_id,
  target_id,
  edge_kind: 'then',
  label: '',
  trashed_at: null,
})
const band = (id: string, order: string): Phase => ({
  kind: 'phase',
  id,
  roadmap_id: 'r',
  name: id.toUpperCase(),
  order,
  start: null,
  end: null,
  gate_id: null,
  completed_at: null,
  archived_at: null,
  trashed_at: null,
})

test('a decided decision hides only the untaken branch', () => {
  const nodes = [
    node({ id: 'd', node_kind: 'decision', decided_at: 1, taken_edge_id: 'e1' }),
    node({ id: 'yes' }),
    node({ id: 'no' }),
    node({ id: 'after-no' }),
    node({ id: 'after-yes' }),
  ]
  const edges = [
    edge('e1', 'd', 'yes'),
    edge('e2', 'd', 'no'),
    edge('e3', 'no', 'after-no'),
    edge('e4', 'yes', 'after-yes'),
  ]
  expect([...hiddenNodeIds(nodes, edges)].sort()).toEqual(['after-no', 'no'])
})

test('an undecided decision hides nothing', () => {
  const nodes = [node({ id: 'd', node_kind: 'decision' }), node({ id: 'a' })]
  expect(hiddenNodeIds(nodes, [edge('e1', 'd', 'a')]).size).toBe(0)
})

test('groups follow band order and end with a null group', () => {
  const bands = [band('later', 'b'), band('first', 'a')]
  const nodes = [
    node({ id: 'n1', band_id: 'later' }),
    node({ id: 'n2', band_id: 'first' }),
    node({ id: 'n3', band_id: null }),
  ]
  const groups = groupByBand(nodes, bands)
  expect(groups.map((g) => g.band?.id ?? null)).toEqual(['first', 'later', null])
  expect(groups[0].nodes.map((n) => n.id)).toEqual(['n2'])
  expect(groups[1].nodes.map((n) => n.id)).toEqual(['n1'])
  expect(groups[2].nodes.map((n) => n.id)).toEqual(['n3'])
})

test('nodes pointing at a missing band go to the null group', () => {
  const groups = groupByBand([node({ id: 'lost', band_id: 'gone' })], [band('a', 'a')])
  expect(groups[0].nodes).toEqual([])
  expect(groups[1].band).toBeNull()
  expect(groups[1].nodes.map((n) => n.id)).toEqual(['lost'])
})

test('tidy puts each node one column after its deepest source and keeps a cycle in the first column', () => {
  const nodes = ['a', 'b', 'c', 'd'].map((id) => node({ id }))
  const out = tidyLayout(nodes, [edge('1', 'a', 'b'), edge('2', 'b', 'c'), edge('3', 'a', 'c')])
  expect(out.get('a')).toEqual({ x: 80, y: 80 })
  expect(out.get('b')!.x).toBe(400)
  expect(out.get('c')!.x).toBe(720)
  expect(out.get('d')).toEqual({ x: 80, y: 200 })
  const loop = tidyLayout([node({ id: 'x' }), node({ id: 'y' })], [edge('1', 'x', 'y'), edge('2', 'y', 'x')])
  expect([...loop.values()].every((p) => p.x === 80)).toBe(true)
})

test('tidy lays out 500 nodes well inside a frame budget', () => {
  const nodes = Array.from({ length: 500 }, (_, i) => node({ id: `n${i}`, title: `Node ${i}` }))
  const edges = nodes.slice(1).map((n, i) => edge(`e${i}`, `n${Math.floor(i / 2)}`, n.id))
  const start = performance.now()
  const out = tidyLayout(nodes, edges)
  expect(out.size).toBe(500)
  expect(performance.now() - start).toBeLessThan(250)
})
