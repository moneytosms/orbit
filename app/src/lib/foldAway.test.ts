// @vitest-environment happy-dom
import { afterEach, expect, test, vi } from 'vitest'
import { findRow, foldAway } from './foldAway'

afterEach(() => {
  document.body.innerHTML = ''
  delete document.documentElement.dataset.motion
})

test('resolves at once when there is no element', async () => {
  await expect(foldAway(null)).resolves.toBeUndefined()
})

test('resolves for a real element when motion is off, without animating', async () => {
  document.documentElement.dataset.motion = 'off'
  document.body.innerHTML = '<li data-task-row="t1">Buy milk</li>'
  const li = document.querySelector('li')!
  const animate = vi.fn()
  Object.defineProperty(li, 'animate', { value: animate, configurable: true })
  await expect(foldAway(li)).resolves.toBeUndefined()
  expect(animate).not.toHaveBeenCalled()
})

test('runs a 240ms fold through the Web Animations API when motion is on', async () => {
  document.documentElement.dataset.motion = 'on'
  document.body.innerHTML = '<div data-row-id="g1">Goal</div>'
  const el = document.querySelector<HTMLElement>('[data-row-id="g1"]')!
  const animate = vi.fn(() => ({ finished: Promise.resolve() }))
  Object.defineProperty(el, 'animate', { value: animate, configurable: true })
  await foldAway(el)
  expect(animate).toHaveBeenCalledTimes(1)
  const [frames, options] = animate.mock.calls[0] as unknown as [Keyframe[], KeyframeAnimationOptions]
  expect(frames[frames.length - 1]).toMatchObject({ height: '0px', opacity: 0 })
  expect(options).toMatchObject({ duration: 240, easing: 'ease' })
  expect(el.style.overflow).toBe('hidden')
})

test('findRow finds a task row by data-task-row', () => {
  document.body.innerHTML = '<ul><li data-task-row="t1">a</li><li data-task-row="t2">b</li></ul>'
  expect(findRow('t2')).toBe(document.querySelector('[data-task-row="t2"]'))
})

test('findRow finds an element by data-row-id', () => {
  document.body.innerHTML = '<article data-row-id="r1">Stretch</article>'
  expect(findRow('r1')?.textContent).toBe('Stretch')
})

test('findRow returns null for an unknown id', () => {
  document.body.innerHTML = '<li data-row-id="r1">x</li>'
  expect(findRow('missing')).toBeNull()
})
