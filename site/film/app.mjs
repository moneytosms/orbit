// Opens the Orbit web demo (Vite dev server, VITE_DEMO=1) seeded with the film's made-up planner.
// Shared by capture.mjs (stills + keystroke sequences) and the site screenshots.
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
export const APP = process.env.APP_URL ?? 'http://127.0.0.1:5199'
const seedSource = readFileSync(join(here, 'seed.js'), 'utf8').replace('export async function', 'async function')

/** A context + page on a freshly seeded demo. `theme` is 'light' | 'dark'. */
export async function openSeeded(browser, { viewport, deviceScaleFactor = 2, theme = 'dark', isMobile = false } = {}) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor,
    isMobile,
    hasTouch: isMobile,
    colorScheme: theme,
    reducedMotion: 'no-preference',
    timezoneId: 'Europe/London',
    locale: 'en-GB',
  })
  await context.addInitScript(() => {
    // Skip the demo's own small seed (ours replaces it) and hide the demo banner.
    localStorage.setItem('orbit-demo-seeded', '1')
    new MutationObserver(() => {
      for (const el of document.body?.children ?? [])
        if (el.textContent?.startsWith('Demo: nothing leaves')) el.remove()
    }).observe(document, { childList: true, subtree: true })
  })
  const page = await context.newPage()
  page.on('pageerror', (e) => console.error('pageerror', e.message))
  await page.goto(APP + '/#/home')
  await page.waitForLoadState('networkidle')
  const ids = await page.evaluate(`(${seedSource.match(/async function seedOrbit[\s\S]*/)[0]})()`.replace('async function seedOrbit', 'async function'))
  const s = await page.evaluate(async (theme) => {
    const sync = await import('/src/lib/sync.ts')
    await sync.updateSettings({ theme })
    return true
  }, theme)
  await page.reload()
  await page.waitForLoadState('networkidle')
  await page.waitForTimeout(800)
  return { context, page, ids, s }
}
