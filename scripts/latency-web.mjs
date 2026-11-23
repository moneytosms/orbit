import { createRequire } from 'node:module'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
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

const POLL_MS = 60_000
const SAMPLE_GAP_MS = 2_300
const profiles = []
const pollers = []
const createdIds = []
const failures = []

function idFromPost(request, title) {
  try {
    const body = JSON.parse(request.postData() ?? '')
    const command = body.commands?.find(item => item.args?.title === title)
    return command?.args?.id
  } catch {
    return undefined
  }
}

async function cli(args, config) {
  const { stdout } = await execFileAsync(config.cliBin, ['--json', ...args], {
    cwd: config.root,
    env: config.cliEnv,
    maxBuffer: 1_000_000,
  })
  return JSON.parse(stdout.trim().split('\n').at(-1))
}

async function dbHas(db, ids) {
  if (!ids.length) return []
  const code = "import json,sqlite3,sys; db=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True); ids=json.loads(sys.argv[2]); print(json.dumps([i for i in ids if db.execute('select 1 from rows where id=?',(i,)).fetchone()]))"
  const { stdout } = await execFileAsync('python3', ['-c', code, db, JSON.stringify(ids)], { maxBuffer: 1_000_000 })
  return JSON.parse(stdout)
}

async function openProfile(config, name) {
  const dir = `${config.profileRoot}/${name}`
  await mkdir(dir, { recursive: true })
  const context = await chromium.launchPersistentContext(dir, {
    executablePath: config.chromium,
    headless: true,
    viewport: { width: 1365, height: 900 },
    args: ['--no-sandbox'],
  })
  profiles.push(context)
  const page = context.pages()[0] ?? await context.newPage()
  const pending = new Map()
  const newTask = page.getByRole('button', { name: /New task/ }).first()
  page.on('request', request => {
    if (request.method() !== 'POST' || new URL(request.url()).pathname !== '/sync') return
    const at = Date.now()
    try {
      const body = JSON.parse(request.postData() ?? '')
      if (body.commands?.length) {
        console.log(`web profile=${name} sync_commands=${body.commands.map(command => command.type).join(',')}`)
        const task = body.commands.find(command => command.type === 'task.create')
        if (task) console.log(`web profile=${name} task_title_match=${[...pending.keys()].some(title => task.args?.title === title)} task_id_present=${Boolean(task.args?.id)} task_arg_keys=${Object.keys(task.args ?? {}).join(',')}`)
      }
    } catch { /* ignored */ }
    for (const [title, waiter] of pending) {
      const id = idFromPost(request, title)
      if (id) {
        pending.delete(title)
        waiter.resolve({ id, uploadAt: at })
      }
    }
  })
  page.on('response', response => {
    if (new URL(response.url()).pathname === '/sync') console.log(`web profile=${name} sync_status=${response.status()}`)
  })
  await page.goto(`${config.url}/#/area/${config.areaId}`, { waitUntil: 'domcontentloaded' })
  const password = page.getByLabel('Password')
  const loginNeeded = await Promise.race([
    password.waitFor({ state: 'visible', timeout: 30_000 }).then(() => true),
    newTask.waitFor({ state: 'visible', timeout: 30_000 }).then(() => false),
  ])
  console.log(`web profile=${name} loaded login=${loginNeeded}`)
  if (loginNeeded) {
    await password.fill(config.password)
    await page.getByRole('button', { name: 'Log in' }).click()
  }
  try {
    await newTask.waitFor({ state: 'visible', timeout: 30_000 })
  } catch {
    const text = (await page.locator('body').innerText()).slice(0, 500).replaceAll(/\s+/g, ' ')
    throw new Error(`browser app shell did not appear; visible text: ${text}`)
  }
  await page.getByRole('link', { name: config.areaName, exact: true }).waitFor({ state: 'visible', timeout: 30_000 })
  const device = await page.evaluate(async () => {
    const response = await fetch('/devices')
    const devices = await response.json()
    return devices.find(item => item.current)?.id
  })
  if (!device) throw new Error('browser profile did not expose its current synthetic device')
  if (await page.evaluate(() => document.visibilityState) !== 'visible') {
    throw new Error('Chromium page is not visible; normal browser polling would be suspended')
  }
  return { context, page, device, pending }
}

function waitForWebTask(receiver, title) {
  const started = Date.now()
  return receiver.page.getByText(title, { exact: true }).first()
    .waitFor({ state: 'visible', timeout: 180_000 })
    .then(() => ({ observedAt: Date.now(), waitStarted: started }))
    .catch(() => null)
}

async function sendWeb(sender, title) {
  const upload = new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      sender.pending.delete(title)
      reject(new Error('browser task upload was not observed'))
    }, 30_000)
    sender.pending.set(title, {
      resolve(value) { clearTimeout(timer); resolve(value) },
    })
  })
  await sender.page.getByRole('button', { name: /New task/ }).first().click()
  await sender.page.getByRole('dialog').locator('#composer').fill(title)
  const enqueueAt = Date.now()
  await sender.page.getByRole('dialog').getByRole('button', { name: 'Add', exact: true }).click()
  const { id, uploadAt } = await upload
  return { id, enqueueAt, uploadAt }
}

async function startCliPoller(config, receiver, pending) {
  const db = `${config.cliDir}/orbit.db`
  let stopped = false
  const worker = (async () => {
    while (!stopped) {
      try {
        await cli(['sync'], config)
        const ids = [...pending.keys()]
        for (const id of await dbHas(db, ids)) {
          const sample = pending.get(id)
          if (sample) {
            sample.observedAt = Date.now()
            pending.delete(id)
          }
        }
      } catch {
        // Keep polling; caller reports any still-missing samples after the deadline.
      }
      await delay(250)
    }
  })()
  return async () => { stopped = true; await worker }
}

async function readEvents() {
  const events = []
  for (const line of (await readFile(input.serverLog, 'utf8')).split('\n')) {
    const marker = 'sync_trace '
    const offset = line.indexOf(marker)
    if (offset < 0) continue
    try { events.push(JSON.parse(line.slice(offset + marker.length))) } catch { /* partial final line */ }
  }
  return events
}

function stats(rows, key) {
  const values = rows.map(row => row[key]).sort((a, b) => a - b)
  const at = p => values[Math.max(0, Math.floor((values.length - 1) * p + 0.5))]
  return [at(.5), at(.95), values.at(-1)]
}

function printSummary(pair, rows) {
  console.log(`web ${input.endpointName} ${pair} samples=${rows.length} directions=${rows.filter(r => r.direction === 'a→b').length}/${rows.filter(r => r.direction === 'b→a').length}`)
  for (const key of ['observed_ms', 'enqueue_to_upload_ms', 'upload_to_server_ms', 'server_apply_ms', 'server_applied_to_fetch_ms', 'fetch_to_observed_ms']) {
    const [p50, p95, max] = stats(rows, key)
    console.log(`web ${input.endpointName} ${pair} ${key}: p50=${p50}ms p95=${p95}ms max=${max}ms`)
  }
  for (const direction of ['a→b', 'b→a']) {
    const subset = rows.filter(row => row.direction === direction)
    const [p50, p95, max] = stats(subset, 'observed_ms')
    console.log(`web ${input.endpointName} ${pair} ${direction} observed_ms: p50=${p50}ms p95=${p95}ms max=${max}ms`)
  }
}

async function measurePair(pair) {
  const isWebWeb = pair === 'Web↔Web'
  const webA = await openProfile(input, `${pair === 'Web↔CLI' ? 'web-cli' : 'web-web'}-a`)
  const webB = isWebWeb ? await openProfile(input, 'web-web-b') : null
  const cliProfile = { cliBin: input.cliBin, root: input.root, cliEnv: input.cliEnv }
  const cliDevice = input.cliDevice
  const samples = []
  const cliPending = new Map()
  const stopCliPoller = !isWebWeb ? await startCliPoller(input, null, cliPending) : null
  if (stopCliPoller) pollers.push(stopCliPoller)
  const initialDelay = input.samples === 100 ? Math.floor(Math.random() * POLL_MS) : 0
  console.log(`web ${input.endpointName} ${pair} randomized_start_delay_ms=${initialDelay}; browser_poll_ms=${POLL_MS}`)
  await delay(initialDelay)

  let nextSlot = Date.now()
  for (let index = 0; index < input.samples; index++) {
    const aToB = index % 2 === 0
    const title = `issue14-web-${input.endpointName}-${pair === 'Web↔CLI' ? 'cli' : 'web'}-${index.toString().padStart(3, '0')}`
    const sender = pair === 'Web↔CLI' ? (aToB ? webA : null) : (aToB ? webA : webB)
    const receiver = pair === 'Web↔CLI' ? (aToB ? null : webA) : (aToB ? webB : webA)
    const direction = aToB ? 'a→b' : 'b→a'
    const sample = { direction, title }
    if (receiver) sample.observe = waitForWebTask(receiver, title)
    else if (sender) {
      // The CLI receiver poller tracks IDs once the browser POST exposes them.
      sample.onWebId = (id) => cliPending.set(id, sample)
    } else {
      // The browser receiver will reveal the CLI task once its normal 60 s timer syncs.
      sample.observe = waitForWebTask(webA, title)
    }

    await delay(Math.max(0, nextSlot - Date.now()))
    if (sender) {
      const sent = await sendWeb(sender, title)
      Object.assign(sample, sent, { senderDevice: sender.device, receiverDevice: receiver?.device ?? cliDevice })
      createdIds.push(sent.id)
      sample.onWebId?.(sent.id)
    } else {
      const enqueueAt = Date.now()
      const added = await cli(['add', `${title} @${input.areaName}`], cliProfile)
      const id = added.id
      const uploadAt = Date.now()
      sample.id = id
      createdIds.push(id)
      sample.enqueueAt = enqueueAt
      sample.uploadAt = uploadAt
      sample.senderDevice = cliDevice
      sample.receiverDevice = webA.device
      sample.observe = waitForWebTask(webA, title)
      await cli(['sync'], cliProfile)
    }
    samples.push(sample)
    nextSlot += SAMPLE_GAP_MS
    if ((index + 1) % 20 === 0) console.log(`web ${input.endpointName} ${pair} queued=${index + 1}/${input.samples}`)
  }

  if (stopCliPoller) await delay(300)
  const deadline = Date.now() + 180_000
  for (const sample of samples) {
    if (sample.observe) {
      const observed = await Promise.race([
        sample.observe,
        delay(Math.max(0, deadline - Date.now())).then(() => null),
      ])
      if (observed) sample.observedAt = observed.observedAt
    } else if (!sample.observedAt) {
      while (!sample.observedAt && Date.now() < deadline) await delay(250)
    }
    if (!sample.id || !sample.observedAt) failures.push(`${pair} sample ${sample.title} missing id or receiver observation`)
  }
  if (stopCliPoller) await stopCliPoller()

  const events = await readEvents()
  for (const sample of samples) {
    const applied = events.find(event => event.device === sample.senderDevice && event.commands?.includes(sample.id))
    const fetched = events.find(event => event.device === sample.receiverDevice && event.delta?.includes(sample.id))
    if (!applied || !fetched || !sample.observedAt) {
      console.log(`web ${input.endpointName} ${pair} trace_match applied=${Boolean(applied)} fetched=${Boolean(fetched)} observed=${Boolean(sample.observedAt)}`)
      failures.push(`${pair} sample ${sample.title} missing sender or receiver trace`)
      continue
    }
    sample.received_ms = applied.received_at
    sample.applied_ms = applied.applied_at
    sample.fetch_received_ms = fetched.received_at
    sample.fetch_completed_ms = fetched.applied_at
    sample.enqueue_to_upload_ms = Math.max(0, sample.uploadAt - sample.enqueueAt)
    sample.upload_to_server_ms = Math.max(0, applied.received_at - sample.uploadAt)
    sample.server_apply_ms = Math.max(0, applied.applied_at - applied.received_at)
    sample.server_applied_to_fetch_ms = Math.max(0, fetched.received_at - applied.applied_at)
    sample.fetch_to_observed_ms = Math.max(0, sample.observedAt - fetched.applied_at)
    sample.observed_ms = Math.max(0, sample.observedAt - sample.enqueueAt)
  }
  if (failures.length) throw new Error(failures[0])
  printSummary(pair, samples)
  return samples.map(({ id, title, ...row }) => ({ id, ...row }))
}

try {
  const webCli = await measurePair('Web↔CLI')
  const webWeb = await measurePair('Web↔Web')
  await writeFile(input.output, JSON.stringify({ webCli, webWeb, probes: createdIds }))
} catch (error) {
  await writeFile(input.output, JSON.stringify({ probes: createdIds })).catch(() => {})
  console.error(`web harness failed: ${error.message}`)
  process.exitCode = 1
} finally {
  for (const stop of pollers.reverse()) await stop().catch(() => {})
  for (const context of profiles.reverse()) await context.close().catch(() => {})
}
