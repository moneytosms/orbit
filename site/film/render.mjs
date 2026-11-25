// Renders film.html frame by frame (no dropped frames, no real-time capture) and encodes it.
//   node film/render.mjs                 -> public/clips/orbit-film.{mp4,webm} + poster
//   node film/render.mjs --stills 5,9.5  -> film/out/still-<t>.jpg, for checking single moments
//   FPS=30 node film/render.mjs          -> a quicker draft
import { chromium } from 'playwright'
import { spawn } from 'node:child_process'
import { createReadStream, existsSync, mkdirSync, statSync } from 'node:fs'
import { createServer } from 'node:http'
import { dirname, extname, join, normalize } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const site = join(here, '..')
const OUT = join(here, 'out')
const CLIPS = join(site, 'public/clips')
mkdirSync(OUT, { recursive: true })
const FPS = Number(process.env.FPS ?? 60)
const args = process.argv.slice(2)
const stills = args[0] === '--stills' ? args[1].split(',').map(Number) : null

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.jpg': 'image/jpeg', '.woff2': 'font/woff2', '.svg': 'image/svg+xml' }
const server = createServer((req, res) => {
  const path = normalize(join(site, decodeURIComponent(new URL(req.url, 'http://x').pathname)))
  if (!path.startsWith(site) || !existsSync(path) || statSync(path).isDirectory()) return res.writeHead(404).end()
  res.writeHead(200, { 'content-type': types[extname(path)] ?? 'application/octet-stream' })
  createReadStream(path).pipe(res)
}).listen(0, '127.0.0.1')
await new Promise((r) => server.once('listening', r))
const url = `http://127.0.0.1:${server.address().port}/film/film.html`

const browser = await chromium.launch({ args: ['--disable-gpu-vsync', '--force-color-profile=srgb'] })
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 })
page.on('pageerror', (e) => console.error('pageerror', e.message))
page.on('console', (m) => m.type() === 'error' && console.error('console', m.text()))
await page.goto(url)
await page.evaluate(() => window.__film.ready)
const duration = await page.evaluate(() => window.__film.duration)
const frame = async (t) => {
  await page.evaluate((t) => window.__film.seek(t), t)
  return page.screenshot({ type: 'jpeg', quality: 96 })
}

if (stills) {
  const { writeFileSync } = await import('node:fs')
  for (const t of stills) writeFileSync(join(OUT, `still-${t}.jpg`), await frame(t))
  console.log('stills', stills.join(' '))
} else {
  const master = join(OUT, `orbit-film-${FPS}.mp4`)
  // High-quality intermediate; the web encodes below come from it.
  const ff = spawn('ffmpeg', ['-v', 'error', '-y', '-f', 'image2pipe', '-c:v', 'mjpeg', '-framerate', String(FPS), '-i', '-',
    '-c:v', 'libx264', '-preset', 'slow', '-crf', '12', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', master], { stdio: ['pipe', 'inherit', 'inherit'] })
  const total = Math.round(duration * FPS)
  const t0 = Date.now()
  for (let i = 0; i < total; i++) {
    const buf = await frame(i / FPS)
    if (!ff.stdin.write(buf)) await new Promise((r) => ff.stdin.once('drain', r))
    if (i % 300 === 0) console.log(`frame ${i}/${total}  ${((Date.now() - t0) / 1000).toFixed(0)}s`)
  }
  ff.stdin.end()
  await new Promise((r) => ff.on('close', r))
  const run = (a) => new Promise((ok, no) => spawn('ffmpeg', ['-v', 'error', '-y', ...a], { stdio: 'inherit' }).on('close', (c) => (c ? no(new Error(`ffmpeg ${c}`)) : ok())))
  mkdirSync(CLIPS, { recursive: true })
  await run(['-i', master, '-c:v', 'libx264', '-preset', 'veryslow', '-crf', '21', '-profile:v', 'high', '-pix_fmt', 'yuv420p', '-tune', 'film', '-movflags', '+faststart', '-an', join(CLIPS, 'orbit-film.mp4')])
  await run(['-i', master, '-c:v', 'libsvtav1', '-preset', '5', '-crf', '34', '-g', '240', '-pix_fmt', 'yuv420p', '-an', join(CLIPS, 'orbit-film.webm')])
  await run(['-ss', '73.6', '-i', master, '-frames:v', '1', '-vf', 'scale=1920:-1', '-c:v', 'libwebp', '-quality', '86', join(CLIPS, 'orbit-film-poster.webp')])
  console.log('rendered', total, 'frames in', ((Date.now() - t0) / 1000).toFixed(0), 's')
}
await browser.close()
server.close()
