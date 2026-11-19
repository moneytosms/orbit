<script lang="ts">
  import { tick } from 'svelte'
  import Shell from './lib/Shell.svelte'
  import Auth from './lib/screens/Auth.svelte'
  import Inbox from './lib/screens/Inbox.svelte'
  import Settings from './lib/screens/Settings.svelte'
  import Stub from './lib/screens/Stub.svelte'
  import { hasDevice, needsSetup } from './lib/auth'
  import { readScreen, routes, type Screen } from './lib/routes'
  import { getSettings, onRejected, onSignedOut, start, tasksIn, tree, updateSettings } from './lib/sync'
  import { applyRemote, getAccent, getTheme, onAppearanceChange } from './lib/theme'
  import { inbox_id } from '../wasm/orbit_wasm'

  let gate: 'loading' | 'setup' | 'login' | 'app' = $state('loading')
  let pending = $state(0)
  let areas: Awaited<ReturnType<typeof tree>> = $state([])
  let inboxCount = $state(0)
  let rows: Awaited<ReturnType<typeof tasksIn>> = $state([])
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
  const listParent = (s: Screen) => ('id' in s ? s.id : s.name === 'inbox' ? inbox_id() : undefined)
  const parent = $derived(listParent(screen))
  const title = $derived.by(() => {
    const s = screen
    if (s.name === 'area') return areas.find((a) => a.area.id === s.id)?.area.name ?? 'Area'
    if (s.name === 'project') return areas.flatMap((a) => a.projects).find((p) => p.id === s.id)?.name ?? 'Project'
    return routes[s.name].label
  })

  const refresh = async () => {
    areas = await tree()
    rows = parent ? await tasksIn(parent) : []
    inboxCount = (await tasksIn(inbox_id())).length
    // Theme and accent from another Device; empty means not set.
    const remote = await getSettings()
    if ((remote.theme && remote.theme !== getTheme()) || (remote.accent && remote.accent !== getAccent())) {
      applyRemote(remote)
    }
  }

  let started = false
  function enter() {
    gate = 'app'
    pending = 0
    if (!started) start(refresh)
    started = true
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

  async function newTask() {
    location.hash = '#/inbox'
    await tick()
    document.getElementById('composer')?.focus()
  }
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
  <Shell {screen} {title} {areas} {inboxCount} onNew={newTask}>
    {#if parent}
      <Inbox parentId={parent} {title} {rows} onchange={refresh} />
    {:else if screen.name === 'settings'}
      <Settings />
    {:else}
      <Stub {title} />
    {/if}
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
