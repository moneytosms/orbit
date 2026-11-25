// The film's timeline. Everything is a tween on one paused GSAP timeline plus a few pure functions of
// time (stars, planets, grain), so seek(t) always draws the same frame. render.mjs calls __film.seek.
/* global gsap */
const DURATION = 76
const $ = (s) => document.querySelector(s)
const $$ = (s) => [...document.querySelectorAll(s)]

// ---- Deterministic sky ------------------------------------------------------
let seed = 11
const rnd = () => (seed = (seed * 16807) % 2147483647) / 2147483647
const stars = Array.from({ length: 220 }, () => {
  const i = document.createElement('i')
  const size = 0.6 + rnd() ** 3 * 2.6
  Object.assign(i.style, { left: `${rnd() * 100}%`, top: `${rnd() * 100}%`, width: `${size}px`, height: `${size}px` })
  $('#stars').append(i)
  return { el: i, base: 0.25 + rnd() * 0.75, speed: 0.6 + rnd() * 2.2, phase: rnd() * 6.28, depth: 0.3 + rnd() }
})
const rings = $$('.ring')
const planets = $$('.planet')
const orbits = [
  [330, 120, 0.34, 0.4],
  [520, 190, 0.22, 2.2],
  [740, 270, 0.15, 4.1],
  [980, 360, 0.1, 5.3],
]
for (const r of rings) {
  const len = r.getTotalLength()
  r.style.strokeDasharray = `${len}`
  r.style.strokeDashoffset = `${len}`
}
const logoRing = $('#logoring')
logoRing.style.strokeDasharray = `${logoRing.getTotalLength()}`
logoRing.style.strokeDashoffset = `${logoRing.getTotalLength()}`

// ---- Split headings into masked words ----------------------------------------
for (const el of $$('.split')) {
  const parts = []
  for (const node of [...el.childNodes]) {
    if (node.nodeType === 3) for (const w of node.textContent.split(/\s+/).filter(Boolean)) parts.push(w)
    else parts.push(node.outerHTML)
  }
  el.innerHTML = parts.map((p) => `<span class="w"><span>${p}</span></span>`).join(' ')
}

// ---- State the timeline animates; draw() maps it to the DOM ---------------------
const cam = { cx: 720, cy: 464, s: 0.8, rx: 28, ry: 0, ox: 0, oy: 420 }
const cur = { x: 900, y: 650, o: 0, press: 0, ripple: 0 }
const seq = { qa: 0, pal: 0 }
const wipe = { r: 0, x: 720, y: 200 }
const win = $('#win')
const layer = (n) => $(`[data-l="${n}"]`)

const tl = gsap.timeline({ paused: true, defaults: { ease: 'power3.inOut' } })

function boxes(M) {
  const B = (b) => ({ x: b.x, y: b.y + 28, w: b.w, h: b.h })
  const m = M.box
  return {
    composer: { x: 437, y: m.qaInput.y + 8, w: 790, h: 120 },
    // After Enter: the stats row and Up next, where the new task lands.
    landed: { x: 437, y: 70, w: 790, h: m.upNext.y + m.upNext.h - 40 },
    glance: B(m.glance),
    next: B(m.next),
    homeRight: { x: 833, y: 398, w: 394, h: 420 },
    // Cursor targets live inside .screen, below the window bar: no bar offset.
    check: m.homeCheck,
    qaInput: m.qaInput,
    boardL: { x: 437, y: 300, w: 430, h: 400 },
    boardR: { x: 800, y: 300, w: 430, h: 400 },
    goal: B(m.goalCard),
    thesis: B(m.goalThesis),
    rlist: B(m.routineList),
    rcard: B(m.routineCard),
    rcheck: m.routineCheck,
    tlL: { ...B(m.timeline), w: m.timeline.w * 0.55 },
    tlR: { ...B(m.timeline), x: m.timeline.x + m.timeline.w * 0.45, w: m.timeline.w * 0.55 },
    palette: { x: 400, y: 150, w: 640, h: 220 },
  }
}

const WIDE = { cx: 720, cy: 430, s: 1.02, rx: 0, ry: 0, ox: 0, oy: 0 }
const focus = (r, max = 2.2) => ({
  cx: r.x + r.w / 2,
  cy: r.y + r.h / 2,
  s: Math.min(1640 / r.w, 700 / r.h, max),
  rx: 0,
  ry: 0,
  ox: 0,
  oy: 0,
})
const centre = (r) => ({ x: r.x + r.w / 2 + 2, y: r.y + r.h / 2 + 2 })

function capIn(id, t) {
  const c = $(id)
  tl.set(c, { autoAlpha: 1 }, t)
  tl.fromTo(`${id} h2 .w > span`, { yPercent: 115 }, { yPercent: 0, duration: 0.9, stagger: 0.055, ease: 'expo.out' }, t)
  tl.fromTo(c.querySelector('.kick'), { opacity: 0, y: 16 }, { opacity: 1, y: 0, duration: 0.7, ease: 'power2.out' }, t)
  const sub = c.querySelector('.sub')
  if (sub) tl.fromTo(sub, { opacity: 0, y: 14 }, { opacity: 1, y: 0, duration: 0.8, ease: 'power2.out' }, t + 0.35)
}
function capOut(id, t) {
  const c = $(id)
  tl.to(`${id} h2 .w > span`, { yPercent: -115, duration: 0.55, stagger: 0.025, ease: 'power3.in' }, t)
  tl.to([c.querySelector('.kick'), c.querySelector('.sub')].filter(Boolean), { opacity: 0, y: -10, duration: 0.4, ease: 'power2.in' }, t)
  tl.set(c, { autoAlpha: 0 }, t + 1.3)
}
const show = (n, t, d = 0.45) => tl.to(layer(n), { opacity: 1, duration: d, ease: 'power1.inOut' }, t)
const hide = (n, t, d = 0.01) => tl.to(layer(n), { opacity: 0, duration: d, ease: 'none' }, t)
const camTo = (to, t, d, ease) => tl.to(cam, { ...to, duration: d, ease: ease ?? 'power3.inOut' }, t)
function cursorTo(p, t, d = 1) {
  tl.to(cur, { x: p.x, y: p.y, duration: d, ease: 'power2.inOut' }, t)
}
function click(t) {
  tl.to(cur, { press: 1, duration: 0.08, ease: 'power2.out' }, t)
  tl.to(cur, { press: 0, duration: 0.18, ease: 'power2.out' }, t + 0.1)
  tl.fromTo(cur, { ripple: 0 }, { ripple: 1, duration: 0.6, ease: 'power2.out', immediateRender: false }, t)
}
/** Swing out of one screen and into the next. */
function cut(from, to, t, next) {
  camTo({ cx: 720, cy: 450, s: 0.84, ry: -7, rx: 3 }, t, 0.7, 'power2.in')
  show(to, t + 0.5, 0.3)
  hide(from, t + 0.8)
  camTo(next, t + 0.7, 1.2, 'expo.out')
}
const pop = (sel, t) => tl.fromTo(sel, { opacity: 0, y: 30, scale: 0.85 }, { opacity: 1, y: 0, scale: 1, duration: 0.45, ease: 'back.out(2)' }, t)
const press = (sel, t) => tl.to(sel, { y: 5, duration: 0.07, yoyo: true, repeat: 1, ease: 'power1.inOut' }, t)
const unpop = (sel, t) => tl.to(sel, { opacity: 0, y: 16, duration: 0.35, ease: 'power2.in' }, t)

function build(M) {
  const b = boxes(M)

  // INTRO 0 - 5.2
  tl.set('.cap', { autoAlpha: 0 }, 0)
  tl.set('#caps', { '--scrim': 0 }, 0)
  tl.set('#stars', { opacity: 0 }, 0)
  tl.to('#stars', { opacity: 1, duration: 2, ease: 'power1.out' }, 0)
  tl.fromTo('.nebula', { opacity: 0, scale: 0.8 }, { opacity: 1, scale: 1, duration: 3, ease: 'power2.out', stagger: 0.3 }, 0)
  tl.to(rings, { strokeDashoffset: 0, duration: 2.4, stagger: 0.18, ease: 'power2.inOut' }, 0.2)
  tl.fromTo(planets, { opacity: 0 }, { opacity: 1, duration: 1, stagger: 0.2 }, 1.4)
  tl.fromTo('#sunglow', { opacity: 0, attr: { r: 120 } }, { opacity: 0.85, attr: { r: 280 }, duration: 2.4, ease: 'power2.out' }, 0.5)
  tl.fromTo('#logo', { opacity: 0, scale: 0.55, rotate: -25 }, { opacity: 1, scale: 1, rotate: 0, duration: 1.4, ease: 'expo.out' }, 0.8)
  tl.to(logoRing, { strokeDashoffset: 0, duration: 1.2, ease: 'power2.out' }, 1.0)
  tl.fromTo('#word', { opacity: 0, y: 50, letterSpacing: '0.08em' }, { opacity: 1, y: 0, letterSpacing: '-0.045em', duration: 1.5, ease: 'expo.out' }, 1.3)
  tl.fromTo('#tag .w > span', { yPercent: 115 }, { yPercent: 0, duration: 1, stagger: 0.07, ease: 'expo.out' }, 2.1)
  tl.to('#intro', { opacity: 0, scale: 0.94, filter: 'blur(14px)', duration: 0.8, ease: 'power2.in' }, 4.3)
  tl.to('#rings', { scale: 1.7, opacity: 0.32, transformOrigin: '50% 50%', duration: 1.8, ease: 'power3.inOut' }, 4.3)
  tl.to('#sunglow', { opacity: 0.25, duration: 1.4 }, 4.4)

  // QUICK ADD 4.9 - 14.6
  tl.set(win, { opacity: 0 }, 0)
  show('qa-empty', 4.9, 0.01)
  tl.to(win, { opacity: 1, duration: 0.6, ease: 'power1.out' }, 4.9)
  camTo(WIDE, 4.9, 1.7, 'expo.out')
  tl.to('#caps', { '--scrim': 1, duration: 1 }, 5.0)
  capIn('#c-qa', 5.3)
  camTo(focus(b.composer, 2.05), 6.6, 1.4)
  tl.to(cur, { o: 1, duration: 0.3 }, 6.9)
  cursorTo(centre({ ...b.qaInput, w: 120 }), 6.9, 1.0)
  click(8.0)
  show('qa', 8.05, 0.08)
  tl.to(cur, { o: 0, duration: 0.3 }, 8.3)
  tl.to(seq, { qa: M.seq.qa.frames - 1, duration: 3.3, ease: 'none' }, 8.3)
  pop('#k3', 11.9)
  press('#k3', 12.25)
  show('qa-added', 12.32, 0.18)
  unpop('#k3', 12.9)
  camTo(focus(b.landed, 1.9), 12.5, 1.6)
  capOut('#c-qa', 14.3)

  // HOME 14.6 - 23.9
  cut('qa-added', 'home', 14.6, WIDE)
  hide('qa', 15.4)
  capIn('#c-home', 15.3)
  camTo(focus(b.glance, 1.9), 16.6, 1.6)
  camTo(focus(b.next, 1.75), 18.4, 1.4)
  tl.set(cur, { x: 760, y: 760 }, 18.9)
  tl.to(cur, { o: 1, duration: 0.3 }, 19.0)
  cursorTo(centre(b.check), 19.0, 1.1)
  click(20.2)
  show('home-checked', 20.25, 0.12)
  tl.to(cur, { o: 0, duration: 0.3 }, 20.9)
  camTo(focus(b.homeRight, 1.75), 21.1, 1.6)
  capOut('#c-home', 23.4)

  // BOARD 23.9 - 29.9
  cut('home-checked', 'board', 23.9, focus(b.boardL, 1.9))
  hide('home', 24.7)
  capIn('#c-board', 24.5)
  camTo(focus(b.boardR, 1.9), 26.4, 2.6, 'sine.inOut')
  capOut('#c-board', 29.4)

  // GOALS 29.9 - 36.2
  cut('board', 'goals', 29.9, focus(b.goal, 1.75))
  capIn('#c-goals', 30.5)
  camTo(focus(b.thesis, 1.75), 33.0, 2.0, 'sine.inOut')
  capOut('#c-goals', 35.7)

  // ROUTINES 36.2 - 44.2
  cut('goals', 'routines', 36.2, focus(b.rlist, 1.6))
  capIn('#c-rout', 36.8)
  camTo(focus({ ...b.rcard, h: b.rcard.h + 30 }, 1.95), 38.6, 1.4)
  tl.set(cur, { x: 640, y: 330 }, 39.2)
  tl.to(cur, { o: 1, duration: 0.3 }, 39.3)
  cursorTo(centre(b.rcheck), 39.3, 1.0)
  click(40.4)
  show('routines-checked', 40.45, 0.14)
  tl.to(cur, { o: 0, duration: 0.3 }, 41.1)
  camTo(focus(b.rlist, 1.6), 41.4, 1.6)
  capOut('#c-rout', 43.7)

  // ROADMAP 44.2 - 50.2
  cut('routines-checked', 'roadmap', 44.2, focus(b.tlL, 2.0))
  hide('routines', 45.0)
  capIn('#c-road', 44.8)
  camTo(focus(b.tlR, 2.0), 46.6, 2.8, 'sine.inOut')
  capOut('#c-road', 49.7)

  // PALETTE 50.2 - 57.3
  cut('roadmap', 'home-checked', 50.2, WIDE)
  capIn('#c-pal', 50.8)
  pop('#k1', 51.9)
  pop('#k2', 52.0)
  press('#k1', 52.4)
  press('#k2', 52.42)
  show('pal', 52.5, 0.15)
  unpop(['#k1', '#k2'], 53.0)
  camTo(focus(b.palette, 1.9), 52.6, 1.2)
  tl.to(seq, { pal: M.seq.pal.frames - 1, duration: 1.1, ease: 'none' }, 53.3)
  pop('#k3', 54.6)
  press('#k3', 54.95)
  unpop('#k3', 55.5)
  tl.set(layer('home-light'), { opacity: 1 }, 55.0)
  tl.fromTo(wipe, { r: 0 }, { r: 1900, duration: 1.3, ease: 'power2.inOut', immediateRender: false }, 55.0)
  camTo(WIDE, 55.1, 1.5)
  hide('pal', 56.4)
  capOut('#c-pal', 56.8)

  // DEVICES 57.3 - 64.9
  camTo({ cx: 720, cy: 464, s: 0.6, ox: -360, oy: 40, ry: 16, rx: 2 }, 57.2, 1.6, 'expo.inOut')
  capIn('#c-dev', 57.8)
  const ph = $$('.phone')
  const spots = [
    [1040, 345, -7],
    [1290, 300, 0],
    [1540, 345, 7],
  ]
  ph.forEach((p, i) => tl.set(p, { left: spots[i][0], top: spots[i][1] }, 0))
  tl.fromTo(ph, { opacity: 0, y: 260, rotate: (i) => spots[i][2] * 2 }, { opacity: 1, y: 0, rotate: (i) => spots[i][2], duration: 1.4, stagger: 0.14, ease: 'expo.out' }, 58.1)
  tl.to(ph, { y: -26, duration: 5, ease: 'sine.inOut', stagger: 0.1 }, 59.4)
  camTo({ ry: 9, s: 0.62 }, 59.0, 5.4, 'sine.inOut')
  capOut('#c-dev', 64.2)
  tl.to(ph, { opacity: 0, y: -120, duration: 0.7, stagger: 0.06, ease: 'power2.in' }, 64.3)
  tl.to(win, { opacity: 0, duration: 0.6, ease: 'power2.in' }, 64.4)
  camTo({ s: 0.5, oy: -60 }, 64.4, 0.7, 'power2.in')
  tl.to('#caps', { '--scrim': 0, duration: 0.8 }, 64.6)

  // HOST 65 - 70.5
  capIn('#c-host', 65.2)
  tl.fromTo('#term', { opacity: 0, y: 50, scale: 0.97 }, { opacity: 1, y: 0, scale: 1, duration: 0.9, ease: 'expo.out' }, 65.6)
  let at = 66.3
  for (const [cls, d, gap] of [
    ['.l1', 1.0, 0.25],
    ['.l2', 0.6, 0.5],
    ['.l3', 0.2, 0.35],
    ['.l4', 1.0, 0.45],
    ['.l5', 0.25, 0],
  ]) {
    const el = $(`#term ${cls}`)
    const n = el.textContent.length
    tl.to(el, { width: el.scrollWidth + 4, duration: d, ease: d > 0.3 ? `steps(${n})` : 'power1.out' }, at)
    at += d + gap
  }
  tl.to('#host', { opacity: 0, y: -30, duration: 0.6, ease: 'power2.in' }, 70.3)

  // OUTRO 70.6 - 76
  tl.to('#rings', { scale: 1, opacity: 1, duration: 2.2, ease: 'expo.out' }, 70.4)
  tl.to('#sunglow', { opacity: 0.6, duration: 2 }, 70.6)
  tl.set('#outro', { opacity: 1 }, 0)
  tl.fromTo('#logo2', { opacity: 0, scale: 0.6, rotate: -20 }, { opacity: 1, scale: 1, rotate: 0, duration: 1.3, ease: 'expo.out' }, 71.0)
  tl.fromTo('#word2', { opacity: 0, y: 40 }, { opacity: 1, y: 0, duration: 1.2, ease: 'expo.out' }, 71.3)
  tl.fromTo('#o1 .w > span', { yPercent: 115 }, { yPercent: 0, duration: 1, stagger: 0.07, ease: 'expo.out' }, 71.8)
  tl.fromTo('#o2 span', { opacity: 0, y: 18 }, { opacity: 1, y: 0, duration: 0.7, stagger: 0.1, ease: 'power2.out' }, 72.5)
  tl.fromTo('#o3', { opacity: 0 }, { opacity: 1, duration: 0.8 }, 73.1)
  tl.to('#stage', { opacity: 0, duration: 0.9, ease: 'power1.in' }, DURATION - 0.9)
  tl.set({}, {}, DURATION)
}

// ---- Per-frame drawing --------------------------------------------------------
let shownQa = -1
let shownPal = -1
const pending = []
function setSrc(img, src) {
  img.src = src
  pending.push(img.decode().catch(() => {}))
}
function draw(t) {
  for (const s of stars) {
    s.el.style.opacity = String(s.base * (0.55 + 0.45 * Math.sin(t * s.speed + s.phase)))
    s.el.style.transform = `translate(${-t * 5 * s.depth}px, ${t * 1.2 * s.depth}px)`
  }
  planets.forEach((p, i) => {
    const [rx, ry, sp, ph] = orbits[i]
    const a = ph + t * sp
    p.setAttribute('cx', (rx * Math.cos(a)).toFixed(2))
    p.setAttribute('cy', (ry * Math.sin(a)).toFixed(2))
  })
  $('#ringset').setAttribute('transform', `translate(960 540) rotate(${-18 + t * 0.35})`)
  $$('.nebula').forEach((n, i) => (n.style.translate = `${Math.sin(t * 0.07 + i) * 40}px ${Math.cos(t * 0.05 + i * 2) * 30}px`))
  const f = Math.round(t * 60)
  $('#grain').style.transform = `translate(${((f * 37) % 100) - 50}px, ${((f * 53) % 100) - 50}px)`

  win.style.transform = `translate(${960 + cam.ox}px, ${640 + cam.oy}px) rotateX(${cam.rx}deg) rotateY(${cam.ry}deg) scale(${cam.s}) translate(${-cam.cx}px, ${-cam.cy}px)`
  const c = $('#cursor')
  c.style.opacity = String(cur.o)
  c.style.transform = `translate(${cur.x}px, ${cur.y}px) scale(${1 - cur.press * 0.18})`
  const rp = c.querySelector('.ripple')
  rp.style.opacity = String(cur.ripple > 0 && cur.ripple < 1 ? (1 - cur.ripple) * 0.95 : 0)
  rp.style.transform = `scale(${0.3 + cur.ripple * 1.3})`

  const qi = Math.round(seq.qa)
  if (qi !== shownQa) setSrc(layer('qa'), `assets/qa-${String((shownQa = qi)).padStart(2, '0')}.jpg`)
  const pi = Math.round(seq.pal)
  if (pi !== shownPal) setSrc(layer('pal'), `assets/pal-${String((shownPal = pi)).padStart(2, '0')}.jpg`)
  layer('home-light').style.clipPath = `circle(${wipe.r}px at ${wipe.x}px ${wipe.y}px)`
}

window.__film = {
  duration: DURATION,
  ready: (async () => {
    const M = await (await fetch('assets/manifest.json')).json()
    build(M)
    await document.fonts.ready
    await Promise.all($$('img').map((i) => i.decode().catch(() => {})))
    tl.seek(0)
    draw(0)
  })(),
  async seek(t) {
    tl.seek(t, false)
    draw(t)
    await Promise.all(pending.splice(0))
  },
}

// Live preview: open film.html?play to watch it in real time.
if (location.search.includes('play'))
  window.__film.ready.then(() => {
    const t0 = performance.now()
    const loop = () => {
      window.__film.seek(((performance.now() - t0) / 1000) % DURATION)
      requestAnimationFrame(loop)
    }
    loop()
  })
