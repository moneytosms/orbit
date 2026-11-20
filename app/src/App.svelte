<script lang="ts">
  import Shell from './lib/Shell.svelte'
  import Auth from './lib/screens/Auth.svelte'
  import Area from './lib/screens/Area.svelte'
  import Goals from './lib/screens/Goals.svelte'
  import Roadmap from './lib/screens/Roadmap.svelte'
  import Routines from './lib/screens/Routines.svelte'
  import Page from './lib/screens/Page.svelte'
  import Inbox from './lib/screens/Inbox.svelte'
  import Next from './lib/screens/Next.svelte'
  import Settings from './lib/screens/Settings.svelte'
  import Trash from './lib/screens/Trash.svelte'
  import View from './lib/screens/View.svelte'
  import Stub from './lib/screens/Stub.svelte'
  import { hasDevice, needsSetup } from './lib/auth'
  import { readScreen, routes, type Screen } from './lib/routes'
  import {
    getSettings,
    onRejected,
    onSignedOut,
    start,
    tasksIn,
    tree,
    updateSettings,
    views,
    VIEW_UPCOMING,
  } from './lib/sync'
  import type { View as ViewRow } from './lib/sync'
  import { applyRemote, getAccent, getTheme, onAppearanceChange } from './lib/theme'
  import { refreshNotifier, startNotifier } from './lib/notify'
  import { inbox_id } from '../wasm/orbit_wasm'

  let gate: 'loading' | 'setup' | 'login' | 'app' = $state('loading')
  let pending = $state(0)
  let areas: Awaited<ReturnType<typeof tree>> = $state([])
  let inboxCount = $state(0)
  let rows: Awaited<ReturnType<typeof tasksIn>> = $state([])
  let allViews = $state<ViewRow[]>([])
  let screen: Screen = $state(readScreen())
  let toast = $state('')
  let hide: ReturnType<typeof setTimeout>

  onRejected((code) => {
    toast = `A change was rejected (${code.replaceAll('_', ' ')}) and has been undone.`
    clearTimeout(hide)
    hide = setTimeout(() => (toast = ''), 5000)
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
  ;(async () => {
    if (await hasDevice()) return enter()
    gate = await needsSetup().then(
      (setup) => (setup ? 'setup' : 'login'),
      () => 'login' as const,
    )
  })()
</script>

<svelte:window
  onhashchange={() => {
    screen = readScreen()
    refresh()
  }}
/>

{#if gate === 'setup' || gate === 'login'}
  <Auth mode={gate} {pending} done={enter} />
{:else if gate === 'app'}
  <Shell {screen} {title} {areas} {inboxCount} parentId={parent ?? inbox_id()}>
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
          <Area id={areaId} {areas} onchange={refresh}>
            {#snippet tasks()}
              <Inbox parentId={areaId} {title} {rows} onchange={refresh} onadd={add} />
            {/snippet}
          </Area>
        {/key}
      {:else if parent}
        <Inbox parentId={parent} {title} {rows} onchange={refresh} onadd={add} />
      {:else if screen.name === 'goals'}
        <Goals {areas} />
      {:else if screen.name === 'routines'}
        <Routines {areas} />
      {:else if screen.name === 'trash'}
        <Trash onchange={refresh} />
      {:else if screen.name === 'settings'}
        <Settings />
      {:else if screen.name === 'home'}
        <Next onchange={refresh} />
      {:else}
        <Stub {title} />
      {/if}
    {/snippet}
  </Shell>
{/if}

{#if toast}
  <p
    role="status"
    class="fixed bottom-24 left-1/2 z-50 max-w-[90vw] -translate-x-1/2 rounded-lg bg-text py-[9px] pr-2.5 pl-3.5 text-bg shadow-2 desk:bottom-6"
  >
    {toast}
  </p>
{/if}
