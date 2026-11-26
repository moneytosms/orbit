<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { req, tx } from './db'
  import { checkServerVersion, currentUpdates, loadUpdates, onUpdates, uiHash, type UpdateState } from './updates'
  import { checkAndroidShellRelease, type AndroidShellRelease } from './androidRelease'
  import { isAndroidTauri } from './platform'
  import { Button } from '$lib/components/ui/button'

  let updates: UpdateState = $state(currentUpdates())
  let pending = $state(0)
  let waitingWorker: ServiceWorker | undefined
  let reloadAfterActivation = false
  let swWaiting = $state(false)
  let shellRelease = $state<AndroidShellRelease | null>(null)
  const readPending = async () => (pending = await tx(['outbox'], (s) => req(s('outbox').count())))

  onMount(() => {
    const off = onUpdates((value) => (updates = value))
    void loadUpdates().then(readPending)
    void checkServerVersion().catch(() => {})
    if (isAndroidTauri()) {
      void invoke<string>('shell_version')
        .then((version) => (version ? checkAndroidShellRelease(version) : null))
        .then((release) => (shellRelease = release))
        .catch(() => {})
    }
    const poll = setInterval(() => void checkServerVersion().catch(() => {}), 300_000)
    const queuePoll = setInterval(() => void readPending(), 2_000)
    const changed = () => void readPending()
    addEventListener('focus', changed)
    const workerChanged = () => {
      if (reloadAfterActivation && navigator.serviceWorker.controller) location.reload()
    }
    navigator.serviceWorker?.addEventListener('controllerchange', workerChanged)
    void navigator.serviceWorker?.register('/sw.js').then((registration) => {
      const checkWaiting = () => {
        if (registration.waiting && navigator.serviceWorker.controller) {
          waitingWorker = registration.waiting
          swWaiting = true
        }
      }
      checkWaiting()
      registration.addEventListener('updatefound', () => {
        const installing = registration.installing
        installing?.addEventListener('statechange', checkWaiting)
      })
      setInterval(() => void registration.update(), 300_000)
    })
    return () => {
      off()
      clearInterval(poll)
      clearInterval(queuePoll)
      removeEventListener('focus', changed)
      navigator.serviceWorker?.removeEventListener('controllerchange', workerChanged)
    }
  })

  const uiChanged = $derived(!!updates.version?.ui && updates.version.ui !== 'unknown' && updates.version.ui !== uiHash)
  const notice = $derived(!!updates.upgrade || uiChanged || swWaiting || !!shellRelease)
  const reloadUpdate = () => {
    if (!waitingWorker) return location.reload()
    reloadAfterActivation = true
    waitingWorker.postMessage({ type: 'SKIP_WAITING' })
  }
</script>

{#if notice}
  <aside
    role="alert"
    class="fixed inset-x-3 bottom-3 z-50 mx-auto max-w-xl rounded-xl border border-line-strong bg-surface p-4 shadow-2 desk:bottom-6"
  >
    <p class="font-semibold">Orbit update available</p>
    {#if updates.upgrade}
      <p class="mt-1 text-sm text-muted">
        This server requires sync protocol {updates.upgrade.min_client_protocol}; this app uses {updates.upgrade
          .client_protocol}.
        {pending} queued {pending === 1 ? 'change' : 'changes'} will stay on this device until you update.
      </p>
      <Button size="sm" class="mt-3" href={updates.upgrade.upgrade} target="_blank" rel="noreferrer"
        >Get the required update</Button
      >
    {:else if uiChanged}
      <p class="mt-1 text-sm text-muted">A newer app build is available. Reload Orbit to use it.</p>
    {:else if swWaiting}
      <p class="mt-1 text-sm text-muted">A new app version is ready to install.</p>
    {:else}
      <p class="mt-1 text-sm text-muted">A newer Android app is available.</p>
    {/if}
    {#if swWaiting || uiChanged}
      <Button size="sm" class="mt-3" onclick={reloadUpdate}>Reload to update</Button>
    {/if}
    {#if shellRelease}
      <a
        class="mt-2 inline-flex rounded-full border border-accent-line bg-accent-soft px-3 py-1 text-sm font-medium text-accent-text"
        href={shellRelease.url}
        target="_blank"
        rel="noreferrer">Update available · Download APK {shellRelease.version}</a
      >
    {/if}
  </aside>
{/if}
