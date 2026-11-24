import { expect, test } from 'vitest'
import { showChecklist } from './onboarding'

test('shows only with no Areas and not dismissed', () => {
  expect(showChecklist(0, false)).toBe(true)
  expect(showChecklist(0, true)).toBe(false)
  expect(showChecklist(1, false)).toBe(false)
  expect(showChecklist(3, true)).toBe(false)
})
