import { expect, test } from 'vitest'
import { DEFAULT_PREFS, knownJson, prefsFromSettings } from './captureKnown'

test('knownJson carries the names and prefs in the shape quick-add reads', () => {
  const json = JSON.parse(
    knownJson(
      { areas: ['Home'], goals: ['Get Fit'], pages: ['Plan'], labels: ['errand'] },
      { dates_off: true, next_week_day: 4, weekend_day: 6 },
    ),
  )
  expect(json).toEqual({
    areas: ['Home'],
    goals: ['Get Fit'],
    pages: ['Plan'],
    labels: ['errand'],
    prefs: { dates_off: true, next_week_day: 4, weekend_day: 6 },
  })
})

test('knownJson without prefs sends the defaults: dates on, days unset', () => {
  const json = JSON.parse(knownJson({ areas: [], goals: [], pages: [], labels: [] }))
  expect(json.prefs).toEqual({ dates_off: false, next_week_day: null, weekend_day: null })
  expect(json.prefs).toEqual(DEFAULT_PREFS)
})

test('prefsFromSettings turns smart dates off into dates_off and keeps the chosen days', () => {
  expect(prefsFromSettings({ smart_dates: false, next_week_day: 2, weekend_day: 4 })).toEqual({
    dates_off: true,
    next_week_day: 2,
    weekend_day: 4,
  })
  expect(prefsFromSettings({ smart_dates: true, next_week_day: 0, weekend_day: 5 })).toEqual({
    dates_off: false,
    next_week_day: 0,
    weekend_day: 5,
  })
})
