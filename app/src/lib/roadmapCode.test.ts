import 'fake-indexeddb/auto'
import { readFileSync } from 'node:fs'
import { beforeAll, expect, test, vi } from 'vitest'
import { initSync } from '../../wasm/orbit_wasm'
import {
  createArea,
  createGraphEdge,
  createGraphNode,
  createPhase,
  createRoadmap,
  createTask,
  decideGraphNode,
  exportRoadmapCode,
  importRoadmapCode,
  parseRoadmapCode,
  previewRoadmapCode,
  roadmapCode,
  roadmapGraph,
  tree,
  updateGraphNode,
} from './sync'
import { PROMPT_EXAMPLE, roadmapPrompt } from './roadmapPrompt'

// Local-only: these tests never reach the network, so every sync attempt fails like an offline device.
vi.stubGlobal('fetch', async () => {
  throw new TypeError('Failed to fetch')
})

beforeAll(() => {
  initSync({ module: readFileSync(new URL('../../wasm/orbit_wasm_bg.wasm', import.meta.url)) })
})

async function newRoadmap(area: string, name: string) {
  await createArea(area)
  const owner = (await tree()).find((x) => x.area.name === area)!.area
  return createRoadmap(owner.id, name)
}

test('an exported Roadmap imports back unchanged', async () => {
  const road = await newRoadmap('Round trip', 'Upskill')
  const band = await createPhase(road, 'Week 1', '2026-11-21', '2026-11-27')
  const task = await createTask('Read the Rust book')
  const choice = await createGraphNode(road, 'decision', 80, 120, 'Do I need a PhD?')
  const read = await createGraphNode(road, 'task', 360, 120, 'Read the Rust book', task)
  const note = await createGraphNode(road, 'note', 360, 260, 'Ask a friend')
  const funded = await createGraphEdge(road, choice, read, 'then', 'if funded')
  await createGraphEdge(road, choice, note, 'decides', 'no')
  await decideGraphNode(choice, funded)
  await updateGraphNode(read, { band_id: band })

  const before = await roadmapCode(road)
  const text = exportRoadmapCode(before)
  const parsed = parseRoadmapCode(text)
  if (!('ok' in parsed)) throw new Error(parsed.error.message)

  const preview = await previewRoadmapCode(road, parsed.ok)
  expect(preview.errors).toEqual([])
  expect(preview.diff).toEqual({
    createNodes: 0,
    updateNodes: 3,
    createEdges: 0,
    updateEdges: 2,
    createBands: 0,
    updateBands: 1,
  })

  await importRoadmapCode(road, parsed.ok)
  const after = await roadmapCode(road)
  expect(after).toEqual(before)
  expect(after.bands[Object.keys(after.bands)[0]].name).toBe('Week 1')
  expect(
    Object.values(after.edges)
      .filter((edge) => edge.taken)
      .map((edge) => edge.label),
  ).toEqual(['if funded'])
  expect(Object.values(after.nodes).find((node) => node.kind === 'task')?.ref_id).toBe(task)
})

test('id-less edges in code stay id-less through export and parse', () => {
  const text = ['%%orbit:v1', 'flowchart LR', '  a["Start"]:::task', '  b["Finish"]:::task', '  a --> b'].join('\n')
  const first = parseRoadmapCode(text)
  if (!('ok' in first)) throw new Error(first.error.message)
  expect(first.ok.edges).toHaveLength(1)
  expect(first.ok.edges[0].id).toBeNull()

  const again = parseRoadmapCode(exportRoadmapCode(first.ok))
  if (!('ok' in again)) throw new Error(again.error.message)
  expect(again.ok.edges[0].id).toBeNull()
})

test('a code with a bad reference reports its line and writes nothing', async () => {
  const road = await newRoadmap('Validation', 'Guarded')
  await createGraphNode(road, 'note', 40, 40, 'Existing')
  const before = await roadmapGraph(road)

  const text = [
    '%%orbit:v1',
    'flowchart LR',
    '  %%orbit node g1 {"ref":"00000000-0000-7000-8000-0000000000aa","x":120,"y":40}',
    '  g1["Ghost"]:::task',
    '  %%orbit node n1 {"x":300,"y":40}',
    '  n1["Fresh"]:::note',
  ].join('\n')
  const parsed = parseRoadmapCode(text)
  if (!('ok' in parsed)) throw new Error(parsed.error.message)

  const preview = await previewRoadmapCode(road, parsed.ok)
  expect(preview.errors).toHaveLength(1)
  // A node's error points at its shape line (the node declaration), which is line 4 here.
  expect(preview.errors[0].message).toMatch(/^Line 4: Reference .* is missing or is not a task$/)

  await expect(importRoadmapCode(road, parsed.ok)).rejects.toThrow('Line 4:')
  const after = await roadmapGraph(road)
  expect(after.nodes.map((node) => node.title)).toEqual(before.nodes.map((node) => node.title))
  expect(after.edges).toHaveLength(before.edges.length)
  expect(after.bands).toHaveLength(before.bands.length)
})

test('the Copy for AI prompt lists the closed kinds and real ids, and its own example imports', async () => {
  const prompt = roadmapPrompt(
    [{ id: 'area-1', title: 'College' }],
    [{ id: 'proj-1', title: 'Thesis' }],
    [{ id: 'goal-1', title: 'Get fit' }],
    [{ id: 'task-1', title: 'Read the Rust book' }],
  )
  for (const needle of [
    '%%orbit:v1',
    'single fenced code block and nothing else',
    'task, project, goal, milestone, decision, note, text, heading',
    'then, blocks, depends_on, decides',
    'College (id area-1)',
    'Thesis (id proj-1)',
    'Get fit (id goal-1)',
    'Read the Rust book (id task-1)',
  ])
    expect(prompt).toContain(needle)
  const road = await newRoadmap('Prompt', 'Fresh')
  const parsed = parseRoadmapCode(PROMPT_EXAMPLE)
  if (!('ok' in parsed)) throw new Error(parsed.error.message)
  const preview = await previewRoadmapCode(road, parsed.ok)
  expect(preview.errors).toEqual([])
  expect(preview.diff).toMatchObject({ createNodes: 3, createEdges: 2, createBands: 1 })
  await importRoadmapCode(road, parsed.ok)
  const graph = await roadmapGraph(road)
  expect(graph.nodes.map((n) => n.node_kind).sort()).toEqual(['decision', 'task', 'task'])
  expect(graph.nodes.filter((n) => n.band_id)).toHaveLength(1)
  expect(parseRoadmapCode(exportRoadmapCode(await roadmapCode(road)))).toHaveProperty('ok')
})
