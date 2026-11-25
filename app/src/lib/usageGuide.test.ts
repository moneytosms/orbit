import { beforeAll, expect, test } from 'vitest'
import { readFileSync } from 'node:fs'
import { initSync, parse_quick_add } from '../../wasm/orbit_wasm'
import { GUIDE_KNOWN, GUIDE_NOW, guide } from './usageGuide'
import type { Parsed } from './sync'

beforeAll(() => {
  initSync({ module: readFileSync(new URL('../../wasm/orbit_wasm_bg.wasm', import.meta.url)) })
})

const parse = (text: string): Parsed => JSON.parse(parse_quick_add(text, GUIDE_NOW, JSON.stringify(GUIDE_KNOWN)))

for (const group of guide) {
  for (const ex of group.examples) {
    test(`guide: ${ex.text}`, () => {
      const p = parse(ex.text) as Parsed & { deadline?: string | null; duration_min?: number | null }
      const e = ex.expect
      if (e.title !== undefined) expect(p.title).toBe(e.title)
      if (e.due !== undefined) expect(p.due ? `${p.due.date}${p.due.time ? ' ' + p.due.time : ''}` : null).toBe(e.due)
      if (e.deadline !== undefined) expect(p.deadline).toBe(e.deadline)
      if (e.duration !== undefined) expect(p.duration_min).toBe(e.duration)
      if (e.priority !== undefined) expect(p.priority).toBe(e.priority)
      if (e.place !== undefined) expect(p.place).toBe(e.place)
      if (e.goal !== undefined) expect(p.goal).toBe(e.goal)
      if (e.page !== undefined) expect(p.page).toBe(e.page)
      if (e.labels !== undefined) expect(p.labels).toEqual(e.labels)
      if (e.recurrence !== undefined) {
        const r = p.recurrence
        expect(r ? `${r.rule}${r.end ? ' end ' + r.end : ''}` : null).toBe(e.recurrence)
      }
      if (e.reminders !== undefined) expect(p.reminders.length).toBe(e.reminders)
    })
  }
}
