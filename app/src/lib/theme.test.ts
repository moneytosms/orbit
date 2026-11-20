import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../../wasm/orbit_wasm', () => ({ default: async () => {}, theme_list: () => '[]', theme_tokens: () => '{}' }))

const store = new Map<string, string>()
vi.stubGlobal('localStorage', {
  getItem: (k: string) => store.get(k) ?? null,
  setItem: (k: string, v: string) => void store.set(k, v),
  removeItem: (k: string) => void store.delete(k),
})
const root = {
  dataset: {} as Record<string, string>,
  style: { length: 0, zoom: '', setProperty() {}, removeProperty() {} },
}
vi.stubGlobal('document', { documentElement: root })

const load = async () => {
  vi.resetModules()
  return import('./theme')
}

describe('display preferences', () => {
  beforeEach(() => {
    store.clear()
    root.dataset = {}
    root.style.zoom = ''
  })

  it('defaults', async () => {
    const t = await load()
    expect([t.getTextSize(), t.getTypeface(), t.getLeading(), t.getMotion()]).toEqual([
      '100',
      'figtree',
      'default',
      'system',
    ])
    expect(root.style.zoom).toBe('')
    expect(root.dataset).toMatchObject({ font: 'figtree', leading: 'default', motion: 'system' })
  })

  it('sets, applies and persists', async () => {
    const t = await load()
    t.setTextSize('150')
    t.setTypeface('mono')
    t.setLeading('relaxed')
    t.setMotion('off')
    expect(root.style.zoom).toBe('1.5')
    expect(root.dataset).toMatchObject({ font: 'mono', leading: 'relaxed', motion: 'off' })
    expect([...store]).toEqual([
      ['textSize', '150'],
      ['typeface', 'mono'],
      ['leading', 'relaxed'],
      ['motion', 'off'],
    ])
  })

  it('restores on load', async () => {
    store.set('textSize', '115')
    store.set('typeface', 'serif')
    store.set('leading', 'compact')
    store.set('motion', 'on')
    const t = await load()
    expect([t.getTextSize(), t.getTypeface(), t.getLeading(), t.getMotion()]).toEqual(['115', 'serif', 'compact', 'on'])
    expect(root.style.zoom).toBe('1.15')
  })

  it('ignores invalid stored values', async () => {
    store.set('textSize', '999')
    store.set('typeface', 'comic')
    store.set('leading', '')
    store.set('motion', 'maybe')
    const t = await load()
    expect([t.getTextSize(), t.getTypeface(), t.getLeading(), t.getMotion()]).toEqual([
      '100',
      'figtree',
      'default',
      'system',
    ])
  })

  it('reset restores defaults and clears storage', async () => {
    const t = await load()
    t.setTextSize('130')
    t.setTypeface('serif')
    t.resetAppearance()
    expect(t.getTextSize()).toBe('100')
    expect(t.getTypeface()).toBe('figtree')
    expect(store.size).toBe(0)
    expect(root.style.zoom).toBe('')
  })

  it('survives failing localStorage', async () => {
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('blocked')
      },
      setItem: () => {
        throw new Error('blocked')
      },
      removeItem: () => {
        throw new Error('blocked')
      },
    })
    const t = await load()
    expect(t.getTextSize()).toBe('100')
    t.setTextSize('130')
    expect(t.getTextSize()).toBe('130')
    expect(root.style.zoom).toBe('1.3')
  })
})
