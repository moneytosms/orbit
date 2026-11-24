import { beforeEach, expect, test, vi } from 'vitest'

const sync = vi.hoisted(() => ({
  lastTask: null,
  tree: vi.fn(async () => []),
  views: vi.fn(async () => []),
  sync: vi.fn(),
}))
vi.mock('./sync', () => sync)
vi.mock('../../wasm/orbit_wasm', () => ({ inbox_id: () => 'inbox' }))

import { filterActions, runAction, type Action } from './actions'

const act = (id: string, name: string): Action => ({ id, name, group: 'g', run: () => {} })
const list = [act('a', 'Create Task'), act('b', 'Toggle asset key'), act('c', 'Toggle theme'), act('d', 'Sync now')]

beforeEach(() => {
  vi.clearAllMocks()
  vi.stubGlobal('location', { hash: '' })
})

test('empty or blank query returns the list unchanged', () => {
  expect(filterActions(list, '')).toBe(list)
  expect(filterActions(list, '   ')).toBe(list)
})

test('matching is case-insensitive and ignores spaces in the query', () => {
  expect(filterActions(list, 'THEME').map((a) => a.id)).toEqual(['c'])
  expect(filterActions(list, 's y n c').map((a) => a.id)).toEqual(['d'])
})

test('non-matching query returns nothing', () => {
  expect(filterActions(list, 'zzz')).toEqual([])
})

test('a substring match ranks above a scattered subsequence', () => {
  expect(filterActions(list, 'task').map((a) => a.id)).toEqual(['a', 'b'])
})

test('runAction with an unknown id is a no-op', async () => {
  await expect(runAction('nope')).resolves.toBeUndefined()
  expect(sync.sync).not.toHaveBeenCalled()
})

test('runAction runs the action with that id', async () => {
  await runAction('sync.now')
  expect(sync.sync).toHaveBeenCalledTimes(1)
})
