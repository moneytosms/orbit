import { expect, test } from 'vitest'
import { commandsFirst, matchCommands, matchTier } from './palette'

const cmd = (name: string) => ({ id: name, name })
const list = [
  cmd('Go to Settings'),
  cmd('Go to Archive & Trash'),
  cmd('Go to Upcoming'),
  cmd('Go to View: Calendar'),
  cmd('Replay tour'),
  cmd('Create Task'),
  cmd('Duplicate task'),
  cmd('Toggle theme'),
  cmd('Keyboard shortcuts'),
]
const names = (q: string) => matchCommands(list, q).map((m) => m.item.name)

test('a plain query finds the Go to command for a page, without the > prefix', () => {
  expect(names('settings')).toEqual(['Go to Settings'])
  expect(names('upcoming')).toEqual(['Go to Upcoming'])
  expect(names('archive')).toEqual(['Go to Archive & Trash'])
})

test('the Go to verb is not needed to match, and an exact title ranks as tier 0', () => {
  expect(matchTier('Go to Settings', 'settings')).toBe(0)
  expect(matchTier('Go to Settings', '  Go   to settings ')).toBe(0)
  expect(matchTier('Go to Upcoming', 'up')).toBe(1)
})

test('exact and prefix matches rank above word-start and inside-word matches', () => {
  expect(matchTier('Replay tour', 'replay tour')).toBe(0)
  expect(matchTier('Replay tour', 'rep')).toBe(1)
  expect(matchTier('Create Task', 'task')).toBe(2)
  expect(matchTier('Keyboard shortcuts', 'board')).toBe(3)
  expect(matchTier('Keyboard shortcuts', 'sxt')).toBeNull()
})

test('a word inside a name matches only at a word start', () => {
  expect(matchTier('Create Task', 'ask')).toBe(3)
  expect(matchTier('Create Task', 'ta')).toBe(2)
})

test('the list order is kept within one tier, and a view is found by its name after the colon', () => {
  expect(names('task')).toEqual(['Create Task', 'Duplicate task'])
  expect(names('calendar')).toEqual(['Go to View: Calendar'])
})

test('a single letter or blank query finds no commands', () => {
  expect(matchCommands(list, 's')).toEqual([])
  expect(matchCommands(list, '  ')).toEqual([])
  expect(matchTier('Go to Settings', '')).toBeNull()
})

test('commands lead the results only for an exact or prefix title match', () => {
  expect(commandsFirst(matchCommands(list, 'settings'))).toBe(true)
  expect(commandsFirst(matchCommands(list, 'task'))).toBe(false)
  expect(commandsFirst([])).toBe(false)
})
