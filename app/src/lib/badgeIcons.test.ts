import { expect, test } from 'vitest'
import { GLYPHS, TIER_COLOR, metricOf, tierKey } from './badgeIcons'

// Every catalog id in crates/orbit-core/src/badge.rs.
const IDS = [
  'tasks-10',
  'tasks-100',
  'tasks-1000',
  'tasks-5000',
  'tasks-20000',
  'checkins-10',
  'checkins-100',
  'checkins-500',
  'streak-7',
  'streak-30',
  'streak-100',
  'streak-365',
  'busy-10',
  'busy-25',
  'milestones-1',
  'milestones-10',
  'milestones-50',
  'goals-1',
  'goals-5',
  'goals-25',
  'patches-1',
  'patches-5',
  'universe',
  'inbox-zero',
]

test('every badge id has a glyph', () => {
  for (const id of IDS) expect(GLYPHS[metricOf(id)], id).toBeTruthy()
})

test('every tier name has a color', () => {
  for (const t of ['Meteorite', 'Comet', 'Planet', 'Star', 'Black Hole']) expect(TIER_COLOR[tierKey(t)], t).toBeTruthy()
})
