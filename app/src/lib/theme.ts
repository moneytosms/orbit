// Appearance, painted on <html>: an Orbit theme or a Monkeytype palette (inline tokens), plus accent,
// density, text size, typeface, line spacing and motion. Everything is kept per browser in localStorage. Theme and accent also sync
// through the Settings row: local changes go to `onAppearanceChange`, remote ones come in via `applyRemote`.
import { invoke } from '@tauri-apps/api/core'
import { isAndroidTauri } from './platform'
import init, { theme_list, theme_tokens } from '../../wasm/orbit_wasm'

export type Theme = 'light' | 'dark' | 'system'
export type Strategy = 'direct' | 'derived' | 'accent'
export type Choice = Theme | `mt:${string}:${Strategy}`
export type Density = 'comfortable' | 'compact'
export const TEXT_SIZES = ['90', '100', '115', '130', '150'] as const
export type TextSize = (typeof TEXT_SIZES)[number]
export const TYPEFACES = ['figtree', 'system', 'serif', 'mono'] as const
export type Typeface = (typeof TYPEFACES)[number]
export const LEADINGS = ['compact', 'default', 'relaxed'] as const
export type Leading = (typeof LEADINGS)[number]
export const MOTIONS = ['system', 'on', 'off'] as const
export type Motion = (typeof MOTIONS)[number]

export type PaletteInfo = {
  name: string
  grade: 'aaa' | 'aa' | 'low'
  offered: Strategy[]
  featured: boolean
  bg: string
  text: string
  main: string
}

/** The synced part of the appearance. Empty theme: not set; empty accent: the theme's own. */
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
// A stored value outside the list falls back to the default.
const pickOf = <T extends string>(key: string, list: readonly T[], dflt: T): T =>
  list.find((v) => v === stored(key)) ?? dflt
let textSize = pickOf<TextSize>('textSize', TEXT_SIZES, '100')
let typeface = pickOf<Typeface>('typeface', TYPEFACES, 'figtree')
let leading = pickOf<Leading>('leading', LEADINGS, 'default')
let motion = pickOf<Motion>('motion', MOTIONS, 'system')

// The palette engine loads with the app's own init(); this is the same promise, so importing the module never loads wasm.
let wasmReady: Promise<boolean> | undefined
const loaded = () =>
  (wasmReady ??= init().then(
    () => true,
    () => false,
  ))

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

/** Android: status/nav bar icons follow the page theme. An older shell lacks the command; ignored. */
const barIcons = (dark: boolean) => {
  if (isAndroidTauri()) void invoke('set_bar_theme', { dark }).catch(() => {})
}

function paint() {
  const root = globalThis.document?.documentElement
  if (!root) return
  const style = root.style
  root.dataset.density = density
  // zoom, not rem: the UI is full of px literals. Applied to <html> so fixed layers scale too.
  style.zoom = textSize === '100' ? '' : String(+textSize / 100)
  root.dataset.text = textSize
  root.dataset.font = typeface
  root.dataset.leading = leading
  root.dataset.motion = motion

  if (!current.startsWith('mt:')) {
    clearTokens(style)
    const dark = current === 'dark' || (current === 'system' && (media?.matches ?? false))
    root.dataset.theme = dark ? 'dark' : 'light'
    barIcons(dark)
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
    barIcons(root.dataset.theme === 'dark')
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

/** Applies a theme and accent that another Device synced. An empty theme keeps the local one; an empty accent clears it. Does not sync back. */
export function applyRemote({ theme, accent: remoteAccent }: Appearance) {
  if (theme) {
    current = theme as Choice
    save('theme', theme)
  }
  if (remoteAccent !== undefined) {
    accent = remoteAccent || null
    save('accent', accent)
  }
  paint()
}

export const getDensity = (): Density => density
export function setDensity(d: Density) {
  density = d
  save('density', d)
  paint()
}

export const getTextSize = (): TextSize => textSize
export function setTextSize(v: TextSize) {
  textSize = v
  save('textSize', v)
  paint()
}

export const getTypeface = (): Typeface => typeface
export function setTypeface(v: Typeface) {
  typeface = v
  save('typeface', v)
  paint()
}

export const getLeading = (): Leading => leading
export function setLeading(v: Leading) {
  leading = v
  save('leading', v)
  paint()
}

export const getMotion = (): Motion => motion
export function setMotion(v: Motion) {
  motion = v
  save('motion', v)
  paint()
}

/** Text size, typeface, line spacing and motion back to their defaults (not theme, accent or density). */
export function resetAppearance() {
  ;[textSize, typeface, leading, motion] = ['100', 'figtree', 'default', 'system']
  for (const k of ['textSize', 'typeface', 'leading', 'motion']) save(k, null)
  paint()
}
