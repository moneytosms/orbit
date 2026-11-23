import { createRequire } from 'node:module'
import { mkdir, writeFile } from 'node:fs/promises'
import { setTimeout as delay } from 'node:timers/promises'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'

const require = createRequire(process.env.PLAYWRIGHT_CLI)
const { chromium } = require('playwright')
const execFileAsync = promisify(execFile)
const input = JSON.parse(await new Promise((resolve, reject) => {
  let value = ''
  process.stdin.setEncoding('utf8')
  process.stdin.on('data', chunk => value += chunk)
  process.stdin.on('end', () => resolve(value))
  process.stdin.on('error', reject)
}))

const profiles = []
const pollers = []
const createdIds = []
let activeStage = 'android_cdp_attach'

async function cli(args, profile) {
  const { stdout } = await execFileAsync(input.cliBin, ['--json', ...args], {
    cwd: input.root, env: profile.env, maxBuffer: 1_000_000,
  })
  return JSON.parse(stdout.trim().split('\n').at(-1))
}

async function dbHas(db, ids) {
  if (!ids.length) return []
  const code = "import json,sqlite3,sys; d=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True); ids=json.loads(sys.argv[2]); print(json.dumps([i for i in ids if d.execute('select 1 from rows where id=?',(i,)).fetchone()]))"
  const { stdout } = await execFileAsync('python3', ['-c', code, db, JSON.stringify(ids)])
  return JSON.parse(stdout)
}

async function installHooks(page) {
  const pending = new Map()
  const starts = new WeakMap()
  const syncs = []
  page.on('request', request => {
    if (request.method() !== 'POST' || new URL(request.url()).pathname !== '/sync') return
    starts.set(request, Date.now())
    try {
      const body = JSON.parse(request.postData() ?? '')
      for (const command of body.commands ?? []) {
        const title = command.args?.title
        const waiter = pending.get(title)
        if (waiter && command.type === 'task.create') {
          waiter.resolve({ id: command.args.id, uploadAt: starts.get(request) })
          pending.delete(title)
        }
      }
    } catch { /* malformed unrelated request */ }
  })
  page.on('response', response => {
    const request = response.request()
    if (starts.has(request)) syncs.push({ startedAt: starts.get(request), completedAt: Date.now() })
  })
  return { pending, syncs }
}

async function waitForAreaScreen(page) {
  await page.waitForFunction(({ route, name }) => {
    const title = document.querySelector('input[aria-label="Area name"]')
    return location.hash === route && title?.value === name
  }, { route: `#/area/${input.areaId}`, name: input.areaName }, { timeout: 30_000 })
}

async function openAndroid(port = input.webviewPort) {
  const browser = await chromium.connectOverCDP(
    `http://127.0.0.1:${port}`, { noDefaults: true },
  )
  profiles.push(browser)
  activeStage = 'android_page_attach'
  const page = browser.contexts().flatMap(context => context.pages())
    .find(item => item.url().startsWith(input.url))
  if (!page) throw new Error('Android WebView target did not load the disposable endpoint')
  page.setDefaultTimeout(30_000)
  const hooks = await installHooks(page)
  const newTask = page.locator('button[aria-label="New task"]')
  const password = page.getByLabel('Password')
  activeStage = 'android_login_or_ready'
  const login = await Promise.race([
    password.waitFor({ state: 'visible' }).then(() => true),
    newTask.waitFor({ state: 'visible' }).then(() => false),
  ])
  if (login) {
    await password.fill(input.password)
    await page.getByRole('button', { name: 'Log in' }).click()
  }
  activeStage = 'android_area_route'
  await page.goto(`${input.url}/#/area/${input.areaId}`, { waitUntil: 'domcontentloaded' })
  await waitForAreaScreen(page)
  await newTask.waitFor({ state: 'visible' })
  activeStage = 'android_visibility'
  if (await page.evaluate(() => document.visibilityState) !== 'visible')
    throw new Error('Android app is not foreground and visible')
  activeStage = 'android_device_lookup'
  const device = await page.evaluate(async () => (await (await fetch('/devices')).json()).find(item => item.current)?.id)
  if (!device) throw new Error('Android WebView has no current synthetic device')
  return { page, ...hooks, device, mobile: true }
}

async function openWeb() {
  const dir = `${input.profileRoot}/chromium-web-peer`
  await mkdir(dir, { recursive: true })
  const context = await chromium.launchPersistentContext(dir, {
    executablePath: input.chromium,
    headless: true,
    viewport: { width: 1365, height: 900 },
    args: ['--no-sandbox'],
  })
  profiles.push(context)
  const page = context.pages()[0] ?? await context.newPage()
  page.setDefaultTimeout(30_000)
  const hooks = await installHooks(page)
  await page.goto(`${input.url}/#/area/${input.areaId}`, { waitUntil: 'domcontentloaded' })
  const newTask = page.getByRole('button', { name: /New task/ }).first()
  const password = page.getByLabel('Password')
  const login = await Promise.race([
    password.waitFor({ state: 'visible' }).then(() => true),
    newTask.waitFor({ state: 'visible' }).then(() => false),
  ])
  if (login) {
    await password.fill(input.password)
    await page.getByRole('button', { name: 'Log in' }).click()
  }
  await newTask.waitFor({ state: 'visible' })
  await waitForAreaScreen(page)
  if (await page.evaluate(() => document.visibilityState) !== 'visible')
    throw new Error('Web peer is not foreground and visible')
  const device = await page.evaluate(async () => (await (await fetch('/devices')).json()).find(item => item.current)?.id)
  if (!device) throw new Error('Web peer has no current synthetic device')
  return { page, ...hooks, device, mobile: false }
}

async function sendWeb(client, title) {
  const uploaded = new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('browser task upload was not observed')), 30_000)
    client.pending.set(title, {
      resolve(value) { clearTimeout(timer); resolve(value) },
    })
  })
  const newTask = client.mobile
    ? client.page.locator('button[aria-label="New task"]')
    : client.page.getByRole('button', { name: /New task/ }).first()
  await newTask.click()
  const dialog = client.page.getByRole('dialog')
  const composer = dialog.locator('#composer')
  await composer.waitFor({ state: 'visible' })
  await composer.fill(title)
  const enqueueAt = Date.now()
  await dialog.getByRole('button', { name: 'Add', exact: true }).click()
  const { id, uploadAt } = await uploaded
  createdIds.push(id)
  return { id, enqueueAt, uploadAt, senderDevice: client.device }
}

function observe(client, title) {
  return client.page.getByText(title, { exact: true }).first()
    .waitFor({ state: 'visible', timeout: 240_000 })
    .then(() => Date.now()).catch(() => null)
}

async function startCliObserver(profile, pending) {
  let stopped = false
  const db = `${profile.dir}/orbit.db`
  const worker = (async () => {
    while (!stopped) {
      try {
        const syncStart = Date.now()
        await cli(['sync'], profile)
        for (const id of await dbHas(db, [...pending.keys()])) {
          const sample = pending.get(id)
          if (sample) {
            sample.observedAt = Date.now()
            sample.receiverSyncStart = syncStart
            pending.delete(id)
          }
        }
      } catch { /* missing samples are reported by the measurement deadline */ }
      await delay(250)
    }
  })()
  return async () => { stopped = true; await worker }
}

async function observeDesktop(profile, id) {
  const db = `${profile.dir}/orbit.db`
  const deadline = Date.now() + 240_000
  while (Date.now() < deadline) {
    try {
      if ((await dbHas(db, [id])).length) return Date.now()
    } catch { /* desktop replica is not ready yet */ }
    await delay(100)
  }
  return null
}

async function readNativeSyncs() {
  const text = await (await import('node:fs/promises')).readFile(input.desktopPeer.log, 'utf8')
  return [...text.matchAll(/native_sync_trace device=([^ ]+) started_at=(\d+) completed_at=(\d+)/g)]
    .map(match => ({ device: match[1], startedAt: Number(match[2]), completedAt: Number(match[3]) }))
}

async function measureDesktopPeer(pair, desktop, peer) {
  const profile = input.desktopPeer.profile
  const rows = []
  const initialDelay = Math.floor(Math.random() * 60_000)
  console.log(`native ${input.endpointName} ${pair} randomized_start_delay_ms=${initialDelay}; normal_poll_ms=60000`)
  await delay(initialDelay)
  let nextSlot = Date.now()
  for (let index = 0; index < input.samples; index++) {
    await delay(Math.max(0, nextSlot - Date.now()))
    const desktopSends = index % 2 === 0
    const sender = desktopSends ? null : peer
    const receiver = desktopSends ? peer : null
    const title = `issue14-native-${input.endpointName}-${pair === 'Desktop↔Web' ? 'web' : 'android'}-${String(index).padStart(3, '0')}`
    const sample = { direction: desktopSends ? 'a→b' : 'b→a', title }
    if (desktopSends) {
      const created = await cli(['add', `${title} @${input.areaName}`], profile)
      sample.id = created.id
      sample.enqueueAt = await taskCreatedAt(profile, sample.id)
      sample.senderDevice = desktop.device
      sample.receiverDevice = peer.device
      sample.observed = observe(peer, title)
    } else {
      Object.assign(sample, await sendWeb(sender, title))
      sample.receiverDevice = desktop.device
      sample.observed = observeDesktop(profile, sample.id)
    }
    createdIds.push(sample.id)
    rows.push(sample)
    nextSlot += 2_300
    if ((index + 1) % 20 === 0)
      console.log(`native ${input.endpointName} ${pair} queued=${index + 1}/${input.samples}`)
  }

  const deadline = Date.now() + 240_000
  for (const sample of rows) {
    const observed = await Promise.race([
      sample.observed,
      delay(Math.max(0, deadline - Date.now())).then(() => null),
    ])
    sample.observedAt = typeof observed === 'number' ? observed : observed?.observedAt
  }
  if (rows.some(sample => !sample.id || !sample.observedAt))
    throw new Error('native peer missed a probe')

  const events = await readTrace()
  const desktopSyncs = await readNativeSyncs()
  for (const sample of rows) {
    const applied = events.find(event => event.device === (sample.senderDevice ?? peer.device)
      && event.commands?.includes(sample.id))
    const fetched = events.find(event => event.device === sample.receiverDevice
      && event.delta?.includes(sample.id))
    if (!applied || !fetched) throw new Error('native peer trace could not correlate a probe')
    const desktopSend = sample.direction === 'a→b'
    const desktopEvent = desktopSend ? applied : fetched
    const desktopSync = desktopSyncs.find(item => item.device === desktop.device
      && item.startedAt <= desktopEvent.received_at && item.completedAt >= desktopEvent.received_at)
    const webSync = peer.syncs?.find(item => item.startedAt <= (desktopSend ? fetched : applied).received_at
      && item.completedAt >= (desktopSend ? fetched : applied).received_at)
    if (!desktopSync || !webSync) throw new Error('native peer poll phase was unavailable')
    const senderSync = desktopSend ? desktopSync.startedAt : sample.uploadAt
    const receiverSync = desktopSend ? webSync.startedAt : desktopSync.startedAt
    sample.enqueue_to_upload_ms = Math.max(0, senderSync - sample.enqueueAt)
    sample.upload_to_server_ms = Math.max(0, applied.received_at - senderSync)
    sample.server_apply_ms = Math.max(0, applied.applied_at - applied.received_at)
    sample.server_applied_to_fetch_ms = Math.max(0, fetched.received_at - applied.applied_at)
    sample.poll_wait_ms = Math.max(0, receiverSync - applied.applied_at)
    sample.fetch_to_observed_ms = Math.max(0, sample.observedAt - fetched.applied_at)
    sample.observed_ms = Math.max(0, sample.observedAt - sample.enqueueAt)
  }
  printSummary(pair, rows)
  return rows.map(({ title, observed, ...row }) => row)
}

function stats(rows, key) {
  const values = rows.map(row => row[key]).sort((a, b) => a - b)
  const at = p => values[Math.max(0, Math.floor((values.length - 1) * p + 0.5))]
  return [at(.5), at(.95), values.at(-1)]
}

async function readTrace() {
  const events = []
  const text = await (await import('node:fs/promises')).readFile(input.serverLog, 'utf8')
  for (const line of text.split('\n')) {
    const marker = 'sync_trace '
    const offset = line.indexOf(marker)
    if (offset < 0) continue
    try { events.push(JSON.parse(line.slice(offset + marker.length))) } catch { /* partial line */ }
  }
  return events
}

function printSummary(pair, rows) {
  console.log(`native ${input.endpointName} ${pair} samples=${rows.length} directions=${rows.filter(r => r.direction === 'a→b').length}/${rows.filter(r => r.direction === 'b→a').length}`)
  for (const key of ['observed_ms', 'enqueue_to_upload_ms', 'upload_to_server_ms', 'server_apply_ms', 'server_applied_to_fetch_ms', 'poll_wait_ms', 'fetch_to_observed_ms']) {
    const [p50, p95, max] = stats(rows, key)
    console.log(`native ${input.endpointName} ${pair} ${key}: p50=${p50}ms p95=${p95}ms max=${max}ms`)
  }
  for (const direction of ['a→b', 'b→a']) {
    const [p50, p95, max] = stats(rows.filter(r => r.direction === direction), 'observed_ms')
    console.log(`native ${input.endpointName} ${pair} ${direction} observed_ms: p50=${p50}ms p95=${p95}ms max=${max}ms`)
  }
}

async function measure(pair, android, peer, cliProfiles) {
  const androidToPeer = android
  const other = pair === 'Android↔Web' ? peer : null
  const cliReceiverPending = new Map()
  const stopCliObserver = pair === 'Android↔CLI'
    ? await startCliObserver(cliProfiles.b, cliReceiverPending) : null
  if (stopCliObserver) pollers.push(stopCliObserver)
  const rows = []
  const nextSlot = { value: Date.now() }
  const count = input.samples
  console.log(`native ${input.endpointName} ${pair} randomized_start_delay_ms=${Math.floor(Math.random() * 60_000)}; normal_poll_ms=60000`)
  await delay(Math.floor(Math.random() * 60_000))
  for (let index = 0; index < count; index++) {
    await delay(Math.max(0, nextSlot.value - Date.now()))
    const aToB = index % 2 === 0
    const androidSends = aToB
    const sender = androidSends ? androidToPeer : other
    const receiver = androidSends ? other : androidToPeer
    const title = `issue14-native-${input.endpointName}-${pair === 'Android↔CLI' ? 'cli' : 'web'}-${index.toString().padStart(3, '0')}`
    const direction = aToB ? 'a→b' : 'b→a'
    const sample = { direction, title }
    const observed = receiver ? observe(receiver, title) : null
    if (sender) {
      Object.assign(sample, await sendWeb(sender, title), {
        receiverDevice: receiver?.device ?? await cliDevice(cliProfiles.b),
      })
      if (pair === 'Android↔CLI') cliReceiverPending.set(sample.id, sample)
      sample.observe = observed
    } else {
      const profile = aToB ? cliProfiles.a : cliProfiles.b
      const created = await cli(['add', `${title} @${input.areaName}`], profile)
      sample.id = created.id
      sample.enqueueAt = await taskCreatedAt(profile, sample.id)
      sample.senderDevice = await cliDevice(profile)
      if (receiver) {
        sample.receiverDevice = receiver.device
        sample.observe = observed
      } else {
        sample.receiverDevice = await cliDevice(cliProfiles[ aToB ? 'b' : 'a' ])
        cliReceiverPending.set(sample.id, sample)
      }
      sample.uploadAt = Date.now()
      await cli(['sync'], profile)
      createdIds.push(sample.id)
    }
    rows.push(sample)
    nextSlot.value = Date.now() + 2_300
    if ((index + 1) % 20 === 0) console.log(`native ${input.endpointName} ${pair} queued=${index + 1}/${count}`)
  }
  const deadline = Date.now() + 240_000
  for (const sample of rows) {
    if (sample.observe) {
      const observedAt = await Promise.race([sample.observe, delay(Math.max(0, deadline - Date.now())).then(() => null)])
      if (observedAt) sample.observedAt = observedAt
    }
  }
  while (pair === 'Android↔CLI' && rows.some(row => !row.observedAt) && Date.now() < deadline)
    await delay(250)
  if (rows.some(row => !row.id || !row.observedAt)) throw new Error('native receiver missed a probe')
  if (stopCliObserver) await stopCliObserver()
  const events = await readTrace()
  for (const sample of rows) {
    const applied = events.find(event => event.device === sample.senderDevice && event.commands?.includes(sample.id))
    const fetched = events.find(event => event.device === sample.receiverDevice && event.delta?.includes(sample.id))
    if (!applied || !fetched) throw new Error('native trace could not correlate a probe')
    sample.receivedAt = applied.received_at
    sample.appliedAt = applied.applied_at
    sample.enqueue_to_upload_ms = Math.max(0, sample.uploadAt - sample.enqueueAt)
    sample.upload_to_server_ms = Math.max(0, applied.received_at - sample.uploadAt)
    sample.server_apply_ms = Math.max(0, applied.applied_at - applied.received_at)
    sample.server_applied_to_fetch_ms = Math.max(0, fetched.received_at - applied.applied_at)
    const receiverSyncStart = sample.receiverSyncStart ?? receiver.syncs
      ?.find(item => item.startedAt <= fetched.received_at + 250 && item.completedAt >= fetched.applied_at - 250)?.startedAt
    if (receiverSyncStart === undefined) throw new Error('normal receiver poll phase could not be matched')
    sample.poll_wait_ms = Math.max(0, receiverSyncStart - applied.applied_at)
    sample.fetch_to_observed_ms = Math.max(0, sample.observedAt - fetched.applied_at)
    sample.observed_ms = Math.max(0, sample.observedAt - sample.enqueueAt)
  }
  printSummary(pair, rows)
  return rows.map(({ title, ...row }) => row)
}

async function taskCreatedAt(profile, id) {
  const code = "import json,sqlite3,sys; d=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True); print(json.loads(d.execute('select data from rows where id=?',(sys.argv[2],)).fetchone()[0]).get('created_at') or 0)"
  const { stdout } = await execFileAsync('python3', ['-c', code, `${profile.dir}/orbit.db`, id])
  return Number(stdout.trim()) || Date.now()
}

async function cliDevice(profile) {
  const code = "import sqlite3,sys; d=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True); print(d.execute(\"select value from meta where key='hlc'\").fetchone()[0])"
  const { stdout } = await execFileAsync('python3', ['-c', code, `${profile.dir}/orbit.db`])
  return JSON.parse(stdout).device
}

try {
  if (input.desktopPeer) {
    activeStage = 'desktop_peer_ready'
    const peer = input.peerKind === 'android'
      ? await openAndroid()
      : await openWeb()
    activeStage = 'desktop_peer_pair'
    const pair = input.peerKind === 'android' ? 'Desktop↔Android' : 'Desktop↔Web'
    const desktop = { device: input.desktopPeer.device }
    const rows = await measureDesktopPeer(pair, desktop, peer)
    activeStage = 'write_results'
    await writeFile(input.output, JSON.stringify({ desktopPeer: rows, probes: createdIds }))
  } else {
  activeStage = 'android_ui_ready'
  const android = await openAndroid()
  const cliProfiles = input.cliProfiles
  const results = { androidForegroundDevice: android.device }
  if (input.pairs.includes('Android↔CLI')) {
    activeStage = 'android_cli_pair'
    results.androidCli = await measure('Android↔CLI', android, null, cliProfiles)
  }
  let web = null
  if (input.pairs.includes('Android↔Web')) {
    activeStage = 'web_peer_ready'
    web = await openWeb()
    activeStage = 'android_web_pair'
    results.androidWeb = await measure('Android↔Web', android, web, cliProfiles)
  }
  activeStage = 'write_results'
  await writeFile(input.output, JSON.stringify({ results, probes: createdIds }))
  }
} catch (error) {
  await writeFile(input.output, JSON.stringify({ probes: createdIds })).catch(() => {})
  const knownExceptions = new Set(['Error', 'TimeoutError', 'TypeError', 'ProtocolError', 'TargetClosedError'])
  const exception = error instanceof Error && knownExceptions.has(error.constructor.name)
    ? error.constructor.name
    : 'Error'
  console.error(`native_failure stage=${activeStage} exception=${exception}`)
  process.exitCode = 1
} finally {
  for (const stop of pollers.reverse()) await stop().catch(() => {})
  for (const profile of profiles.reverse()) await profile.close().catch(() => {})
}
