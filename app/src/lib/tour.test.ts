// @vitest-environment happy-dom
import { expect, test } from 'vitest'
import { cardPosition, STEPS, visibleBox } from './tour'

test('a card goes below its target when it fits and above when it does not', () => {
  const view = { width: 1000, height: 700 }
  const card = { width: 300, height: 160 }
  const high = cardPosition({ left: 100, top: 100, width: 80, height: 30 }, card, view)
  expect(high.top).toBe(142)
  const low = cardPosition({ left: 100, top: 600, width: 80, height: 30 }, card, view)
  expect(low.top).toBe(600 - 12 - 160)
})

test('a card stays on screen and centres without a target', () => {
  const view = { width: 400, height: 700 }
  const card = { width: 300, height: 160 }
  expect(cardPosition({ left: 0, top: 50, width: 40, height: 40 }, card, view).left).toBe(12)
  expect(cardPosition({ left: 380, top: 50, width: 40, height: 40 }, card, view).left).toBe(88)
  expect(cardPosition(null, card, view)).toEqual({ left: 50, top: 270 })
})

test('visibleBox ignores elements that take no space', () => {
  document.body.innerHTML = '<div data-tour="x"></div>'
  expect(visibleBox('[data-tour="x"]')).toBeNull()
  expect(visibleBox('[data-tour="none"]')).toBeNull()
})

test('every step has a title and body, and optional steps name a target', () => {
  for (const s of STEPS) {
    expect(s.title.length).toBeGreaterThan(0)
    expect(s.body.length).toBeGreaterThan(0)
    if (s.optional) expect(s.target).toBeTruthy()
  }
})
