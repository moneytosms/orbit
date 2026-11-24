<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import { addFeed, feeds, FEED_COLORS, removeFeed, updateFeed, type Feed } from '$lib/auth'

  let list = $state<Feed[]>([])
  let name = $state('')
  let url = $state('')
  let busy = $state(false)
  let error = $state('')

  const load = () =>
    feeds().then(
      (f) => (list = f),
      () => (error = 'Could not load your calendars.'),
    )
  load()

  async function run(job: () => Promise<unknown>) {
    busy = true
    error = ''
    try {
      await job()
      await load()
    } catch (e) {
      error = e instanceof Error ? e.message : 'Something went wrong.'
    } finally {
      busy = false
    }
  }
  const add = (e: SubmitEvent) => (
    e.preventDefault(),
    run(async () => {
      await addFeed(name, url)
      name = url = ''
    })
  )
  const patch = (f: Feed, p: Parameters<typeof updateFeed>[1]) => run(() => updateFeed(f.id, p))
  const rename = (f: Feed, e: Event) => {
    const v = (e.currentTarget as HTMLInputElement).value.trim()
    if (v && v !== f.name) patch(f, { name: v })
  }
  const when = (ms: number) => new Date(ms).toLocaleString()
</script>

<p class="text-[0.7812rem] text-muted">
  In Google Calendar open Settings, pick the calendar, then Integrate calendar and copy "Secret address in iCal format".
  Orbit polls it and shows the events read-only on Home and Calendar, never in your tasks. Anyone with the address can
  read that calendar, so Orbit keeps it on the server and never shows it again.
</p>

{#if error}<p role="alert" class="mt-2 text-warn-text">{error}</p>{/if}

<ul class="mt-3 grid gap-3" aria-label="Calendars">
  {#each list as f (f.id)}
    <li class="grid gap-2 rounded-md border border-line p-3">
      <div class="flex items-center gap-2">
        <Checkbox
          checked={f.visible}
          onCheckedChange={(v) => patch(f, { visible: v === true })}
          aria-label="Show {f.name}"
          disabled={busy}
        />
        <Input value={f.name} maxlength={60} aria-label="Name of {f.name}" onchange={(e) => rename(f, e)} />
        <Button type="button" variant="destructive" disabled={busy} onclick={() => run(() => removeFeed(f.id))}
          >Remove</Button
        >
      </div>
      <div class="flex gap-1.5" role="radiogroup" aria-label="Color of {f.name}">
        {#each FEED_COLORS as c (c)}
          <button
            type="button"
            role="radio"
            aria-checked={f.color === c}
            aria-label={c}
            disabled={busy}
            class="size-6 rounded-full border-2 {f.color === c ? 'border-text' : 'border-transparent'}"
            style="background: var(--{c})"
            onclick={() => f.color !== c && patch(f, { color: c })}
          ></button>
        {/each}
      </div>
      <p class="text-xs text-muted">
        {f.url_masked} · {f.event_count} events · Last sync {f.last_sync_ms === null ? 'not yet' : when(f.last_sync_ms)}
      </p>
      {#if f.last_error}<p class="text-xs text-warn-text">Last error: {f.last_error}</p>{/if}
    </li>
  {/each}
</ul>

<form onsubmit={add} class="mt-3 grid gap-2 sm:grid-cols-[10rem_1fr_auto]">
  <Input bind:value={name} placeholder="Name" maxlength={60} aria-label="New calendar name" required />
  <Input
    type="password"
    bind:value={url}
    placeholder="https://calendar.google.com/…/basic.ics"
    aria-label="Secret iCal address"
    autocomplete="off"
    required
  />
  <Button type="submit" disabled={busy}>Add calendar</Button>
</form>
