<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { clearFeed, feedHealth, setFeed, type FeedHealth } from '$lib/auth'

  let url = $state('')
  let busy = $state(false)
  let error = $state('')
  let health = $state<FeedHealth | null>(null)

  feedHealth().then(
    (h) => (health = h),
    () => (error = 'Could not load the calendar status.'),
  )

  async function run(job: () => Promise<FeedHealth>) {
    busy = true
    error = ''
    try {
      health = await job()
      url = ''
    } catch (e) {
      error = e instanceof Error ? e.message : 'Something went wrong.'
    } finally {
      busy = false
    }
  }
  const save = (e: SubmitEvent) => (e.preventDefault(), run(() => setFeed(url)))
  const when = (ms: number) => new Date(ms).toLocaleString()
</script>

<p class="text-[0.7812rem] text-muted">
  Paste the secret iCal address from Google Calendar (Settings, your calendar, Integrate calendar, Secret address in
  iCal format). Orbit polls it and shows the events read-only. Anyone with the address can read your calendar, so Orbit
  keeps it on the server and never shows it again.
</p>
<form onsubmit={save} class="mt-3 flex gap-2">
  <Input
    type="password"
    bind:value={url}
    placeholder={health?.configured ? 'Paste a new address to replace it' : 'https://calendar.google.com/…/basic.ics'}
    aria-label="Secret iCal address"
    autocomplete="off"
    required
  />
  <Button type="submit" disabled={busy}>{health?.configured ? 'Replace' : 'Save'}</Button>
  {#if health?.configured}
    <Button type="button" variant="outline" disabled={busy} onclick={() => run(clearFeed)}>Remove</Button>
  {/if}
</form>
{#if error}<p role="alert" class="mt-2 text-warn-text">{error}</p>{/if}
{#if health?.configured}
  <dl class="mt-3 grid gap-2 text-sm sm:grid-cols-[10rem_1fr]" aria-label="Calendar integration health">
    <dt class="text-muted">Feed</dt>
    <dd>{health.url_masked}</dd>
    <dt class="text-muted">Last sync</dt>
    <dd>{health.last_sync_ms === null ? 'Not yet' : when(health.last_sync_ms)}</dd>
    <dt class="text-muted">Events</dt>
    <dd>{health.event_count}</dd>
    <dt class="text-muted">Status</dt>
    <dd class={health.last_error ? 'text-warn-text' : ''}>{health.last_error ?? 'OK'}</dd>
  </dl>
{/if}
