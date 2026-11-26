// @vitest-environment happy-dom
import { afterEach, expect, test, vi } from 'vitest'
import { completionFx, motionAllowed } from './completionFx'

afterEach(() => {
  document.body.innerHTML = ''
  delete document.documentElement.dataset.motion
})

const row = () => {
  document.body.innerHTML = '<li data-task-row="t1"><button>o</button></li>'
  return document.querySelector('li')!
}

test('shakes the row and releases stardust when motion is on', () => {
  document.documentElement.dataset.motion = 'on'
  const li = row()
  completionFx('t1')
  expect(li.classList.contains('task-shake')).toBe(true)
  expect(document.querySelectorAll('.stardust i').length).toBe(12)
})

test('does nothing visual when motion is off', () => {
  document.documentElement.dataset.motion = 'off'
  const li = row()
  completionFx('t1')
  expect(motionAllowed()).toBe(false)
  expect(li.classList.contains('task-shake')).toBe(false)
  expect(document.querySelector('.stardust')).toBeNull()
})

test('a missing row is harmless', () => {
  document.documentElement.dataset.motion = 'on'
  expect(() => completionFx('gone')).not.toThrow()
  vi.restoreAllMocks()
})
