import { expect, test } from 'vitest'
import { clampZoom, fit, frameAt, gatePath, layout, linkPath, zoomAt, STEP_H, STEP_W } from './canvas'
import type { PhaseSummary } from './sync'

const item = (id: string, links: string[] = [], kind: 'task' | 'milestone' = 'task') => ({
  id,
  kind,
  title: id,
  progress: 0,
  links,
})
const phase = (id: string, items: ReturnType<typeof item>[], extra: Partial<PhaseSummary> = {}): PhaseSummary => ({
  id,
  name: id,
  start: null,
  end: null,
  completed: false,
  progress: 0,
  current: false,
  gate_id: null,
  gate_title: null,
  locked: false,
  items,
  ...extra,
})

test('frames run left to right and steps stack inside them', () => {
  const l = layout([phase('a', [item('x'), item('y')]), phase('b', [])])
  expect(l.frames.map((f) => f.x)).toEqual([0, 350])
  expect(l.frames[1].h).toBe(120)
  expect(l.steps.map((s) => [s.x, s.y])).toEqual([
    [16, 84],
    [16, 152],
  ])
  expect(l.steps[0]).toMatchObject({ w: STEP_W, h: STEP_H })
  expect(l.frames[0].h).toBeGreaterThanOrEqual(l.steps[1].y + STEP_H)
})

test('frame state follows done, locked and current', () => {
  const l = layout([
    phase('a', [], { completed: true }),
    phase('b', [], { locked: true }),
    phase('c', [], { current: true }),
    phase('d', []),
  ])
  expect(l.frames.map((f) => f.state)).toEqual(['done', 'locked', 'active', 'todo'])
})

test('links keep only placed targets and mark links into a locked phase', () => {
  const l = layout([phase('a', [item('x', ['y', 'gone', 'x'])]), phase('b', [item('y')], { locked: true })])
  expect(l.links).toEqual([{ from: 'x', to: 'y', locked: true }])
})

test('a gate line needs its Milestone placed on the canvas', () => {
  const placed = layout([phase('a', [item('m', [], 'milestone')]), phase('b', [], { gate_id: 'm' })])
  expect(placed.gates).toEqual([{ from: 'm', to: 'b' }])
  const away = layout([phase('a', []), phase('b', [], { gate_id: 'm', gate_title: 'M' })])
  expect(away.gates).toEqual([])
})

test('link paths go side to side across frames and top to bottom within one', () => {
  const l = layout([phase('a', [item('x'), item('y')]), phase('b', [item('z')])])
  const [x, y, z] = l.steps
  expect(linkPath(x, z)).toMatch(/^M294 112C/)
  expect(linkPath(x, y)).toMatch(/^M155 140C/)
  expect(gatePath(x, l.frames[1])).toMatch(/^M294 112C.* 350 28$/)
})

test('zoom clamps and keeps the point under the cursor fixed', () => {
  expect(clampZoom(10)).toBe(2)
  expect(clampZoom(0.01)).toBe(0.4)
  const v = zoomAt({ x: 10, y: 20, z: 1 }, 2, 110, 120)
  expect(v.z).toBe(2)
  expect((110 - v.x) / v.z).toBeCloseTo((110 - 10) / 1)
  expect(zoomAt({ x: 0, y: 0, z: 2 }, 2, 0, 0).z).toBe(2)
})

test('frameAt finds the frame under a world point and fit centres the canvas', () => {
  const l = layout([phase('a', []), phase('b', [])])
  expect(frameAt(l.frames, 400, 50)?.id).toBe('b')
  expect(frameAt(l.frames, 330, 50)).toBeNull()
  const v = fit(l.frames, 1000, 600)
  expect(v.z).toBeLessThanOrEqual(1)
  expect(v.x).toBeGreaterThanOrEqual(0)
})
