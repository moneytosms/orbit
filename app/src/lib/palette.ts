/** Lower-cases and collapses spaces, so the typed query and a command name compare the same way. */
const norm = (s: string) => s.trim().toLowerCase().replace(/\s+/g, ' ')

const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

/** 0 exact, 1 prefix, 2 a word starts with the query, 3 inside a word; null when it does not match. */
function tierOf(title: string, q: string): number | null {
  if (title === q) return 0
  if (title.startsWith(q)) return 1
  if (new RegExp(`(^|[^\\p{L}\\p{N}])${escapeRe(q)}`, 'u').test(title)) return 2
  if (title.includes(q)) return 3
  return null
}

/**
 * How well a command name matches the typed query: 0 exact, 1 prefix, 2 a word starts with it, 3 inside a word,
 * or null. "Go to " is not part of what people type, so "settings" matches "Go to Settings" as an exact title.
 */
export function matchTier(name: string, query: string): number | null {
  const q = norm(query)
  if (!q) return null
  const t = norm(name)
  const bare = t.replace(/^go to /, '')
  const tiers = [tierOf(t, q), tierOf(bare, q)].filter((x): x is number => x !== null)
  return tiers.length ? Math.min(...tiers) : null
}

/**
 * The commands an unprefixed query also finds, best match first (the list order is kept within a tier).
 * One letter matches too many names to be useful, so it finds none.
 */
export function matchCommands<T extends { name: string }>(list: T[], query: string): { item: T; tier: number }[] {
  if (norm(query).length < 2) return []
  return list
    .flatMap((item) => {
      const tier = matchTier(item.name, query)
      return tier === null ? [] : [{ item, tier }]
    })
    .sort((a, b) => a.tier - b.tier)
}

/** Commands go above the task and project results only when one of them is an exact or prefix title match. */
export function commandsFirst(matches: { tier: number }[]): boolean {
  return matches.length > 0 && matches[0].tier <= 1
}
