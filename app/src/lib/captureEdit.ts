// Edits quick-add text from the visual capture bar. The typed line stays the source of truth:
// every pick rewrites the same syntax the parser reads (`@Area`, `#label`, `p1`, `!30m`, a date),
// so what you click and what you type are always the same thing.
import type { Parsed } from './sync'

type Token = Parsed['tokens'][number]
export type Kind = Token['kind']

const enc = new TextEncoder()
const dec = new TextDecoder()

// Token offsets are UTF-8 bytes (Rust), the draft is a UTF-16 string.
function cut(draft: string, start: number, end: number, insert: string): string {
  const bytes = enc.encode(draft)
  return dec.decode(bytes.slice(0, start)) + insert + dec.decode(bytes.slice(end))
}

const tidy = (s: string) => s.replace(/ {2,}/g, ' ').trim()

/** Replace every token of `kind` with `text` (at the first one's place), or append when there is none. An empty `text` removes them. */
export function setToken(draft: string, tokens: Token[], kind: Kind, text: string): string {
  const mine = tokens.filter((t) => t.kind === kind).sort((a, b) => b.start - a.start)
  if (!mine.length) return text ? tidy(`${draft} ${text}`) : draft
  let out = draft
  mine.forEach((t, i) => {
    // Walking back to front keeps earlier offsets valid; the first token in the line takes the text.
    out = cut(out, t.start, t.end, i === mine.length - 1 ? text : '')
  })
  return tidy(out)
}

/** Add `text` as a `kind` token, or remove the token whose value is `value` when it is already there. */
export function toggleToken(draft: string, tokens: Token[], kind: Kind, value: string, text: string): string {
  const hit = tokens.find((t) => t.kind === kind && t.value.toLowerCase() === value.toLowerCase())
  if (!hit) return tidy(`${draft} ${text}`)
  return tidy(cut(draft, hit.start, hit.end, ''))
}

/** A due date as typed text: the ISO date, then the time when there is one. */
export const dateText = (date: string, time?: string) => (time ? `${date} ${time}` : date)

/** Quick picks that stay natural language so the line reads the way you would say it. */
export const dateQuick = [
  { label: 'Today', text: 'today' },
  { label: 'Tomorrow', text: 'tomorrow' },
  { label: 'This weekend', text: 'this weekend' },
  { label: 'Next week', text: 'next week' },
] as const

/** A deadline as typed text: `{<date phrase>}`. Dates from the calendar are `{YYYY-MM-DD}`. */
export const deadlineText = (phrase: string) => `{${phrase}}`

/** Natural-language quick picks for the deadline (the braces are added by `deadlineText`). */
export const deadlineQuick = [
  { label: 'Today', text: 'today' },
  { label: 'Tomorrow', text: 'tomorrow' },
  { label: 'This weekend', text: 'this weekend' },
  { label: 'Next week', text: 'next week' },
  { label: 'End of month', text: 'end of month' },
] as const

/** `2026-12-05` as `Sat 5 Dec`. */
export const deadlineLabel = (date: string) => dueLabel({ date, time: null })

/** A duration as typed text the parser reads back: `for 45m`, `for 1h`, `for 1h30m`. */
export function durationText(minutes: number): string {
  const h = Math.floor(minutes / 60)
  const m = minutes % 60
  return `for ${h ? `${h}h` : ''}${m || !h ? `${m}m` : ''}`
}

/** `90` as `1h 30m`, `45` as `45m`, `120` as `2h`. */
export function durationLabel(minutes: number): string {
  const h = Math.floor(minutes / 60)
  const m = minutes % 60
  return [h ? `${h}h` : '', m || !h ? `${m}m` : ''].filter(Boolean).join(' ')
}

export const durationQuick = [
  { label: '15m', minutes: 15 },
  { label: '30m', minutes: 30 },
  { label: '1h', minutes: 60 },
  { label: '2h', minutes: 120 },
] as const

export const reminderQuick = [
  { label: '10 min before', minutes: '10', text: '!10m' },
  { label: '30 min before', minutes: '30', text: '!30m' },
  { label: '1 hour before', minutes: '60', text: '!1h' },
  { label: '1 day before', minutes: '1440', text: '!1d' },
] as const

/** `2026-11-20` and `18:00` as a short label like `Fri 20 Nov, 18:00`. */
export function dueLabel(due: { date: string; time: string | null }): string {
  const [y, m, d] = due.date.split('-').map(Number)
  const day = new Date(Date.UTC(y, m - 1, d)).toLocaleDateString('en-GB', {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    timeZone: 'UTC',
  })
  return due.time ? `${day}, ${due.time}` : day
}

const iso = (ms: number) => new Date(ms).toISOString().slice(0, 10)

/** Six weeks of days (Monday first) covering month `m` (1-12) of `y`, for a month picker. */
export function monthGrid(y: number, m: number): { date: string; inMonth: boolean }[] {
  const first = Date.UTC(y, m - 1, 1)
  const lead = (new Date(first).getUTCDay() + 6) % 7
  return Array.from({ length: 42 }, (_, i) => {
    const ms = first + (i - lead) * 86_400_000
    return { date: iso(ms), inMonth: new Date(ms).getUTCMonth() === m - 1 }
  })
}

/** The month `delta` months from `y`/`m`, as `{y, m}` with `m` 1-12. */
export function shiftMonth(y: number, m: number, delta: number): { y: number; m: number } {
  const idx = y * 12 + (m - 1) + delta
  return { y: Math.floor(idx / 12), m: (((idx % 12) + 12) % 12) + 1 }
}
