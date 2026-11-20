import 'fake-indexeddb/auto'
import { readFileSync } from 'node:fs'
import { beforeAll, expect, test } from 'vitest'
import { initSync } from '../../wasm/orbit_wasm'
import { filterActions, type Action } from './actions'
import { createSearch, groupHits } from './search'

const id = (n: number) => `00000000-0000-7000-8000-${String(n).padStart(12, '0')}`
const ROOT = '00000000-0000-7000-8000-000000000001'

const task = (n: number, title: string, description = '') => ({
  kind: 'task',
  id: id(n),
  parent_id: ROOT,
  title,
  description,
  order: 'a',
  completed_at: null,
})
const page = (n: number, title: string, body: string) => ({
  kind: 'page',
  id: id(n),
  parent_id: ROOT,
  title,
  body,
  rev: 0,
  order: 'a',
  trashed_at: null,
  links: [],
  blobs: [],
})

beforeAll(() => {
  initSync({ module: readFileSync(new URL('../../wasm/orbit_wasm_bg.wasm', import.meta.url)) })
})

test('a task is found by a fuzzy title', () => {
  const s = createSearch(false)
  s.upsert(task(10, 'Renew passport'))
  s.upsert(task(11, 'Buy milk'))
  const hits = s.query('rnwpass')
  expect(hits.map((h) => h.id)).toEqual([id(10)])
  expect(hits[0].kind).toBe('task')
})

test('a page is found by the prefix of a word in its body', () => {
  const s = createSearch(false)
  s.upsert(page(20, 'Trip notes', 'We should book the accommodation early'))
  const hits = s.query('accomm')
  expect(hits.map((h) => h.id)).toEqual([id(20)])
  expect(groupHits(hits)).toMatchObject([{ kind: 'page' }])
})

test('an attachment is found by its filename', () => {
  const s = createSearch(false)
  s.upsert(page(30, 'Taxes', 'See ![receipt-scan.png](blob:abc123) attached'))
  expect(s.query('receipt-scan').map((h) => h.id)).toEqual([id(30)])
})

test('opened items come first on empty input', () => {
  const s = createSearch(false)
  s.upsert(task(40, 'Alpha'))
  s.upsert(task(41, 'Beta'))
  s.recordOpen(id(41), Date.now())
  expect(s.query('')[0].id).toBe(id(41))
})

test('> filters actions by name', () => {
  const run = () => {}
  const list: Action[] = [
    { id: 'a', name: 'Create Task', group: 'Create', run },
    { id: 'b', name: 'Toggle theme', group: 'App', run },
    { id: 'c', name: 'Go to Settings', group: 'Go', run },
  ]
  expect(filterActions(list, 'thm').map((a) => a.id)).toEqual(['b'])
  expect(filterActions(list, '').length).toBe(3)
  expect(filterActions(list, 'settings').map((a) => a.id)).toEqual(['c'])
})
