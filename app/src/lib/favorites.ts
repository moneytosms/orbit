import { writable } from 'svelte/store'

/** Where a favorite links to and how it is labelled in the sidebar. */
export type FavoriteEntry = { id: string; kind: 'area' | 'project' | 'view' | 'goal'; name: string; href: string }

export const FAVORITES_KEY = 'orbit.favorites'

/** Stored ids from localStorage's text; anything unreadable is no favorites. */
export function parseFavorites(raw: string | null): string[] {
  try {
    const value: unknown = JSON.parse(raw ?? '[]')
    return Array.isArray(value) ? value.filter((x): x is string => typeof x === 'string') : []
  } catch {
    return []
  }
}

/** Adds `id` to the end of the list, or removes it when already there. */
export const toggleIn = (list: string[], id: string) =>
  list.includes(id) ? list.filter((x) => x !== id) : [...list, id]

/** The favorites that still exist, in the order they were starred. */
export function resolveFavorites(ids: string[], lookup: Map<string, FavoriteEntry>): FavoriteEntry[] {
  return ids.flatMap((id) => {
    const entry = lookup.get(id)
    return entry ? [entry] : []
  })
}

const read = (): string[] => {
  try {
    return parseFavorites(localStorage.getItem(FAVORITES_KEY))
  } catch {
    // Storage can be blocked (private windows, cleared site data): favorites then last for this page only.
    return []
  }
}

/** The favorite ids on this device, in the order they were starred. */
export const favorites = writable<string[]>(read())

/** Stars or unstars an Area, Project, saved View or Goal. Stored on this device only. */
export function toggleFavorite(id: string) {
  favorites.update((list) => {
    const next = toggleIn(list, id)
    try {
      localStorage.setItem(FAVORITES_KEY, JSON.stringify(next))
    } catch {
      // Keep the choice for this page even when it cannot be saved.
    }
    return next
  })
}
