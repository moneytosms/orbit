import { describe, expect, test } from 'vitest'
import {
  colsFor,
  defaultLayout,
  defaultPageLayout,
  dragRect,
  expand,
  fits,
  pageKinds,
  reflow,
  doneShare,
  move,
  phoneOrder,
  place,
  resize,
  setRect,
  taskIsUpcoming,
  withCalendar,
  type Widget,
} from './dashboard'

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
  expect(d).toHaveLength(7)
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

test("Upcoming omits overdue and due-today tasks already shown by What's next", () => {
  const rows = [
    { id: 'late', kind: 'task', due: { date: '2026-11-21' } },
    { id: 'today', kind: 'task', due: { date: '2026-11-22' } },
    { id: 'future', kind: 'task', due: { date: '2026-11-23' } },
    { id: 'goal', kind: 'goal' },
  ]
  expect(rows.filter((row) => taskIsUpcoming(row, '2026-11-22')).map((row) => row.id)).toEqual(['future', 'goal'])
})

test('doneShare is done over all of today and 1 on an empty day', () => {
  expect(doneShare(3, 1)).toBe(0.75)
  expect(doneShare(0, 4)).toBe(0)
  expect(doneShare(0, 0)).toBe(1)
})

test('a layout saved before the Calendar widget gets it appended once, without moving the rest', () => {
  const saved = [w('glance', 0, 0, 12, 2), w('next', 0, 2, 6, 4)]
  const out = withCalendar(saved)
  expect(out.slice(0, 2)).toEqual(saved)
  expect(out[2]).toMatchObject({ kind: 'calendar', x: 0, y: 6, hidden: false })
  expect(overlaps(out)).toBe(false)
  expect(withCalendar(out)).toEqual(out)
  const hid = [...saved, { ...w('calendar', 0, 6, 6, 2), hidden: true }]
  expect(withCalendar(hid)).toEqual(hid)
})

test('colsFor is 12 on desktop, 6 on tablet and 2 on phone', () => {
  expect([1440, 1024, 1023, 640, 639, 360].map(colsFor)).toEqual([12, 12, 6, 6, 2, 2])
})

test('reflow keeps the desktop layout, scales to 6 columns with a 2-column minimum, and stacks on a phone', () => {
  const ws = [
    w('a', 0, 0, 12, 2),
    w('b', 0, 2, 6, 3),
    w('c', 6, 2, 6, 3),
    w('d', 10, 5, 1, 1),
    w('h', 0, 9, 4, 2, true),
  ]
  expect(reflow(ws, 12).map((x) => x.id)).toEqual(['a', 'b', 'c', 'd'])
  const t = reflow(ws, 6)
  expect(t.map((x) => [x.id, x.x, x.w])).toEqual([
    ['a', 0, 6],
    ['b', 0, 3],
    ['c', 3, 3],
    ['d', 4, 2],
  ])
  expect(overlaps(t)).toBe(false)
  expect(t.every((x) => x.x + x.w <= 6 && x.w >= 2)).toBe(true)
  const p = reflow(ws, 2)
  expect(p.map((x) => [x.id, x.x, x.w, x.y])).toEqual([
    ['a', 0, 2, 0],
    ['b', 0, 2, 2],
    ['c', 0, 2, 5],
    ['d', 0, 2, 8],
  ])
})

test('reflow on a tablet pushes tiles that would collide down, in reading order', () => {
  const t = reflow([w('a', 0, 0, 1, 2), w('b', 1, 0, 1, 2)], 6)
  expect(overlaps(t)).toBe(false)
  expect(t.map((x) => x.id)).toEqual(['a', 'b'])
})

test('expand maps a tablet edit back onto 12 columns and keeps hidden tiles', () => {
  const saved = [w('a', 0, 0, 12, 2), w('b', 0, 2, 6, 3), w('h', 0, 9, 4, 2, true)]
  const edited = reflow(saved, 6).map((x) => (x.id === 'b' ? { ...x, x: 3, y: 2 } : x))
  const back = expand(edited, 6, saved)
  expect(back.map((x) => x.id)).toEqual(['a', 'b', 'h'])
  expect(back.find((x) => x.id === 'b')).toMatchObject({ x: 6, w: 6, y: 2 })
  expect(back.find((x) => x.id === 'h')).toEqual(saved[2])
  expect(expand(saved, 12, saved)).toBe(saved)
})

test('move and resize honour a narrower column count', () => {
  const ws = [w('a', 0, 0, 2, 1)]
  expect(move(ws, 'a', 9, 0, 6)[0].x).toBe(4)
  expect(resize(ws, 'a', 9, 1, 6)[0].w).toBe(6)
})

test('fits is false off the grid or onto another visible tile, true for hidden ones and itself', () => {
  const ws = [w('a', 0, 0, 4, 2), w('b', 4, 0, 4, 2), w('h', 8, 0, 4, 2, true)]
  expect(fits(ws, 'a', { x: 0, y: 0, w: 4, h: 2 })).toBe(true)
  expect(fits(ws, 'a', { x: 2, y: 0, w: 4, h: 2 })).toBe(false)
  expect(fits(ws, 'a', { x: 9, y: 0, w: 4, h: 2 })).toBe(false)
  expect(fits(ws, 'a', { x: -1, y: 0, w: 4, h: 2 })).toBe(false)
  expect(fits(ws, 'a', { x: 8, y: 0, w: 4, h: 2 })).toBe(true)
  expect(fits(ws, 'a', { x: 0, y: 0, w: 7, h: 2 }, 6)).toBe(false)
})

test('page layouts: each kind has a default without overlaps, only with tiles its page offers', () => {
  for (const kind of ['area', 'project'] as const) {
    const d = defaultPageLayout(kind)
    expect(overlaps(d)).toBe(false)
    expect(d.every((x) => x.x + x.w <= 12)).toBe(true)
    const offered = pageKinds(kind).map((k) => k.kind)
    expect(d.every((x) => offered.includes(x.kind))).toBe(true)
  }
  expect(defaultPageLayout('area').map((x) => x.kind)).toContain('goals')
  expect(defaultPageLayout('project').map((x) => x.kind)).not.toContain('goals')
})

describe('dragRect and setRect', () => {
  const base = { x: 4, y: 2, w: 4, h: 3 }
  test('the east and south handles grow the tile and keep its corner', () => {
    expect(dragRect(base, 'e', 2, 0)).toEqual({ x: 4, y: 2, w: 6, h: 3 })
    expect(dragRect(base, 'se', 1, 2)).toEqual({ x: 4, y: 2, w: 5, h: 5 })
  })
  test('the west and north handles move the near edge and keep the far one', () => {
    expect(dragRect(base, 'w', -2, 0)).toEqual({ x: 2, y: 2, w: 6, h: 3 })
    expect(dragRect(base, 'nw', -1, -1)).toEqual({ x: 3, y: 1, w: 5, h: 4 })
  })
  test('a handle never shrinks past the minimum or off the grid', () => {
    expect(dragRect(base, 'e', -9, 0).w).toBe(2)
    expect(dragRect(base, 'w', 9, 0)).toEqual({ x: 6, y: 2, w: 2, h: 3 })
    expect(dragRect(base, 'n', 0, 9)).toEqual({ x: 4, y: 4, w: 4, h: 1 })
    expect(dragRect(base, 'w', -9, 0)).toEqual({ x: 0, y: 2, w: 8, h: 3 })
    expect(dragRect(base, 'e', 9, 0).w).toBe(8)
  })
  test('setRect clamps and pushes the tiles it lands on down', () => {
    const list: Widget[] = [
      { id: 'a', kind: 'next', x: 0, y: 0, w: 6, h: 2, hidden: false },
      { id: 'b', kind: 'goals', x: 6, y: 0, w: 6, h: 2, hidden: false },
    ]
    const out = setRect(list, 'a', { x: 5, y: 0, w: 9, h: 2 })
    const a = out.find((w) => w.id === 'a')!
    expect(a.x + a.w).toBeLessThanOrEqual(12)
    expect(out.find((w) => w.id === 'b')!.y).toBeGreaterThanOrEqual(2)
  })
})
