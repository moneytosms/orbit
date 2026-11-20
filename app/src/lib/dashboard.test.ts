import { expect, test } from 'vitest'
import { defaultLayout, move, phoneOrder, place, resize, type Widget } from './dashboard'

const w = (id: string, x: number, y: number, ww: number, h: number, hidden = false): Widget => ({
  id,
  kind: id,
  x,
  y,
  w: ww,
  h,
  hidden,
})
const overlaps = (ws: Widget[]) =>
  ws
    .filter((a) => !a.hidden)
    .some((a, i) =>
      ws
        .filter((b) => !b.hidden)
        .slice(i + 1)
        .some((b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h),
    )

test('the default layout has no overlaps and fits 12 columns', () => {
  const d = defaultLayout()
  expect(d).toHaveLength(6)
  expect(overlaps(d)).toBe(false)
  expect(d.every((x) => x.x + x.w <= 12)).toBe(true)
})

test('move clamps to the columns and pushes overlapping widgets down', () => {
  const ws = [w('a', 0, 0, 6, 2), w('b', 6, 0, 6, 2)]
  const moved = move(ws, 'a', 9, 0)
  expect(moved.find((x) => x.id === 'a')).toMatchObject({ x: 6, y: 0 })
  expect(moved.find((x) => x.id === 'b')).toMatchObject({ x: 6, y: 2 })
  expect(overlaps(moved)).toBe(false)
  expect(move(ws, 'a', -3, -1).find((x) => x.id === 'a')).toMatchObject({ x: 0, y: 0 })
})

test('resize clamps to the columns and pushes widgets below it down', () => {
  const ws = [w('a', 0, 0, 4, 1), w('b', 0, 1, 4, 1), w('c', 8, 0, 4, 1)]
  const r = resize(ws, 'a', 20, 2)
  expect(r.find((x) => x.id === 'a')).toMatchObject({ w: 12, h: 2 })
  expect(r.find((x) => x.id === 'b')?.y).toBe(2)
  expect(r.find((x) => x.id === 'c')?.y).toBe(2)
  expect(overlaps(r)).toBe(false)
  expect(resize(ws, 'a', 0, 0).find((x) => x.id === 'a')).toMatchObject({ w: 1, h: 1 })
})

test('hidden widgets neither block nor get pushed', () => {
  const ws = [w('a', 0, 0, 6, 1), w('h', 0, 1, 6, 1, true)]
  expect(move(ws, 'a', 0, 1).find((x) => x.id === 'h')?.y).toBe(1)
})

test('phoneOrder is y then x, without hidden ones', () => {
  const ws = [w('c', 0, 2, 12, 1), w('b', 6, 0, 6, 1), w('a', 0, 0, 6, 1), w('h', 0, 1, 6, 1, true)]
  expect(phoneOrder(ws).map((x) => x.id)).toEqual(['a', 'b', 'c'])
})

test('place puts a widget at the bottom and shows a hidden one again', () => {
  const ws = [w('a', 0, 0, 6, 2), w('h', 6, 0, 6, 1, true)]
  const p = place(ws, ws[1])
  expect(p.find((x) => x.id === 'h')).toMatchObject({ x: 0, y: 2, hidden: false })
  expect(place(ws, w('n', 0, 0, 6, 1)).find((x) => x.id === 'n')?.y).toBe(2)
})
