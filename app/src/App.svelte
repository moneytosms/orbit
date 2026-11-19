<script lang="ts">
  import { tick } from 'svelte'
  import Shell from './lib/Shell.svelte'
  import Auth from './lib/screens/Auth.svelte'
  import Inbox from './lib/screens/Inbox.svelte'
  import Settings from './lib/screens/Settings.svelte'
  import Stub from './lib/screens/Stub.svelte'
  import { hasDevice, needsSetup } from './lib/auth'
  import { readRoute, routes } from './lib/routes'
  import { inboxTasks, onRejected, onSignedOut, start, type Task } from './lib/sync'

  let gate: 'loading' | 'setup' | 'login' | 'app' = $state('loading')
  let pending = $state(0)
  let tasks: Task[] = $state([])
  let route = $state(readRoute())
  let toast = $state('')
  let hide: ReturnType<typeof setTimeout>

  onRejected((code) => {
    toast = `A change was rejected (${code.replaceAll('_', ' ')}) and has been undone.`
    clearTimeout(hide)
    hide = setTimeout(() => (toast = ''), 5000)
  })
  onSignedOut((n) => {
    pending = n
    gate = 'login'
  })
  const refresh = async () => (tasks = await inboxTasks())

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

<svelte:window onhashchange={() => (route = readRoute())} />

{#if gate === 'setup' || gate === 'login'}
  <Auth mode={gate} {pending} done={enter} />
{:else if gate === 'app'}
  <Shell {route} inboxCount={tasks.length} onNew={newTask}>
    {#if route === 'inbox'}
      <Inbox {tasks} />
    {:else if route === 'settings'}
      <Settings />
    {:else}
      <Stub title={routes[route].label} />
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
