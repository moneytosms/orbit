import { describe, expect, it } from 'vitest'
import { groupedKeys, keyLabel, keymap, matcher } from './keys'

const k = (key: string, extra: object = {}) => ({ key, typing: false, ...extra })

describe('keymap matcher', () => {
  it('matches single keys', () => {
    const m = matcher()
    expect(m.press(k('j'), 0)).toBe('cursor.next')
    expect(m.press(k('e'), 0)).toBe('task.complete')
    expect(m.press(k('?'), 0)).toBe('help.open')
  })

  it('matches Shift+T to clear the date and C to open the task', () => {
    const m = matcher()
    expect(m.press(k('T', { shiftKey: true }), 0)).toBe('task.cleardate')
    expect(m.press(k('t'), 0)).toBe('task.today')
    expect(m.press(k('c'), 0)).toBe('cursor.open')
    expect(m.press(k('T', { typing: true }), 0)).toBeNull()
  })

  it('matches g-chords within the timeout', () => {
    const m = matcher()
    expect(m.press(k('g'), 0)).toBeNull()
    expect(m.press(k('i'), 500)).toBe('go.inbox')
    expect(m.press(k('g'), 0)).toBeNull()
    expect(m.press(k('g'), 10)).toBe('go.goals')
  })

  it('drops a chord after 1 s', () => {
    const m = matcher()
    m.press(k('g'), 0)
    expect(m.press(k('i'), 1500)).toBeNull()
  })

  it('ignores unknown keys and breaks a chord on them', () => {
    const m = matcher()
    expect(m.press(k('z'), 0)).toBeNull()
    m.press(k('g'), 0)
    expect(m.press(k('z'), 1)).toBeNull()
    expect(m.press(k('i'), 2)).toBeNull()
  })

  it('does not fire while typing, except Ctrl+K', () => {
    const m = matcher()
    expect(m.press(k('j', { typing: true }), 0)).toBeNull()
    expect(m.press(k('g', { typing: true }), 0)).toBeNull()
    expect(m.press(k('k', { typing: true, ctrlKey: true }), 0)).toBe('search.open')
    expect(m.press(k('K', { metaKey: true }), 0)).toBe('search.open')
  })

  it('ignores Alt combos and plain keys with Ctrl', () => {
    const m = matcher()
    expect(m.press(k('j', { altKey: true }), 0)).toBeNull()
    expect(m.press(k('j', { ctrlKey: true }), 0)).toBeNull()
  })
})

describe('keymap table', () => {
  it('has unique key strings', () => {
    expect(new Set(keymap.map((x) => x.keys)).size).toBe(keymap.length)
  })
  it('groups and labels for the cheat sheet', () => {
    expect(groupedKeys().flatMap((g) => g.keys)).toHaveLength(keymap.length)
    expect(keyLabel('g i')).toBe('g then i')
    expect(keyLabel('ctrl+k')).toBe('Ctrl K')
    expect(keyLabel('T')).toBe('Shift T')
  })
})
