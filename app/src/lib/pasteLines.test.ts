import { expect, test } from 'vitest'
import { pasteLines } from './pasteLines'

test('splits lines and drops blanks', () => {
  expect(pasteLines('a\n\n  \nb\r\nc')).toEqual([
    { text: 'a', depth: 0 },
    { text: 'b', depth: 0 },
    { text: 'c', depth: 0 },
  ])
})

test('strips list markers', () => {
  const t = pasteLines('- one\n* two\n+ three\n1. four\n2) five\n[ ] six\n- [x] seven\n• eight').map((l) => l.text)
  expect(t).toEqual(['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight'])
})

test('keeps a bare star or dash word', () => {
  expect(pasteLines('-5 degrees\n*bold*').map((l) => l.text)).toEqual(['-5 degrees', '*bold*'])
})

test('indentation nests under the line above and unwinds', () => {
  const l = pasteLines('- a\n  - b\n    - c\n      - d\n  - e\n- f')
  expect(l.map((x) => x.depth)).toEqual([0, 1, 2, 2, 1, 0])
})

test('a marker-only line is dropped', () => {
  expect(pasteLines('-\n[ ]\nx')).toEqual([{ text: 'x', depth: 0 }])
})
