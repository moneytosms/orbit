/** Badge art: a tier body (meteorite, comet, planet, star, black hole) with a metric glyph on it. Ids match crates/orbit-core/src/badge.rs. */
export const TIER_COLOR: Record<string, string> = {
  meteorite: '#B07A56',
  comet: '#3E9FB0',
  planet: '#5B9A6A',
  star: '#E26D4F',
  blackhole: '#2A2830',
}

export const tierKey = (tier: string) => tier.toLowerCase().replace(/[^a-z]/g, '')

export function metricOf(id: string): string {
  if (id === 'universe') return 'stardust'
  if (id === 'inbox-zero') return 'inbox'
  const head = id.split('-')[0]
  return head === 'busy' ? 'busy' : head
}

/** 24-unit glyph paths, stroked white. */
export const GLYPHS: Record<string, string> = {
  tasks: '<path d="M5.5 12.5l4.5 4.5 8.5-9.5"/>',
  checkins: '<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="2.6" fill="white" stroke="none"/>',
  streak: '<path d="M12 3c.8 4 5 5.5 5 10a5 5 0 0 1-10 0c0-2 1-3.2 2-4 0 2 .8 3 2 3 .2-3-1-5 1-9z"/>',
  busy: '<path d="M13.5 3L6 13h5.2l-1 8L18 11h-5.2z"/>',
  milestones: '<path d="M7 21V4M7 5h10.5l-2.5 4 2.5 4H7"/>',
  goals:
    '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="5"/><circle cx="12" cy="12" r="1.4" fill="white" stroke="none"/>',
  patches:
    '<rect x="4.5" y="4.5" width="15" height="15" rx="3.5" stroke-dasharray="2.2 2.4"/><path d="M9 12h6M12 9v6"/>',
  stardust: '<path d="M12 3l2.2 6.8L21 12l-6.8 2.2L12 21l-2.2-6.8L3 12l6.8-2.2z"/>',
  inbox: '<path d="M4 13l3-8h10l3 8v6H4zM4 13h4.5l1.5 2.5h4l1.5-2.5H20"/>',
}

export function starPoints(points: number, outer: number, inner: number): string {
  return Array.from({ length: points * 2 }, (_, i) => {
    const r = i % 2 ? inner : outer
    const a = (Math.PI * i) / points - Math.PI / 2
    return `${(32 + r * Math.cos(a)).toFixed(1)},${(32 + r * Math.sin(a)).toFixed(1)}`
  }).join(' ')
}
