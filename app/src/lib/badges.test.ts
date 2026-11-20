import { beforeEach, expect, test, vi } from 'vitest'
import { awardUp } from './sync'

beforeEach(() => {
  const store = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
  })
})

test('the first run records the Awards without announcing them', () => {
  expect(awardUp(3)).toBe(false)
  expect(awardUp(3)).toBe(false)
})

test('a later rise in Awards is announced once', () => {
  awardUp(3)
  expect(awardUp(4)).toBe(true)
  expect(awardUp(4)).toBe(false)
})
