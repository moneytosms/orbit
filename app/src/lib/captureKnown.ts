import { getQuickAddSettings, type QuickAddSettings } from './sync'

/** The `prefs` quick-add reads from `Known`; a null day means the default (Monday for next week, Saturday for weekend). */
export type QuickAddPrefs = { dates_off: boolean; next_week_day: number | null; weekend_day: number | null }
/** The names quick-add resolves: Areas and Projects, Goals, Pages and Labels. */
export type CaptureLists = { areas: string[]; goals: string[]; pages: string[]; labels: string[] }

export const DEFAULT_PREFS: QuickAddPrefs = { dates_off: false, next_week_day: null, weekend_day: null }

/** The synced Settings as quick-add prefs; the day values stay explicit. */
export const prefsFromSettings = (s: QuickAddSettings): QuickAddPrefs => ({
  dates_off: !s.smart_dates,
  next_week_day: s.next_week_day,
  weekend_day: s.weekend_day,
})

/** The `known_json` for `parse_quick_add`: the names plus the prefs. */
export const knownJson = (lists: CaptureLists, prefs: QuickAddPrefs = DEFAULT_PREFS): string =>
  JSON.stringify({ areas: lists.areas, goals: lists.goals, pages: lists.pages, labels: lists.labels, prefs })

/** The quick-add prefs from the synced Settings; the defaults when the local store cannot be read. */
export async function quickAddPrefs(): Promise<QuickAddPrefs> {
  try {
    return prefsFromSettings(await getQuickAddSettings())
  } catch {
    return DEFAULT_PREFS
  }
}
