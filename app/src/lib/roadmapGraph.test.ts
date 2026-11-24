import { expect, test } from 'vitest'
import { borderPoint, convertTargets, edgeEnds, groupByBand, hiddenNodeIds, placeNew, tidyLayout } from './roadmapGraph'
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

test('a task can become a note, decision or heading but a linked goal cannot become a task', () => {
  expect(convertTargets({ node_kind: 'task' }).map((t) => [t.kind, t.ok])).toEqual([
    ['decision', true],
    ['note', true],
    ['heading', true],
  ])
  const fromGoal = convertTargets({ node_kind: 'goal' }).find((t) => t.kind === 'task')
  expect(fromGoal).toMatchObject({ ok: false })
  expect(fromGoal?.reason).toMatch(/goal/)
  expect(convertTargets({ node_kind: 'note' }).find((t) => t.kind === 'task')?.ok).toBe(true)
})

test('a new node goes right of the anchor and steps down past nodes in its way', () => {
  expect(placeNew([], { x: 0, y: 0 }, { x: 5, y: 5 })).toEqual({ x: 320, y: 0 })
  expect(placeNew([{ x: 0, y: 0 }], undefined, { x: 100, y: 100 })).toEqual({ x: 100, y: 100 })
  const busy = [{ x: 320, y: 0 }]
  expect(placeNew(busy, { x: 0, y: 0 }, { x: 0, y: 0 })).toEqual({ x: 320, y: 96 })
})

test('connections land on the node border: rectangles and decision diamonds', () => {
  const rect = { x: 0, y: 0, node_kind: 'task' as const }
  const diamond = { x: 0, y: 0, node_kind: 'decision' as const }
  // Due right of a 240x64 box centred at (120,32): left/right edge midpoints, top/bottom edge midpoints.
  expect(borderPoint(rect, { x: 1000, y: 32 })).toEqual({ x: 240, y: 32 })
  expect(borderPoint(rect, { x: -500, y: 32 })).toEqual({ x: 0, y: 32 })
  expect(borderPoint(rect, { x: 120, y: 900 })).toEqual({ x: 120, y: 64 })
  // Diagonal at the box's own slope hits the corner.
  expect(borderPoint(rect, { x: 240, y: 64 })).toEqual({ x: 240, y: 64 })
  // The diamond's right vertex, and a diagonal that meets the edge between vertices (|dx|/120+|dy|/32 = 1).
  expect(borderPoint(diamond, { x: 500, y: 32 })).toEqual({ x: 240, y: 32 })
  const d = borderPoint(diamond, { x: 120 + 60, y: 32 + 16 })
  expect(d.x).toBeCloseTo(180)
  expect(d.y).toBeCloseTo(48)
  // Zero distance falls back to the centre.
  expect(borderPoint(rect, { x: 120, y: 32 })).toEqual({ x: 120, y: 32 })
  // Both ends sit on their own borders and on the segment between the centres, whichever way the nodes lie.
  const e = edgeEnds(rect, { x: 400, y: 200, node_kind: 'decision' })
  expect(e.x1 === 240 || e.y1 === 64).toBe(true)
  const slope = (200 + 32 - 32) / (400 + 120 - 120)
  expect((e.y2 - 32) / (e.x2 - 120)).toBeCloseTo(slope)
  expect(Math.abs(e.x2 - 520) / 120 + Math.abs(e.y2 - 232) / 32).toBeCloseTo(1)
  const back = edgeEnds({ x: 400, y: 0, node_kind: 'task' }, rect)
  expect(back).toMatchObject({ x1: 400, y1: 32, x2: 240, y2: 32 })
})

test('upward and leftward connections land on the borders too, and a bend keeps both ends on them', () => {
  const a = { x: 0, y: 400, node_kind: 'task' as const }
  const b = { x: 0, y: 0, node_kind: 'task' as const }
  expect(edgeEnds(a, b)).toMatchObject({ x1: 120, y1: 400, x2: 120, y2: 64 })
  const bent = edgeEnds(a, b, 40)
  for (const [x, y, n] of [
    [bent.x1, bent.y1, a],
    [bent.x2, bent.y2, b],
  ] as const) {
    const onX = Math.abs(x - n.x) < 1e-6 || Math.abs(x - n.x - 240) < 1e-6
    const onY = Math.abs(y - n.y) < 1e-6 || Math.abs(y - n.y - 64) < 1e-6
    expect(onX || onY).toBe(true)
  }
  expect(edgeEnds(b, a, 40).cx).not.toBeCloseTo(bent.cx)
})

test('tidy runs in every direction, including against a cycle', () => {
  const ns = [node({ id: 'a' }), node({ id: 'b' }), node({ id: 'c' })]
  const es = [edge('1', 'a', 'b'), edge('2', 'b', 'c')]
  const at = (dir: 'lr' | 'rl' | 'tb' | 'bt') => tidyLayout(ns, es, dir)
  expect(at('lr').get('c')).toEqual({ x: 720, y: 80 })
  expect(at('rl').get('a')).toEqual({ x: 720, y: 80 })
  expect(at('rl').get('c')).toEqual({ x: 80, y: 80 })
  expect(at('tb').get('c')).toEqual({ x: 80, y: 360 })
  expect(at('bt').get('a')).toEqual({ x: 80, y: 360 })
  expect(at('bt').get('c')).toEqual({ x: 80, y: 80 })
  const loop = tidyLayout(ns, [...es, edge('3', 'c', 'a')], 'tb')
  expect(loop.size).toBe(3)
})
