<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import Laptop from '@lucide/svelte/icons/laptop'
  import Smartphone from '@lucide/svelte/icons/smartphone'
  import Globe from '@lucide/svelte/icons/globe'
  import Terminal from '@lucide/svelte/icons/terminal'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import MonitorSmartphone from '@lucide/svelte/icons/monitor-smartphone'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Select } from '$lib/components/ui/select'
  import LabelManager from '$lib/LabelManager.svelte'
  import { Switch } from '$lib/components/ui/switch'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { checkServerVersion, currentUpdates, onUpdates, uiHash, type UpdateState } from '$lib/updates'
  import { isAndroidTauri, isDesktopTauri } from '$lib/platform'
  import * as auth from '$lib/auth'
  import CalendarFeeds from '$lib/CalendarFeeds.svelte'
  import TodoistImport from '$lib/TodoistImport.svelte'
  import ProfileEditor from '$lib/ProfileEditor.svelte'

  let { version = 0 }: { version?: number } = $props()
  import { notifyEnabled, setNotifyEnabled } from '$lib/notify'
  import AboutSection from '$lib/AboutSection.svelte'
  import { badgePreference, badgeSupported, refreshAppBadge, setBadgePreference } from '$lib/appBadge'
  import {
    getDaySettings,
    getDefaultReminders,
    getGamification,
    setDefaultReminders,
    setGamification,
    getSettings,
    setDayStart,
    setStartView,
    setTimezone,
    getNextSettings,
    getQuickAddSettings,
    setNextSettings,
    setQuickAddSettings,
    views,
    VIEW_INBOX,
    VIEW_UPCOMING,
    getWeekStart,
    setWeekStart,
    setTargets,
    setVacation,
    stardust,
    type StardustTargets,
    type View,
  } from '$lib/sync'
  import {
    getAccent,
    getDensity,
    getLeading,
    getMotion,
    getTextSize,
    getTheme,
    getTypeface,
    paletteList,
    setAccent,
    setDensity,
    resetAppearance,
    setLeading,
    setMotion,
    setTextSize,
    setTheme,
    setTypeface,
    type Choice,
    type Density,
    type Leading,
    type Motion,
    type TextSize,
    type Typeface,
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
  let textSize: TextSize = $state(getTextSize())
  let typeface: Typeface = $state(getTypeface())
  let leading: Leading = $state(getLeading())
  let motion: Motion = $state(getMotion())
  const textSizes: [TextSize, string][] = [
    ['90', 'Small'],
    ['100', 'Default'],
    ['115', 'Large'],
    ['130', 'Larger'],
    ['150', 'Largest'],
  ]
  const typefaces: [Typeface, string][] = [
    ['figtree', 'Figtree'],
    ['system', 'System UI'],
    ['serif', 'Serif'],
    ['mono', 'Mono'],
  ]
  const leadings: [Leading, string][] = [
    ['compact', 'Compact'],
    ['default', 'Default'],
    ['relaxed', 'Relaxed'],
  ]
  const motions: [Motion, string][] = [
    ['system', 'Follow system'],
    ['on', 'On'],
    ['off', 'Off'],
  ]
  const resetAll = () => {
    resetAppearance()
    ;[textSize, typeface, leading, motion] = [getTextSize(), getTypeface(), getLeading(), getMotion()]
  }
  let accent: string | null = $state(getAccent())
  const cssAccent = () => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() || '#4f7f5b'

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
  let updates: UpdateState = $state(currentUpdates())
  let appVersion = $state(__ORBIT_APP_VERSION__)
  let autoUpdates = $state(true)
  let captureShortcut = $state('ALT+T')
  let captureShortcutError = $state('')
  let recordingShortcut = $state(false)
  let captureShortcutInput = $state<HTMLInputElement>()
  onMount(() => {
    const off = onUpdates((value) => (updates = value))
    void checkServerVersion().catch(() => {})
    if (isDesktopTauri()) {
      void invoke<[string, string | null]>('get_capture_shortcut')
        .then(([shortcut, error]) => {
          captureShortcut = shortcut
          captureShortcutError = error ?? ''
        })
        .catch(() => {})
      void invoke<boolean>('get_auto_updates')
        .then((enabled) => (autoUpdates = enabled))
        .catch(() => {})
    }
    if (isAndroidTauri() || isDesktopTauri()) {
      void invoke<string>('shell_version')
        .then((version) => {
          if (version) appVersion = version
        })
        .catch(() => {})
    }
    return off
  })
  async function setAutoUpdates(value: boolean) {
    autoUpdates = value
    try {
      await invoke('set_auto_updates', { enabled: value })
    } catch {
      autoUpdates = !value
    }
  }
  let exportMsg = $state('')
  let exportOk = $state(false)
  let lastBackup = $state('Checking...')
  const ago = (ms: number) => {
    const m = Math.floor((Date.now() - ms) / 60_000)
    if (m < 1) return 'just now'
    const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })
    return m < 60
      ? rtf.format(-m, 'minute')
      : m < 1440
        ? rtf.format(-Math.floor(m / 60), 'hour')
        : rtf.format(-Math.floor(m / 1440), 'day')
  }
  auth.backupStatus().then(
    (ms) => (lastBackup = ms === null ? 'Never' : ago(ms)),
    () => (lastBackup = 'Unknown'),
  )
  async function exportAll() {
    exporting = true
    exportMsg = ''
    exportOk = false
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
      exportOk = true
      exportMsg = 'Export downloaded.'
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
  // Offsets in minutes before a timed due; `[]` is none.
  let defaultReminders = $state<number[]>([0])
  getDefaultReminders().then((v) => (defaultReminders = v))
  let appBadge = $state(badgePreference())
  let smartDates = $state(true)
  let nextWeekDay = $state('0')
  let weekendDay = $state('5')
  getQuickAddSettings().then((q) => {
    smartDates = q.smart_dates
    nextWeekDay = String(q.next_week_day)
    weekendDay = String(q.weekend_day)
  })
  const REMINDER_CHIPS: [number, string][] = [
    [0, 'At due time'],
    [10, '10 min'],
    [30, '30 min'],
    [60, '1 hour'],
    [1440, '1 day'],
  ]
  const toggleReminder = (m: number) => {
    const next = defaultReminders.includes(m)
      ? defaultReminders.filter((x) => x !== m)
      : [...defaultReminders, m].sort((a, b) => a - b)
    defaultReminders = next
    void setDefaultReminders(next)
  }
  const clearReminders = () => {
    defaultReminders = []
    void setDefaultReminders([])
  }
  let gamification = $state(true)
  let targets = $state<StardustTargets>()
  const loadTargets = () => stardust().then((d) => (targets = d.targets))
  loadTargets()
  const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
  const goal = (e: Event) => Math.max(1, Math.floor(Number((e.target as HTMLInputElement).value) || 1))
  const saveTargets = (t: Parameters<typeof setTargets>[0]) => setTargets(t).then(loadTargets)
  const toggleOff = (d: number) => {
    const off = targets?.days_off ?? []
    saveTargets({ days_off: off.includes(d) ? off.filter((x) => x !== d) : [...off, d].sort() })
  }
  const sections = [
    ['appearance', 'Appearance'],
    ['quick-add', 'Quick add'],
    ['labels', 'Labels'],
    ['general', 'Day and calendar'],
    ['capture-shortcut', 'Quick capture'],
    ['reminders', 'Reminders and notifications'],
    ['targets', 'Goals and targets'],
    ['profile', 'Profile'],
    ['account', 'Account and devices'],
    ['calendar', 'Calendars'],
    ['data', 'Backup and export'],
    ['advanced', 'Advanced'],
    ['updates', 'Version and updates'],
    ['about', 'About'],
  ]

  async function recordCaptureShortcut(event: KeyboardEvent) {
    if (!recordingShortcut) return
    event.preventDefault()
    event.stopPropagation()
    if (event.key === 'Escape') {
      recordingShortcut = false
      return
    }
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return
    const mods = [
      event.ctrlKey && 'CTRL',
      event.altKey && 'ALT',
      event.shiftKey && 'SHIFT',
      event.metaKey && 'SUPER',
    ].filter(Boolean)
    if (!mods.length || event.key.length !== 1) {
      captureShortcutError = 'Use at least one modifier and a letter or number.'
      return
    }
    const next = `${mods.join('+')}+${event.key.toUpperCase()}`
    try {
      await invoke('set_capture_shortcut', { shortcut: next })
      captureShortcut = next
      captureShortcutError = ''
      recordingShortcut = false
    } catch (error) {
      captureShortcutError = String(error)
    }
  }

  function beginCaptureShortcut() {
    recordingShortcut = true
    requestAnimationFrame(() => captureShortcutInput?.focus())
  }
  const jump = (id: string) => {
    const el = document.getElementById(id)
    if (el instanceof HTMLDetailsElement) el.open = true
    el?.scrollIntoView({ block: 'start' })
  }
  // Other screens link here with a section to open (see Profile).
  onMount(() => {
    const section = sessionStorage.getItem('orbit.jump')
    if (!section) return
    sessionStorage.removeItem('orbit.jump')
    requestAnimationFrame(() => jump(section))
  })
  getGamification().then((v) => (gamification = v))
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
    <span class="text-[0.75rem] text-icon">{gradeLabel[p.grade]}</span>
  </button>
{/snippet}

<!-- One setting: label and one-line helper on the left, control on the right. -->
{#snippet row(label: string, help: string, control: import('svelte').Snippet, labelId?: string)}
  <div
    class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-line py-3 first:pt-0 last:border-0 last:pb-0"
  >
    <div class="min-w-0 flex-1 basis-56">
      <div id={labelId} class="font-medium">{label}</div>
      <div class="text-[0.7812rem] text-muted">{help}</div>
    </div>
    {@render control()}
  </div>
{/snippet}

{#snippet seg(labelId: string, items: [string, string][], current: string, onpick: (v: string) => void)}
  <div class="flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-labelledby={labelId}>
    {#each items as [value, label] (value)}
      {@render tab(current === value, () => onpick(value), label)}
    {/each}
  </div>
{/snippet}

{#snippet summaryRow(title: string, hint: string)}
  <summary class="flex cursor-pointer list-none items-center gap-2 py-1 font-medium [&::-webkit-details-marker]:hidden">
    <ChevronRight size={16} class="shrink-0 text-icon transition-transform group-open:rotate-90" />
    {title}
    <span class="hidden font-normal text-muted sm:inline">{hint}</span>
  </summary>
{/snippet}

<h1 class="page-title">Settings</h1>
<div class="lg:grid lg:grid-cols-[11rem_minmax(0,1fr)] lg:items-start lg:gap-6">
  <nav aria-label="Settings sections" class="sticky top-4 hidden flex-col gap-0.5 lg:flex">
    {#each sections as [id, label] (id)}
      <button
        type="button"
        onclick={() => jump(id)}
        class="h-8 rounded-[7px] px-2.5 text-left text-muted hover:bg-hover hover:text-text"
      >
        {label}
      </button>
    {/each}
  </nav>

  <div class="flex min-w-0 flex-col gap-4">
    {#if freeMb !== undefined}
      <div role="alert" class="rounded-2xl border border-line bg-warn-soft p-4 text-warn-text">
        Server disk is nearly full: {freeMb} MB free. Back up and free space.
      </div>
    {/if}

    <section id="appearance" class="panel scroll-mt-4">
      <h2 class="panel-title">Appearance</h2>

      <div class="mb-4 flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="Theme">
        {#each orbit as [value, label] (value)}
          {@render tab(choice === value, () => pick(value), label)}
        {/each}
        {#each featured as p (p.name)}
          {@render swatchTab(p, p.offered[0])}
        {/each}
      </div>

      {#snippet accentControl()}
        <div class="flex items-center gap-2">
          <input
            id="accent"
            type="color"
            aria-label="Accent"
            value={accent ?? cssAccent()}
            oninput={(e) => setAccent((accent = e.currentTarget.value))}
            class="size-8 cursor-pointer rounded-md border border-line bg-surface p-0.5"
          />
          <Button size="sm" disabled={accent === null} onclick={() => setAccent((accent = null))}>Reset</Button>
        </div>
      {/snippet}
      {#snippet sizeControl()}
        {@render seg('size-label', textSizes, textSize, (v) => setTextSize((textSize = v as TextSize)))}
      {/snippet}
      {#snippet densityControl()}
        {@render seg(
          'density-label',
          [
            ['comfortable', 'Comfortable'],
            ['compact', 'Compact'],
          ],
          density,
          (v) => setDensity((density = v as Density)),
        )}
      {/snippet}
      {@render row('Text size', 'Scales all text in Orbit.', sizeControl, 'size-label')}
      {@render row('Accent colour', 'Used for the main action, selection and Today.', accentControl)}
      {@render row('Density', 'How much room rows get.', densityControl, 'density-label')}

      <details class="group pt-3">
        {@render summaryRow('More themes', `${palettes.length} palettes`)}
        <div class="mt-3 flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="More themes">
          {#each palettes as p (p.name)}
            {@render swatchTab(p, p.offered[0])}
          {/each}
        </div>

        <details class="group mt-3">
          {@render summaryRow('Direct themes', 'Exact palette colours, no adjustment')}
          <div
            class="mt-3 flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]"
            role="group"
            aria-label="Direct themes"
          >
            {#each direct as p (p.name)}
              {@render swatchTab(p, 'direct')}
            {/each}
          </div>
        </details>

        {#if active}
          <h3 class="mt-3 mb-1.5 text-[0.7812rem] font-medium text-muted">
            Mapping for {active.name.replaceAll('_', ' ')}
          </h3>
          <div class="flex flex-wrap gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="Mapping">
            {#each active.offered as s (s)}
              {@render tab(activeStrategy === s, () => pick(mt(active.name, s)), strategyLabel[s])}
            {/each}
          </div>
        {/if}
      </details>

      <details class="group mt-3 border-t border-line pt-3">
        {@render summaryRow('Typography and motion', 'Typeface, line spacing, reduce motion')}
        <div class="mt-3">
          {#snippet faceControl()}
            {@render seg('face-label', typefaces, typeface, (v) => setTypeface((typeface = v as Typeface)))}
          {/snippet}
          {#snippet leadingControl()}
            {@render seg('leading-label', leadings, leading, (v) => setLeading((leading = v as Leading)))}
          {/snippet}
          {#snippet motionControl()}
            {@render seg('motion-label', motions, motion, (v) => setMotion((motion = v as Motion)))}
          {/snippet}
          {@render row('Typeface', 'The font used across the app.', faceControl, 'face-label')}
          {@render row('Line spacing', 'Space between lines of text.', leadingControl, 'leading-label')}
          {@render row(
            'Reduce motion',
            'Follow the system, or force animations on or off.',
            motionControl,
            'motion-label',
          )}
          <div class="pt-3"><Button size="sm" onclick={resetAll}>Reset appearance</Button></div>
        </div>
      </details>
    </section>

    <section id="labels" class="panel scroll-mt-4">
      <h2 class="panel-title">Labels</h2>
      <p class="mb-3 text-sm text-muted">
        The #tags you put on tasks. Rename one to fix a typo everywhere, or delete it to take it off every task.
      </p>
      <LabelManager />
    </section>

    <section id="quick-add" class="panel scroll-mt-4">
      <h2 class="panel-title">Quick add</h2>
      {#snippet smartControl()}
        <Switch
          bind:checked={smartDates}
          aria-label="Recognise dates in quick add"
          onCheckedChange={(v) => setQuickAddSettings({ smart_dates: v === true })}
        />
      {/snippet}
      {#snippet nextWeekControl()}
        <Select
          bind:value={nextWeekDay}
          onchange={() => setQuickAddSettings({ next_week_day: Number(nextWeekDay) })}
          aria-label="Next week means"
          class="w-48"
          options={[
            { value: '0', label: 'Monday' },
            { value: '1', label: 'Tuesday' },
            { value: '2', label: 'Wednesday' },
            { value: '3', label: 'Thursday' },
            { value: '4', label: 'Friday' },
            { value: '5', label: 'Saturday' },
            { value: '6', label: 'Sunday' },
          ]}
        />
      {/snippet}
      {#snippet weekendControl()}
        <Select
          bind:value={weekendDay}
          onchange={() => setQuickAddSettings({ weekend_day: Number(weekendDay) })}
          aria-label="Weekend means"
          options={[
            { value: '4', label: 'Friday' },
            { value: '5', label: 'Saturday' },
            { value: '6', label: 'Sunday' },
          ]}
        />
      {/snippet}
      {@render row(
        'Recognise dates in quick add',
        'Words like "tomorrow", "fri" and "next week" set the due date. Off keeps them in the title.',
        smartControl,
      )}
      {@render row('Next week means', 'The day "next week" resolves to.', nextWeekControl)}
      {@render row('Weekend means', 'The day "weekend" resolves to.', weekendControl)}
    </section>

    <section id="general" class="panel scroll-mt-4">
      <h2 class="panel-title">Day and calendar</h2>
      {#snippet startControl()}
        <Select
          bind:value={startView}
          onchange={() => setStartView(startView)}
          aria-label="Open Orbit on"
          class="w-48"
          segmented={false}
          options={[
            { value: '', label: 'Inbox' },
            { value: 'upcoming', label: 'Upcoming' },
            { value: 'goals', label: 'Goals' },
            ...startOptions.map((v) => ({ value: `view:${v.id}`, label: v.name })),
          ]}
        />
      {/snippet}
      {#snippet weekControl()}
        <Select
          bind:value={weekStart}
          onchange={() => setWeekStart(Number(weekStart))}
          aria-label="Week starts on"
          options={[
            { value: '1', label: 'Monday' },
            { value: '0', label: 'Sunday' },
          ]}
        />
      {/snippet}
      {#snippet dayControl()}
        <input
          type="time"
          bind:value={dayStart}
          onchange={() => dayStart && setDayStart(dayStart)}
          aria-label="Day start"
          class="h-[2.125rem] w-48 rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
        />
      {/snippet}
      {#snippet zoneControl()}
        <Select
          bind:value={timezone}
          onchange={() => setTimezone(timezone)}
          aria-label="Timezone"
          class="w-48"
          segmented={false}
          options={[{ value: '', label: 'This device' }, ...zones.map((z) => ({ value: z, label: z }))]}
        />
      {/snippet}
      {@render row('Open Orbit on', 'The page Orbit shows when you launch it.', startControl)}
      {@render row('Week starts on', 'First day of the Calendar week.', weekControl)}
      {@render row('Day starts at', 'Routines roll over to a new day at this time.', dayControl)}
      {@render row('Timezone', 'Days and due times are read in this zone.', zoneControl)}
    </section>

    {#if isDesktopTauri()}
      <section id="capture-shortcut" class="panel scroll-mt-4">
        <h2 class="panel-title">Quick capture</h2>
        <div class="flex flex-wrap items-center gap-3">
          <input
            aria-label="Quick capture shortcut"
            bind:this={captureShortcutInput}
            readonly
            value={recordingShortcut
              ? 'Press a key combination…'
              : captureShortcut
                  .replaceAll('ALT', 'Alt')
                  .replaceAll('CTRL', 'Ctrl')
                  .replaceAll('SHIFT', 'Shift')
                  .replaceAll('SUPER', 'Super')}
            onkeydown={recordCaptureShortcut}
            onclick={beginCaptureShortcut}
            onblur={() => (recordingShortcut = false)}
            class="h-[2.125rem] w-56 rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
          />
          <Button size="sm" onclick={beginCaptureShortcut}>{recordingShortcut ? 'Press shortcut' : 'Change'}</Button>
        </div>
        {#if captureShortcutError}<p role="status" class="mt-2 text-sm text-warn-text">{captureShortcutError}</p>{/if}
        <p class="mt-2 text-sm text-muted">
          Opens a small capture window. Captures are saved on this device and sync when online.
        </p>
      </section>
    {/if}

    <section id="reminders" class="panel scroll-mt-4">
      <h2 class="panel-title">Reminders and notifications</h2>
      {#snippet notifyControl()}
        <Switch
          bind:checked={notify}
          aria-label="Notify on this device"
          onCheckedChange={async (v) => (notify = await setNotifyEnabled(v === true))}
        />
      {/snippet}
      {#snippet reminderControl()}
        <div class="flex flex-wrap justify-end gap-1.5" role="group" aria-label="Default reminders">
          {#each REMINDER_CHIPS as [m, label] (m)}
            <Button
              size="sm"
              variant={defaultReminders.includes(m) ? 'on' : 'ghost'}
              aria-pressed={defaultReminders.includes(m)}
              onclick={() => toggleReminder(m)}>{label}</Button
            >
          {/each}
          <Button
            size="sm"
            variant={defaultReminders.length === 0 ? 'on' : 'ghost'}
            aria-pressed={defaultReminders.length === 0}
            onclick={clearReminders}>None</Button
          >
        </div>
      {/snippet}
      {@render row('Notify on this device', 'Show system notifications for reminders here.', notifyControl)}
      {@render row(
        'Default reminders',
        'Tasks with a time get a reminder at each chosen time before it.',
        reminderControl,
      )}
      {#if badgeSupported()}
        {#snippet badgeControl()}
          <Switch
            bind:checked={appBadge}
            aria-label="Show due count on the app icon"
            onCheckedChange={(v) => {
              setBadgePreference(v === true)
              void refreshAppBadge()
            }}
          />
        {/snippet}
        {@render row(
          'Show due count on the app icon',
          'Shows how many tasks are due today or overdue on the app icon, where the browser allows it.',
          badgeControl,
        )}
      {/if}
    </section>

    <section id="targets" class="panel scroll-mt-4">
      <h2 class="panel-title">Goals and targets</h2>
      {#snippet gamControl()}
        <Switch
          bind:checked={gamification}
          aria-label="Gamification"
          onCheckedChange={(v) => setGamification(v === true)}
        />
      {/snippet}
      {@render row('Gamification', 'Stardust, Levels, Badges and Patches.', gamControl)}
      {#if targets}
        {#snippet dailyControl()}
          <Input
            type="number"
            min="1"
            class="w-24"
            aria-label="Daily target"
            value={targets?.daily}
            onchange={(e) => saveTargets({ target_daily: goal(e) })}
          />
        {/snippet}
        {#snippet weeklyControl()}
          <Input
            type="number"
            min="1"
            class="w-24"
            aria-label="Weekly target"
            value={targets?.weekly}
            onchange={(e) => saveTargets({ target_weekly: goal(e) })}
          />
        {/snippet}
        {#snippet offControl()}
          <div class="flex flex-wrap gap-x-3 gap-y-1">
            {#each DAYS as name, d (d)}
              <Label class="font-normal">
                <Checkbox checked={targets?.days_off.includes(d)} onCheckedChange={() => toggleOff(d)} />
                {name}
              </Label>
            {/each}
          </div>
        {/snippet}
        {#snippet vacationControl()}
          <Switch
            checked={targets?.vacation}
            aria-label="Vacation"
            onCheckedChange={(v) => setVacation(v === true).then(loadTargets)}
          />
        {/snippet}
        {@render row('Daily target', 'Tasks to finish each day.', dailyControl)}
        {@render row('Weekly target', 'Tasks to finish each week.', weeklyControl)}
        {@render row('Days off', 'Days that never break the streak.', offControl)}
        {@render row('Vacation', 'Pause the target streak until you turn this off.', vacationControl)}
      {/if}
    </section>

    <section id="profile" class="panel scroll-mt-4">
      <h2 class="panel-title">Profile</h2>
      <ProfileEditor {version} />
    </section>

    <section id="account" class="panel scroll-mt-4">
      <h2 class="panel-title">Account and devices <span class="font-normal text-muted">{list.length || ''}</span></h2>
      {#if listError}<p class="text-muted">{listError}</p>{/if}
      <ul>
        {#each list as d (d.id)}
          {@const Icon = icon(d.platform)}
          <li class="flex min-h-[2.625rem] items-center gap-3 border-b border-line py-2 last:border-0">
            <Icon size={16} class="shrink-0 text-icon" />
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="truncate font-medium">{d.name}</span>
                {#if d.current}<span class="chip bg-accent-soft text-accent-text">This device</span>{/if}
              </div>
              <div class="text-[0.7812rem] text-muted">
                {d.platform} · added {when(d.created)} · last seen {when(d.last_seen)}
              </div>
            </div>
            {#if !d.current}
              <Button size="sm" variant="destructive" onclick={() => revoke(d.id)}>Revoke</Button>
            {/if}
          </li>
        {:else}
          {#if !listError}
            <li class="flex flex-col items-center gap-2 px-3 py-4 text-center text-muted">
              <MonitorSmartphone size={20} class="text-icon" />
              <span>Devices signed in to your account appear here.</span>
            </li>
          {/if}
        {/each}
      </ul>

      <details class="group mt-3 border-t border-line pt-3">
        {@render summaryRow('API token', 'For scripts and the CLI')}
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
      </details>

      <details class="group mt-3 border-t border-line pt-3">
        {@render summaryRow('Change password', '')}
        <form onsubmit={changePassword} class="mt-3 flex max-w-sm flex-col gap-3">
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
      </details>

      <details class="group mt-3 border-t border-line pt-3">
        {@render summaryRow('Two-step login', 'Authenticator app code at sign-in')}
        {#if enroll}
          <form onsubmit={confirmTotp} class="mt-3 flex max-w-sm flex-col gap-3">
            <p class="text-muted">
              Add this key to your authenticator app, or <a class="link" href={enroll.url}>open it there</a>, then enter
              the code it shows.
            </p>
            <code class="rounded-xs bg-sunk p-2 tracking-wider break-all select-all">{enroll.secret}</code>
            <Input bind:value={totpCode} inputmode="numeric" autocomplete="one-time-code" aria-label="Code" required />
            <Button type="submit" variant="primary" class="self-start">Turn on</Button>
          </form>
        {:else}
          <form onsubmit={startTotp} class="mt-3 flex max-w-sm flex-col gap-3">
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
      </details>
    </section>

    <section id="calendar" class="panel scroll-mt-4">
      <h2 class="panel-title">Calendars</h2>
      <CalendarFeeds />
    </section>

    <TodoistImport />

    <section id="data" class="panel scroll-mt-4">
      <h2 class="panel-title">Backup and export</h2>
      {@render row('Last backup', 'When the server last saved a snapshot of your data.', lastBackupText)}
      {#snippet lastBackupText()}<span>{lastBackup}</span>{/snippet}
      {#snippet exportControl()}
        <Button disabled={exporting} onclick={exportAll}>Export everything</Button>
      {/snippet}
      {@render row('Export', 'Download every item as a zip of JSON. Keep it as a backup.', exportControl)}
      {#if exportMsg}<p role="status" class="mt-2 {exportOk ? 'text-muted' : 'text-warn-text'}">{exportMsg}</p>{/if}
    </section>

    <details id="advanced" class="group panel scroll-mt-4">
      <summary
        class="flex cursor-pointer list-none items-center gap-2 text-base font-semibold [&::-webkit-details-marker]:hidden"
      >
        <ChevronRight size={16} class="shrink-0 text-icon transition-transform group-open:rotate-90" />
        Advanced
        <span class="text-sm font-normal text-muted">What's next windows</span>
      </summary>
      <div class="mt-3">
        {#snippet deadlineControl()}
          <Input
            type="number"
            min="0"
            class="w-24"
            aria-label="Deadline window in days"
            bind:value={nextDeadline}
            onchange={() => setNextSettings({ deadline_days: days(nextDeadline) })}
          />
        {/snippet}
        {#snippet dueControl()}
          <Input
            type="number"
            min="0"
            class="w-24"
            aria-label="Due window in days"
            bind:value={nextDue}
            onchange={() => setNextSettings({ due_days: days(nextDue) })}
          />
        {/snippet}
        {#snippet p1Control()}
          <Switch
            checked={nextP1}
            aria-label="Include undated P1 tasks"
            onCheckedChange={(c) => setNextSettings({ undated_p1: (nextP1 = c === true) })}
          />
        {/snippet}
        {@render row('Deadline window', 'Days ahead a deadline counts as next.', deadlineControl)}
        {@render row('Due window', 'Days ahead a due date counts as next.', dueControl)}
        {@render row('Undated P1 tasks', "Include P1 tasks with no date in What's next.", p1Control)}
      </div>
    </details>
    <section id="updates" class="panel scroll-mt-4">
      <h2 class="panel-title">Version and updates</h2>
      <dl class="grid gap-2 text-sm sm:grid-cols-[10rem_1fr]">
        <dt class="text-muted">App version</dt>
        <dd>{appVersion}</dd>
        <dt class="text-muted">UI version</dt>
        <dd>{uiHash}</dd>
        <dt class="text-muted">Server version</dt>
        <dd>{updates.version?.server ?? 'Unavailable'}</dd>
        <dt class="text-muted">Protocol</dt>
        <dd>
          {updates.version ? `${updates.version.min_client_protocol}–${updates.version.protocol}` : 'Unavailable'}
        </dd>
      </dl>
      {#if updates.upgrade}
        <p role="status" class="mt-3 text-sm text-muted">
          Update required: server requires protocol {updates.upgrade.min_client_protocol}; this app is protocol {updates
            .upgrade.client_protocol}. Queued changes remain stored on this device.
        </p>
        <a class="link mt-2 inline-block" href={updates.upgrade.upgrade} target="_blank" rel="noreferrer"
          >Get the required update</a
        >
      {:else if updates.version?.ui && updates.version.ui !== 'unknown' && updates.version.ui !== uiHash}
        <p role="status" class="mt-3 text-sm text-muted">A newer Web/PWA build is deployed. Reload to use it.</p>
      {/if}
      {#if updates.version?.min_shell_version && updates.version.min_shell_version !== '0.0.0'}
        <p class="mt-2 text-sm text-muted">Minimum shell version: {updates.version.min_shell_version}</p>
      {/if}
      <a
        class="link mt-2 inline-block"
        href="https://github.com/moneytosms/orbit/blob/main/docs/releasing.md"
        target="_blank"
        rel="noreferrer">Compatibility and update policy</a
      >
      {#if isDesktopTauri()}
        {#snippet updateChecksControl()}
          <Switch
            checked={autoUpdates}
            aria-label="Automatically check for desktop updates"
            onCheckedChange={(value) => void setAutoUpdates(value === true)}
          />
        {/snippet}
        <div class="mt-3 border-t border-line pt-1">
          {@render row(
            'Automatic desktop updates',
            'Check on launch and every 6 hours. Signed updates are staged until restart.',
            updateChecksControl,
          )}
        </div>
      {/if}
    </section>

    <AboutSection />
  </div>
</div>
