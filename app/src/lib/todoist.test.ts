import { afterEach, expect, test, vi } from 'vitest'
import { startImport } from './todoist'

const reply = (body: unknown) => new Response(JSON.stringify(body), { status: 200 })

afterEach(() => vi.unstubAllGlobals())

test('startImport polls until the preview is ready', async () => {
  const answers = [{ job: 'j1' }, { status: 'running' }, { status: 'ready', projects: [], totals: {} }]
  const fetch = vi.fn(async () => reply(answers.shift()))
  vi.stubGlobal('fetch', fetch)
  const { job, preview } = await startImport('tok', 0)
  expect(job).toBe('j1')
  expect(preview.projects).toEqual([])
  expect(fetch).toHaveBeenCalledTimes(3)
  expect(fetch).toHaveBeenNthCalledWith(
    1,
    '/import/todoist',
    expect.objectContaining({ method: 'POST', body: JSON.stringify({ token: 'tok' }) }),
  )
})

test('startImport throws the failure message', async () => {
  const answers = [{ job: 'j1' }, { status: 'failed', error: 'Todoist rejected the token' }]
  vi.stubGlobal('fetch', async () => reply(answers.shift()))
  await expect(startImport('bad', 0)).rejects.toThrow('Todoist rejected the token')
})
