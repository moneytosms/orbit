<script lang="ts">
  import Laptop from '@lucide/svelte/icons/laptop'
  import Smartphone from '@lucide/svelte/icons/smartphone'
  import Globe from '@lucide/svelte/icons/globe'
  import Terminal from '@lucide/svelte/icons/terminal'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import * as auth from '$lib/auth'
  import { notifyEnabled, setNotifyEnabled } from '$lib/notify'
  import {
    getDaySettings,
    getDefaultReminder,
    setDefaultReminder,
    getSettings,
    setDayStart,
    setStartView,
    setTimezone,
    getNextSettings,
    setNextSettings,
    views,
    VIEW_INBOX,
    VIEW_UPCOMING,
    getWeekStart,
    setWeekStart,
    type View,
  } from '$lib/sync'
  import {
    getAccent,
    getDensity,
    getFontSize,
    getTheme,
    paletteList,
    setAccent,
    setDensity,
    setFontSize,
    setTheme,
    type Choice,
    type Density,
    type FontSize,
    type PaletteInfo,
    type Strategy,
  } from '$lib/theme'

  let choice = $state<Choice>(getTheme())
  const pick = (c: Choice) => setTheme((choice = c))
  const orbit: [Choice, string][] = [
    ['light', 'Orbit Light'],
    ['dark', 'Orbit Dark'],
    ['system', 'System'],
  ]
  const palettes = paletteList()
  const featured = palettes.filter((p) => p.featured)
  const direct = palettes.filter((p) => p.offered.includes('direct'))
  const strategyLabel: Record<Strategy, string> = { direct: 'Direct', derived: 'Derived', accent: 'Accent' }
  const gradeLabel = { aaa: 'AAA', aa: 'AA', low: 'Low' }
  const mt = (name: string, s: Strategy): Choice => `mt:${name}:${s}`
  // The palette the current choice uses, so its other offered strategies can be picked.
  const active = $derived(choice.startsWith('mt:') ? palettes.find((p) => p.name === choice.split(':')[1]) : undefined)
  const activeStrategy = $derived(choice.split(':')[2])

  let density: Density = $state(getDensity())
  let fontSize: FontSize = $state(getFontSize())
  let accent: string | null = $state(getAccent())
  const cssAccent = () => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() || '#c2552f'

  // Free disk on the server; a failed check shows nothing.
  let freeMb: number | undefined = $state()
  fetch('/disk')
    .then((r) => (r.ok ? r.json() : Promise.reject()))
    .then(
      (d: { free_bytes: number }) => {
        if (d.free_bytes < 1_000_000_000) freeMb = Math.floor(d.free_bytes / 1_000_000)
      },
      () => {},
    )

  let exporting = $state(false)
  let exportMsg = $state('')
  async function exportAll() {
    exporting = true
    exportMsg = ''
    try {
      const res = await fetch('/export')
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const url = URL.createObjectURL(await res.blob())
      const d = new Date()
      const p = (n: number) => String(n).padStart(2, '0')
      const day = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
      // A plain link would be intercepted by the PWA, so download from an object URL.
      const a = Object.assign(document.createElement('a'), { href: url, download: `orbit-export-${day}.zip` })
      a.click()
      setTimeout(() => URL.revokeObjectURL(url), 0)
    } catch {
      exportMsg = "Couldn't export. Try again when you're online."
    } finally {
      exporting = false
    }
  }

  let list: auth.Device[] = $state([])
  let listError = $state('')
  const load = () =>
    auth.devices().then(
      (d) => ((list = d), (listError = '')),
      () => (listError = "Can't load Devices while offline."),
    )
  load()
  const icon = (p: string) => ({ web: Globe, android: Smartphone, cli: Terminal, script: Terminal })[p] ?? Laptop
  const when = (ms: number) => new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })

  let tokenName = $state('')
  let newToken = $state('')
  let startView = $state('')
  let allViews = $state<View[]>([])
  let timezone = $state('')
  let dayStart = $state('00:00')
  const zones: string[] =
    typeof Intl.supportedValuesOf === 'function' ? (Intl.supportedValuesOf as (k: string) => string[])('timeZone') : []
  getSettings().then((s) => {
    if (s) startView = s.start_view
  })
  getDaySettings().then((s) => {
    timezone = s.timezone
    dayStart = s.day_start
  })
  let notify = $state(notifyEnabled())
  let defaultReminder = $state(true)
  getDefaultReminder().then((v) => (defaultReminder = v))
  let weekStart = $state('1')
  getWeekStart().then((v) => (weekStart = String(v)))
  views().then((v) => (allViews = v))
  let nextDeadline = $state(3)
  let nextDue = $state(7)
  let nextP1 = $state(true)
  getNextSettings().then((n) => {
    nextDeadline = n.deadline_days
    nextDue = n.due_days
    nextP1 = n.undated_p1
  })
  const days = (v: number) => (Number.isFinite(v) ? Math.max(0, Math.floor(v)) : 0)
  // Inbox and Upcoming are already plain routes; hidden views stay listed only if they are the current start.
  const startOptions = $derived(
    allViews.filter(
      (v) => v.id !== VIEW_INBOX && v.id !== VIEW_UPCOMING && (!v.hidden || `view:${v.id}` === startView),
    ),
  )
  async function createToken(e: SubmitEvent) {
    e.preventDefault()
    try {
      newToken = (await auth.createToken(tokenName)).token
      tokenName = ''
      load()
    } catch (err) {
      listError =
        err instanceof Error && err.message === 'device_name' ? 'Give the token a name.' : "Couldn't create the token."
    }
  }
  const revoke = (id: string) => auth.revoke(id).then(load, () => (listError = "Couldn't revoke that Device."))

  let current = $state('')
  let next = $state('')
  let others = $state(false)
  let pwMsg = $state('')
  async function changePassword(e: SubmitEvent) {
    e.preventDefault()
    try {
      await auth.changePassword(current, next, others)
      pwMsg = others ? 'Password changed. Other Devices are signed out.' : 'Password changed.'
      current = next = ''
      load()
    } catch (err) {
      const c = err instanceof Error ? err.message : ''
      pwMsg =
        { credentials: 'Current password is wrong.', weak_password: 'Use at least 8 characters.' }[c] ??
        'Could not change the password.'
    }
  }

  let enroll: { secret: string; url: string } | undefined = $state()
  let totpCode = $state('')
  let totpPassword = $state('')
  async function startTotp(e: SubmitEvent) {
    e.preventDefault()
    try {
      enroll = await auth.startTotp(totpPassword)
      totpMsg = ''
    } catch {
      totpMsg = 'Current password is wrong.'
    }
    totpPassword = ''
  }
  let totpMsg = $state('')
  async function confirmTotp(e: SubmitEvent) {
    e.preventDefault()
    try {
      await auth.confirmTotp(totpCode)
      enroll = undefined
      totpMsg = 'Two-step login is on. Logins now ask for a code.'
    } catch {
      totpMsg = 'That code is wrong or expired.'
    }
    totpCode = ''
  }
</script>

<!-- Snippets keep the segmented-button styling in one place. -->
{#snippet tab(pressed: boolean, onpick: () => void, text: string)}
  <button
    type="button"
    aria-pressed={pressed}
    onclick={onpick}
    class="h-7 rounded-md px-3 font-medium text-muted aria-pressed:bg-surface aria-pressed:text-text aria-pressed:shadow-1"
  >
    {text}
  </button>
{/snippet}

{#snippet swatchTab(p: PaletteInfo, s: Strategy)}
  <button
    type="button"
    aria-pressed={choice === mt(p.name, s)}
    onclick={() => pick(mt(p.name, s))}
    class="flex h-7 items-center gap-2 rounded-md px-2.5 text-muted aria-pressed:bg-surface aria-pressed:text-text aria-pressed:shadow-1"
  >
    <span aria-hidden="true" class="flex overflow-hidden rounded-xs border border-line">
      <span class="size-2.5" style:background={p.bg}></span>
      <span class="size-2.5" style:background={p.text}></span>
      <span class="size-2.5" style:background={p.main}></span>
    </span>
    <span>{p.name.replaceAll('_', ' ')}</span>
    <span class="text-[12px] text-icon">{gradeLabel[p.grade]}</span>
  </button>
{/snippet}

<h1 class="page-title">Settings</h1>
<div class="flex flex-col gap-4">
  {#if freeMb !== undefined}
    <div role="alert" class="rounded-2xl border border-line bg-warn-soft p-4 text-warn-text">
      Server disk is nearly full: {freeMb} MB free. Back up and free space.
    </div>
  {/if}

  <section class="panel">
    <h2 class="panel-title">Appearance</h2>

    <h3 class="mb-1.5 text-[12.5px] font-medium text-muted">Featured</h3>
    <div class="mb-4 flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="Featured themes">
      {#each orbit as [value, label] (value)}
        {@render tab(choice === value, () => pick(value), label)}
      {/each}
      {#each featured as p (p.name)}
        {@render swatchTab(p, p.offered[0])}
      {/each}
    </div>

    <h3 class="mb-1.5 text-[12.5px] font-medium text-muted">More themes</h3>
    <div class="mb-4 flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="More themes">
      {#each palettes as p (p.name)}
        {@render swatchTab(p, p.offered[0])}
      {/each}
    </div>

    <h3 class="mb-1.5 text-[12.5px] font-medium text-muted">Direct</h3>
    <div class="mb-4 flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="Direct themes">
      {#each direct as p (p.name)}
        {@render swatchTab(p, 'direct')}
      {/each}
    </div>

    {#if active}
      <h3 class="mb-1.5 text-[12.5px] font-medium text-muted">Mapping for {active.name.replaceAll('_', ' ')}</h3>
      <div class="mb-4 flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="Mapping">
        {#each active.offered as s (s)}
          {@render tab(activeStrategy === s, () => pick(mt(active.name, s)), strategyLabel[s])}
        {/each}
      </div>
    {/if}

    <div class="flex flex-col gap-3">
      <div class="flex items-center gap-3">
        <Label for="accent">Accent</Label>
        <input
          id="accent"
          type="color"
          value={accent ?? cssAccent()}
          oninput={(e) => setAccent((accent = e.currentTarget.value))}
          class="size-8 cursor-pointer rounded-md border border-line bg-surface p-0.5"
        />
        <Button size="sm" disabled={accent === null} onclick={() => setAccent((accent = null))}>Reset accent</Button>
      </div>

      <div class="flex items-center gap-3">
        <span id="density-label">Density</span>
        <div class="flex gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-labelledby="density-label">
          {#each [['comfortable', 'Comfortable'], ['compact', 'Compact']] as [value, label] (value)}
            {@render tab(density === value, () => setDensity((density = value as Density)), label)}
          {/each}
        </div>
      </div>

      <div class="flex items-center gap-3">
        <span id="font-label">Font size</span>
        <div class="flex gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-labelledby="font-label">
          {#each [['s', 'S'], ['m', 'M'], ['l', 'L']] as [value, label] (value)}
            {@render tab(fontSize === value, () => setFontSize((fontSize = value as FontSize)), label)}
          {/each}
        </div>
      </div>
    </div>
  </section>

  <section class="panel">
    <h2 class="panel-title">Start view</h2>
    <p class="mb-3 text-muted">Where Orbit opens when you launch it.</p>
    <select
      bind:value={startView}
      onchange={() => setStartView(startView)}
      aria-label="Start view"
      class="h-[34px] w-full max-w-sm rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
    >
      <option value="">Inbox</option>
      <option value="upcoming">Upcoming</option>
      <option value="goals">Goals</option>
      {#each startOptions as v (v.id)}
        <option value="view:{v.id}">{v.name}</option>
      {/each}
    </select>
  </section>

  <section class="panel">
    <h2 class="panel-title">Day</h2>
    <p class="mb-3 text-muted">Routines roll over to a new day at the Day start, in this timezone.</p>
    <div class="grid max-w-sm gap-3">
      <select
        bind:value={timezone}
        onchange={() => setTimezone(timezone)}
        aria-label="Timezone"
        class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
      >
        <option value="">This device</option>
        {#each zones as z (z)}
          <option value={z}>{z}</option>
        {/each}
      </select>
      <input
        type="time"
        bind:value={dayStart}
        onchange={() => dayStart && setDayStart(dayStart)}
        aria-label="Day start"
        class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
      />
    </div>
  </section>

  <section class="panel">
    <h2 class="panel-title">What's next</h2>
    <div class="grid max-w-sm gap-3">
      <Label class="flex items-center justify-between gap-3">
        Deadline window (days)
        <Input
          type="number"
          min="0"
          class="w-24"
          bind:value={nextDeadline}
          onchange={() => setNextSettings({ deadline_days: days(nextDeadline) })}
        />
      </Label>
      <Label class="flex items-center justify-between gap-3">
        Due window (days)
        <Input
          type="number"
          min="0"
          class="w-24"
          bind:value={nextDue}
          onchange={() => setNextSettings({ due_days: days(nextDue) })}
        />
      </Label>
      <Label class="flex items-center gap-2">
        <Checkbox
          checked={nextP1}
          aria-label="Include undated P1 tasks"
          onCheckedChange={(c) => setNextSettings({ undated_p1: (nextP1 = c === true) })}
        />
        Include undated P1 tasks
      </Label>
    </div>
  </section>

  <section class="panel">
    <h2 class="panel-title">Reminders</h2>
    <div class="flex max-w-sm flex-col gap-3">
      <Label class="font-normal">
        <Checkbox bind:checked={notify} onCheckedChange={async (v) => (notify = await setNotifyEnabled(v === true))} />
        Notify on this device
      </Label>
      <Label class="font-normal">
        <Checkbox bind:checked={defaultReminder} onCheckedChange={(v) => setDefaultReminder(v === true)} />
        Remind at the due time of a Task with a time
      </Label>
    </div>
  </section>

  <section class="panel">
    <h2 class="panel-title">Calendar</h2>
    <div class="grid max-w-sm gap-3">
      <select
        bind:value={weekStart}
        onchange={() => setWeekStart(Number(weekStart))}
        aria-label="Week starts on"
        class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
      >
        <option value="1">Week starts on Monday</option>
        <option value="0">Week starts on Sunday</option>
      </select>
    </div>
  </section>

  <section class="panel">
    <h2 class="panel-title">Export</h2>
    <p class="mb-3 text-muted">Download every item as a zip of JSON.</p>
    <Button class="self-start" disabled={exporting} onclick={exportAll}>Export everything</Button>
    {#if exportMsg}<p role="status" class="mt-2 text-muted">{exportMsg}</p>{/if}
  </section>

  <section class="panel">
    <h2 class="panel-title">Devices <span class="font-normal text-muted">{list.length || ''}</span></h2>
    {#if listError}<p class="text-muted">{listError}</p>{/if}
    <ul>
      {#each list as d (d.id)}
        {@const Icon = icon(d.platform)}
        <li class="flex min-h-[42px] items-center gap-3 border-b border-line py-2 last:border-0">
          <Icon size={16} class="shrink-0 text-icon" />
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="truncate font-medium">{d.name}</span>
              {#if d.current}<span class="chip bg-accent-soft text-accent-text">This device</span>{/if}
            </div>
            <div class="text-[12.5px] text-muted">
              {d.platform} · added {when(d.created)} · last seen {when(d.last_seen)}
            </div>
          </div>
          {#if !d.current}
            <Button size="sm" variant="destructive" onclick={() => revoke(d.id)}>Revoke</Button>
          {/if}
        </li>
      {/each}
    </ul>
    <form onsubmit={createToken} class="mt-3 flex gap-2">
      <Input bind:value={tokenName} placeholder="Token name, e.g. Backup script" aria-label="Token name" required />
      <Button type="submit">Create token</Button>
    </form>
    {#if newToken}
      <div class="mt-3 rounded-lg bg-warn-soft p-3 text-warn-text">
        <p>Copy this token now; it won't be shown again. Send it as <code>Authorization: Bearer …</code></p>
        <code class="mt-2 block rounded-xs bg-surface p-2 break-all text-text select-all">{newToken}</code>
      </div>
    {/if}
  </section>

  <section class="panel">
    <h2 class="panel-title">Password</h2>
    <form onsubmit={changePassword} class="flex max-w-sm flex-col gap-3">
      <div class="flex flex-col gap-1.5">
        <Label for="current">Current password</Label>
        <Input id="current" type="password" bind:value={current} autocomplete="current-password" required />
      </div>
      <div class="flex flex-col gap-1.5">
        <Label for="new">New password</Label>
        <Input id="new" type="password" bind:value={next} autocomplete="new-password" required />
      </div>
      <Label class="font-normal"><Checkbox bind:checked={others} /> Sign out other devices</Label>
      {#if pwMsg}<p role="status" class="text-muted">{pwMsg}</p>{/if}
      <Button type="submit" class="self-start">Change password</Button>
    </form>
  </section>

  <section class="panel">
    <h2 class="panel-title">Two-step login</h2>
    {#if enroll}
      <form onsubmit={confirmTotp} class="flex max-w-sm flex-col gap-3">
        <p class="text-muted">
          Add this key to your authenticator app, or <a class="link" href={enroll.url}>open it there</a>, then enter the
          code it shows.
        </p>
        <code class="rounded-xs bg-sunk p-2 tracking-wider break-all select-all">{enroll.secret}</code>
        <Input bind:value={totpCode} inputmode="numeric" autocomplete="one-time-code" aria-label="Code" required />
        <Button type="submit" variant="primary" class="self-start">Turn on</Button>
      </form>
    {:else}
      <form onsubmit={startTotp} class="flex max-w-sm flex-col gap-3">
        <p class="text-muted">Ask for a code from an authenticator app after the password.</p>
        <Input
          type="password"
          bind:value={totpPassword}
          placeholder="Current password"
          aria-label="Current password"
          autocomplete="current-password"
          required
        />
        <Button type="submit" class="self-start">Set up authenticator</Button>
      </form>
    {/if}
    {#if totpMsg}<p role="status" class="mt-2 text-muted">{totpMsg}</p>{/if}
  </section>
</div>
