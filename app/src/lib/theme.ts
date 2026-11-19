// Appearance, painted on <html>: an Orbit theme or a Monkeytype palette (inline tokens), plus accent,
// density and font size. Everything is kept per browser in localStorage. Theme and accent also sync
// through the Settings row: local changes go to `onAppearanceChange`, remote ones come in via `applyRemote`.
import init, { theme_list, theme_tokens } from '../../wasm/orbit_wasm'

export type Theme = 'light' | 'dark' | 'system'
export type Strategy = 'direct' | 'derived' | 'accent'
export type Choice = Theme | `mt:${string}:${Strategy}`
export type Density = 'comfortable' | 'compact'
export type FontSize = 's' | 'm' | 'l'

export type PaletteInfo = {
  name: string
  grade: 'aaa' | 'aa' | 'low'
  offered: Strategy[]
  featured: boolean
  bg: string
  text: string
  main: string
}

const FONT_PX: Record<FontSize, string> = { s: '14px', m: '15px', l: '16px' }

/** The synced part of the appearance; an empty string means not set. */
export type Appearance = { theme?: string; accent?: string }
let onChange: (p: Appearance) => void = () => {}
/** Called with the theme or accent whenever this Device changes it (not for `applyRemote`). */
export const onAppearanceChange = (fn: (p: Appearance) => void) => (onChange = fn)

function stored(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    // Storage blocked: use the default.
    return null
  }
}

function save(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key)
    else localStorage.setItem(key, value)
  } catch {
    // Not remembered; still applied for this visit.
  }
}

const media = (() => {
  try {
    return matchMedia('(prefers-color-scheme: dark)')
  } catch {
    return undefined
  }
})()

let current: Choice = (stored('theme') as Choice | null) ?? 'system'
let accent: string | null = stored('accent')
let density: Density = stored('density') === 'compact' ? 'compact' : 'comfortable'
let fontSize: FontSize = (['s', 'm', 'l'] as const).find((f) => f === stored('fontSize')) ?? 'm'

// The palette engine loads with the app's own init(); this is the same promise, so importing the module never loads wasm.
let wasmReady: Promise<boolean> | undefined
const loaded = () => (wasmReady ??= init().then(() => true, () => false))

/** Relative luminance of `#rgb` or `#rrggbb`, 0 to 1. */
function luminance(hex: string): number {
  const h = hex.slice(1)
  const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h
  const [r, g, b] = [0, 2, 4].map((i) => {
    const v = parseInt(full.slice(i, i + 2), 16) / 255
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

function clearTokens(style: CSSStyleDeclaration) {
  // Palette tokens are app.css var names; drop any previous set before painting the next.
  for (let i = style.length - 1; i >= 0; i--) {
    const name = style[i]
    if (name.startsWith('--')) style.removeProperty(name)
  }
}

function applyAccent(style: CSSStyleDeclaration) {
  if (!accent) return
  style.setProperty('--accent', accent)
  style.setProperty('--accent-text', accent)
  style.setProperty('--accent-soft', `color-mix(in srgb, ${accent} 14%, var(--bg))`)
  style.setProperty('--accent-line', `color-mix(in srgb, ${accent} 45%, var(--bg))`)
}

function paint() {
  const root = globalThis.document?.documentElement
  if (!root) return
  const style = root.style
  root.dataset.density = density
  style.fontSize = FONT_PX[fontSize]

  if (!current.startsWith('mt:')) {
    clearTokens(style)
    const dark = current === 'dark' || (current === 'system' && (media?.matches ?? false))
    root.dataset.theme = dark ? 'dark' : 'light'
    applyAccent(style)
    return
  }

  const [, name, strategy] = current.split(':')
  const choice = current
  loaded().then((ok) => {
    // Skip if the user has picked something else while the palette was loading.
    if (!ok || choice !== current) return
    const tokens: Record<string, string> = JSON.parse(theme_tokens(name, strategy))
    clearTokens(style)
    for (const [key, value] of Object.entries(tokens)) style.setProperty(`--${key}`, value)
    root.dataset.theme = luminance(tokens.bg) <= 0.18 ? 'dark' : 'light'
    applyAccent(style)
  })
}

media?.addEventListener('change', paint)
paint()

/** The palettes, with grades and offered strategies. Call only after the app's wasm init. */
export const paletteList = (): PaletteInfo[] => JSON.parse(theme_list())

export const getTheme = (): Choice => current
export function setTheme(t: Choice) {
  current = t
  save('theme', t)
  paint()
  onChange({ theme: t })
}

export const getAccent = (): string | null => accent
/** `null` clears the accent back to the theme's own. */
export function setAccent(hex: string | null) {
  accent = hex
  save('accent', hex)
  paint()
  onChange({ accent: hex ?? '' })
}

/** Applies a theme and accent that another Device synced. Empty values keep the local ones. Does not sync back. */
export function applyRemote({ theme, accent: remoteAccent }: Appearance) {
  if (theme) {
    current = theme as Choice
    save('theme', theme)
  }
  if (remoteAccent) {
    accent = remoteAccent
    save('accent', remoteAccent)
  }
  paint()
}

export const getDensity = (): Density => density
export function setDensity(d: Density) {
  density = d
  save('density', d)
  paint()
}

export const getFontSize = (): FontSize => fontSize
export function setFontSize(f: FontSize) {
  fontSize = f
  save('fontSize', f)
  paint()
}
