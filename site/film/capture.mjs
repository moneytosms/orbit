// Records the film's footage from the real Orbit web app: 2x stills, one frame per keystroke for
// typing, before/after frames for clicks, and the on-screen boxes the film's cursor aims at.
// Needs the demo dev server: (cd app && VITE_DEMO=1 pnpm exec vite --port 5199)
// Usage: node film/capture.mjs   ->   film/assets/{*.jpg,manifest.json}
import { chromium } from 'playwright'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { APP, openSeeded } from './app.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const OUT = join(here, 'assets')
rmSync(OUT, { recursive: true, force: true })
mkdirSync(OUT, { recursive: true })
const manifest = { desktop: { w: 1440, h: 900 }, phone: { w: 390, h: 844 }, seq: {}, box: {} }

const browser = await chromium.launch()

async function settle(page, ms = 500) {
  await page.waitForLoadState('networkidle')
  await page.evaluate(() => document.fonts.ready)
  await page.waitForTimeout(ms)
}
const shot = async (page, name) => {
  await page.mouse.move(-10, -10)
  await page.screenshot({ path: join(OUT, `${name}.jpg`), type: 'jpeg', quality: 94 })
  console.log('shot', name)
}
const go = async (page, route, ms) => {
  await page.goto(`${APP}/#/${route}`)
  await settle(page, ms)
}
/** Box of the round check control on the row whose text starts with `title`. */
async function checkBox(page, title) {
  return page.evaluate((title) => {
    const el = [...document.querySelectorAll('main *')].find(
      (e) => e.children.length === 0 && e.textContent.trim() === title && e.getClientRects().length,
    )
    let row = el
    for (let i = 0; i < 6 && row; i++, row = row.parentElement) {
      const c = row.querySelector('button, [role=checkbox], input[type=checkbox]')
      if (c && c !== el) {
        const r = c.getBoundingClientRect()
        return { x: r.x, y: r.y, w: r.width, h: r.height }
      }
    }
    return null
  }, title)
}
/** Box of the nearest ancestor of the element reading `text` that is at least `minH` tall. */
async function card(page, text, minH, maxH = 2000) {
  return page.evaluate(([text, minH, maxH]) => {
    const el = [...document.querySelectorAll('body *')].find(
      (e) => e.children.length === 0 && e.textContent.trim().startsWith(text) && e.getClientRects().length,
    )
    for (let n = el; n; n = n.parentElement) {
      const r = n.getBoundingClientRect()
      if (r.height >= minH && r.height <= maxH) return { x: r.x, y: r.y, w: r.width, h: r.height }
    }
    return null
  }, [text, minH, maxH])
}
const box = async (loc) => {
  const b = await loc.boundingBox()
  return b && { x: b.x, y: b.y, w: b.width, h: b.height }
}

// ---- Desktop, dark ---------------------------------------------------------
{
  const { context, page, ids } = await openSeeded(browser, { viewport: { width: 1440, height: 900 } })

  await go(page, 'home', 900)
  await shot(page, 'home')
  manifest.box.glance = await card(page, 'Life at a glance', 150)
  manifest.box.next = await card(page, "What's next", 200)
  manifest.box.homeRoutines = await card(page, 'Routines', 100)
  manifest.box.homeCheck = await checkBox(page, 'Pay rent')
  await page.mouse.click(manifest.box.homeCheck.x + manifest.box.homeCheck.w / 2, manifest.box.homeCheck.y + manifest.box.homeCheck.h / 2)
  await page.waitForTimeout(250)
  await shot(page, 'home-checked')
  await settle(page, 1600)
  await shot(page, 'home-after')

  // Quick add: one frame per keystroke.
  await go(page, `area/${ids.home}`, 300)
  await page.reload()
  await settle(page, 900)
  await shot(page, 'qa-empty')
  const add = page.getByPlaceholder('Add a task').first()
  manifest.box.qaInput = await box(add)
  await add.click()
  await page.waitForTimeout(250)
  const text = 'Renew passport friday 10am p1 @Home'
  manifest.seq.qa = { text, frames: text.length + 1 }
  await shot(page, 'qa-00')
  for (let i = 0; i < text.length; i++) {
    await page.keyboard.type(text[i])
    await page.waitForTimeout(110)
    await shot(page, `qa-${String(i + 1).padStart(2, '0')}`)
  }
  manifest.box.composer = await card(page, 'Renew passport', 70)
  await page.keyboard.press('Enter')
  await settle(page, 700)
  await shot(page, 'qa-added')
  manifest.box.upNext = await card(page, 'Up next', 80, 320)

  await go(page, `project/${ids.hunt}`, 500)
  await page.getByRole('button', { name: 'Board', exact: true }).first().click()
  await settle(page, 700)
  await shot(page, 'board')
  manifest.box.board = await card(page, 'To apply', 300)

  await go(page, 'goals', 600)
  await page.evaluate(() => {
    const h = [...document.querySelectorAll('main h2, main h3, main div')].find((e) => e.textContent.trim() === 'Career')
    const main = document.querySelector('main')
    const sc = [...document.querySelectorAll('main, main *')].find((e) => e.scrollHeight > e.clientHeight + 20 && getComputedStyle(e).overflowY !== 'visible') ?? document.scrollingElement
    sc.scrollTop += h.getBoundingClientRect().top - 70
    void main
  })
  await settle(page, 500)
  await shot(page, 'goals')
  manifest.box.goalCard = await card(page, 'Land a summer internship', 180)
  manifest.box.goalThesis = await card(page, 'Submit the thesis', 180)

  await go(page, 'routines', 700)
  await shot(page, 'routines')
  manifest.box.routineCard = await card(page, 'Read 20 pages', 60)
  manifest.box.routineList = await card(page, 'Meditate', 400)
  manifest.box.routineCheck = await checkBox(page, 'Read 20 pages')
  await page.mouse.click(manifest.box.routineCheck.x + manifest.box.routineCheck.w / 2, manifest.box.routineCheck.y + manifest.box.routineCheck.h / 2)
  await page.waitForTimeout(250)
  await shot(page, 'routines-checked')
  await settle(page, 1200)
  await shot(page, 'routines-after')

  await go(page, `roadmap/${ids.map}`, 500)
  await page.getByRole('button', { name: 'Timeline', exact: true }).first().click()
  await settle(page, 800)
  await shot(page, 'roadmap')
  manifest.box.timeline = await card(page, 'Offer & move', 250)
  await page.getByRole('button', { name: 'Board', exact: true }).first().click()
  await settle(page, 600)
  await shot(page, 'roadmap-board')

  await go(page, 'upcoming', 500)
  await shot(page, 'upcoming')
  await page.getByRole('button', { name: 'Calendar', exact: true }).first().click()
  await settle(page, 2500)
  await shot(page, 'calendar')

  // Command palette: type a query, then switch theme from it.
  await go(page, 'home', 800)
  await page.keyboard.press('Control+k')
  await page.waitForTimeout(400)
  const q = '>theme'
  manifest.seq.pal = { text: q, frames: q.length + 1 }
  await shot(page, 'pal-00')
  manifest.box.palette = await card(page, 'Tasks', 200)
  for (let i = 0; i < q.length; i++) {
    await page.keyboard.type(q[i])
    await page.waitForTimeout(140)
    await shot(page, `pal-${String(i + 1).padStart(2, '0')}`)
  }
  manifest.box.paletteTyped = await card(page, 'Toggle theme', 60)
  await page.keyboard.press('Enter')
  await settle(page, 1200)
  await shot(page, 'home-light')
  await context.close()
}

// ---- Phone, dark -----------------------------------------------------------
{
  const { context, page } = await openSeeded(browser, { viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, isMobile: true })
  for (const r of ['home', 'upcoming', 'routines', 'goals']) {
    await go(page, r, 900)
    await shot(page, `phone-${r}`)
  }
  await context.close()
}

writeFileSync(join(OUT, 'manifest.json'), JSON.stringify(manifest, null, 2))
await browser.close()
console.log('done', manifest)
