<script lang="ts">
  import CalendarDays from '@lucide/svelte/icons/calendar-days'
  import { feedEvents, type FeedEvent } from '$lib/auth'

  let { version = 0 }: { version?: number } = $props()
  let events = $state<FeedEvent[]>([])

  $effect(() => {
    void version
    const d = new Date()
    const from = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
    // All-day events sit at UTC midnight of their date, so match them on that date instead of the local day.
    const today = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate())).getTime()
    feedEvents(Math.min(from, today) - 1, Math.max(from, today) + 86_400_000).then(
      (all) =>
        (events = all.filter((e) =>
          e.all_day ? e.start_ms <= today && e.end_ms > today : e.start_ms < from + 86_400_000 && e.end_ms > from,
        )),
      () => (events = []),
    )
  })
  const time = (e: FeedEvent) =>
    e.all_day ? 'All day' : new Date(e.start_ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
</script>

{#if events.length}
  <ul class="mb-2 grid gap-0.5" aria-label="Today's calendar events">
    {#each events as e (e.uid + e.start_ms)}
      <li class="flex items-center gap-2 rounded-md border border-dashed border-line px-2 py-1 text-sm italic">
        <CalendarDays size={14} class="shrink-0 text-icon" />
        <span class="shrink-0 text-muted">{time(e)}</span>
        <span class="truncate">{e.summary || '(no title)'}</span>
      </li>
    {/each}
  </ul>
{/if}
