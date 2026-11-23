<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import Shell from './lib/Shell.svelte'
  import Auth from './lib/screens/Auth.svelte'
  import Banner from './lib/Banner.svelte'
  import Backlinks from './lib/Backlinks.svelte'
  import Area from './lib/screens/Area.svelte'
  import Goals from './lib/screens/Goals.svelte'
  import Roadmap from './lib/screens/Roadmap.svelte'
  import Routines from './lib/screens/Routines.svelte'
  import Page from './lib/screens/Page.svelte'
  import Inbox from './lib/screens/Inbox.svelte'
  import Dashboard from './lib/screens/Dashboard.svelte'
  import Settings from './lib/screens/Settings.svelte'
  import Trash from './lib/screens/Trash.svelte'
  import View from './lib/screens/View.svelte'
  import Profile from './lib/screens/Profile.svelte'
  import Stub from './lib/screens/Stub.svelte'
  import { hasDevice, needsSetup } from './lib/auth'
  import { readScreen, readTaskLink, routes, type Screen } from './lib/routes'
  import TaskDrawer from './lib/TaskDrawer.svelte'
  import {
    awardUp,
    badgeShelf,
    getGamification,
    getSettings,
    levelUp,
    onRejected,
    onTaskCompleted,
    onSignedOut,
    start,
    tasksIn,
    tree,
    stardust,
    updateProject,
    updateSettings,
    uncompleteTask,
    views,
    VIEW_UPCOMING,
  } from './lib/sync'
  import type { CompletionUndo } from './lib/sync'
  import type { View as ViewRow } from './lib/sync'
  import { applyRemote, getAccent, getTheme, onAppearanceChange } from './lib/theme'
  import { refreshNotifier, startNotifier } from './lib/notify'
  import { inbox_id } from '../wasm/orbit_wasm'
  import UpdateNotice from './lib/UpdateNotice.svelte'
  import CaptureOverlay from './lib/CaptureOverlay.svelte'
  import { inTauri, isAndroidTauri } from './lib/platform'

  const captureOverlay = inTauri() && !isAndroidTauri() && getCurrentWindow().label === 'capture'
  // The shells load the bundled UI only to hop to the saved server; showing its login there flashed (#89).
  const launcher = inTauri() && (location.protocol === 'tauri:' || location.hostname === 'tauri.localhost')

  let gate: 'loading' | 'setup' | 'login' | 'app' = $state('loading')
  let pending = $state(0)
  let areas: Awaited<ReturnType<typeof tree>> = $state([])
  let inboxCount = $state(0)
  let rows: Awaited<ReturnType<typeof tasksIn>> = $state([])
  let allViews = $state<ViewRow[]>([])
  let screen: Screen = $state(readScreen())
  let linkedTask = $state(readTaskLink())
  let toast = $state<{ text: string; undo?: CompletionUndo; sparkle?: boolean } | null>(null)
  let version = $state(0)
  let hide: ReturnType<typeof setTimeout> | undefined
  let latestCompletion: CompletionUndo['completion_hlc'] | undefined

  function showToast(next: NonNullable<typeof toast>, duration = 5000) {
    toast = next
    clearTimeout(hide)
    hide = setTimeout(() => (toast = null), duration)
  }

  onRejected((code) => {
    showToast({ text: `A change was rejected (${code.replaceAll('_', ' ')}) and has been undone.` })
  })
  onTaskCompleted((undo) => {
    if (latestCompletion) {
      const old = latestCompletion
      const next = undo.completion_hlc
      const order = old.wall - next.wall || old.counter - next.counter || old.device.localeCompare(next.device)
      if (order > 0) return
    }
    latestCompletion = undo.completion_hlc
    if (matchMedia?.('(pointer: coarse)').matches && 'vibrate' in navigator) {
      try { navigator.vibrate(8) } catch { /* unsupported by this browser */ }
    }
    showToast({ text: 'Task completed', undo, sparkle: true })
  })
  onAppearanceChange((p) => updateSettings(p))
  onSignedOut((n) => {
    pending = n
    gate = 'login'
  })
  /** The parent whose Tasks the list shows; undefined on screens without a list. */
  const listParent = (s: Screen) =>
    s.name === 'page' || s.name === 'view' || s.name === 'roadmap'
      ? undefined
      : 'id' in s
        ? s.id
        : s.name === 'inbox'
          ? inbox_id()
          : undefined
  const parent = $derived(listParent(screen))
  const title = $derived.by(() => {
    const s = screen
    if (s.name === 'area') return areas.find((a) => a.area.id === s.id)?.area.name ?? 'Area'
    if (s.name === 'project') return areas.flatMap((a) => a.projects).find((p) => p.id === s.id)?.name ?? 'Project'
    if (s.name === 'page') return 'Page'
    if (s.name === 'roadmap') return 'Roadmap'
    if (s.name === 'view') return allViews.find((v) => v.id === s.id)?.name ?? 'View'
    return routes[s.name].label
  })

  const refresh = async () => {
    areas = await tree()
    rows = parent ? await tasksIn(parent) : []
    inboxCount = (await tasksIn(inbox_id())).length
    allViews = await views()
    void refreshNotifier()
    version++
    if (await getGamification()) {
      const sd = await stardust()
      const shelf = await badgeShelf()
      const earned = shelf.badges.reduce((n, b) => n + b.count, 0) + shelf.patches
      if (levelUp(sd.level.n)) {
        const text = `Level up! You reached ${sd.level.name}.`
        showToast({ text: toast?.undo ? `${text} · Task completed` : text, undo: toast?.undo, sparkle: toast?.sparkle })
      } else if (awardUp(earned)) {
        showToast({ text: toast?.undo ? 'New Badge earned · Task completed' : 'New Badge earned.', undo: toast?.undo, sparkle: toast?.sparkle })
      }
    }
    // Theme and accent from another Device. Empty theme: not set; empty accent: theme's own.
    const remote = await getSettings()
    if (remote && ((remote.theme && remote.theme !== getTheme()) || (remote.accent || null) !== getAccent())) {
      applyRemote(remote)
    }
  }

  // A saved start view opens at launch, but only while the address bar names no screen.
  async function applyStartView() {
    if (location.hash) return
    const stored = (await getSettings())?.start_view
    if (!stored) return
    // A hidden start view falls back to the default screen.
    const id =
      stored === 'upcoming' ? VIEW_UPCOMING : stored.startsWith('view:') ? stored.slice('view:'.length) : undefined
    if (id && (await views()).find((v) => v.id === id)?.hidden !== false) return
    location.hash = stored.startsWith('view:') ? `#/view/${stored.slice('view:'.length)}` : `#/${stored}`
    screen = readScreen()
  }

  let started = false
  async function enter() {
    gate = 'app'
    pending = 0
    if (!started) (start(refresh), startNotifier())
    started = true
    await applyStartView()
    refresh()
  }

  // Offline with a Device: straight in. Otherwise ask the server whether this is the first run.
  if (!captureOverlay && !launcher) void (async () => {
    if (await hasDevice()) return enter()
    gate = await needsSetup().then(
      (setup) => (setup ? 'setup' : 'login'),
      () => 'login' as const,
    )
  })()
</script>

<svelte:window
  onhashchange={() => {
    if (captureOverlay) return
    screen = readScreen()
    linkedTask = readTaskLink()
    refresh()
  }}
/>

{#if captureOverlay}
  <CaptureOverlay />
{:else}
{#if gate === 'setup' || gate === 'login'}
  <Auth mode={gate} {pending} done={enter} />
{:else if gate === 'app'}
  <Shell {screen} {title} {areas} {inboxCount} parentId={parent ?? inbox_id()} onchange={refresh}>
    {#snippet children(add)}
      {#if screen.name === 'page'}
        <Page id={screen.id} onchange={refresh} />
      {:else if screen.name === 'roadmap'}
        {#key screen.id}
          <Roadmap id={screen.id} {areas} onchange={refresh} />
        {/key}
      {:else if screen.name === 'roadmaps'}
        <Roadmap {areas} onchange={refresh} />
      {:else if screen.name === 'view'}
        {#key screen.id}
          <View id={screen.id} {areas} onchange={refresh} />
        {/key}
      {:else if screen.name === 'upcoming'}
        <View id={VIEW_UPCOMING} {areas} onchange={refresh} />
      {:else if screen.name === 'area'}
        {#key screen.id}
          {@const areaId = screen.id}
          <Area id={areaId} {areas} onchange={refresh} onnewtask={(text) => add(text, true)}>
            {#snippet tasks()}
              <Inbox parentId={areaId} {title} {rows} onchange={refresh} />
            {/snippet}
          </Area>
        {/key}
      {:else if parent}
        {#if screen.name === 'project'}
          {@const pid = screen.id}
          {@const proj = areas.flatMap((a) => a.projects).find((p) => p.id === pid)}
          {#if proj}
            {#key proj.id}
              <Banner
                icon={proj.icon}
                cover={proj.cover}
                onsave={async (patch) => {
                  await updateProject(proj.id, patch)
                  refresh()
                }}
              />
            {/key}
          {/if}
        {/if}
        <Inbox parentId={parent} {title} {rows} onchange={refresh} />
        {#if screen.name === 'project'}
          <Backlinks id={screen.id} {version} />
        {/if}
      {:else if screen.name === 'goals'}
        <Goals {areas} />
      {:else if screen.name === 'routines'}
        <Routines {areas} />
      {:else if screen.name === 'trash'}
        <Trash onchange={refresh} />
      {:else if screen.name === 'settings'}
        <Settings {version} />
      {:else if screen.name === 'home'}
        <Dashboard {version} onchange={refresh} />
      {:else if screen.name === 'profile'}
        <Profile {version} />
      {:else}
        <Stub {title} />
      {/if}
    {/snippet}
  </Shell>
{/if}

{#if gate === 'app'}
  <TaskDrawer
    taskId={linkedTask}
    onclose={() => {
      linkedTask = null
      history.replaceState(null, '', location.pathname)
    }}
  />
{/if}

<UpdateNotice />

{#if toast}
  <p
    role="status"
    class="fixed bottom-24 left-1/2 z-50 flex max-w-[calc(90vw/var(--zoom,1))] -translate-x-1/2 items-center rounded-lg bg-text py-[9px] pr-2.5 pl-3.5 text-bg shadow-2 desk:bottom-6"
  >
    {#if toast.sparkle}<span aria-hidden="true" class="completion-spark">✦</span>{/if}
    <span>{toast.text}</span>
    {#if toast.undo}
      <button
        class="ml-3 rounded px-1.5 py-0.5 font-semibold underline underline-offset-2"
        onclick={async () => {
          const undo = toast?.undo
          if (!undo) return
          await uncompleteTask(undo)
          showToast({ text: 'Task restored' }, 2500)
        }}
      >Undo</button>
    {/if}
    <button
      class="ml-2 rounded px-1 text-bg/70 hover:text-bg"
      aria-label="Dismiss notification"
      onclick={() => { clearTimeout(hide); toast = null }}
    >×</button>
  </p>
{/if}
{/if}
