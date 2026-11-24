<script lang="ts">
  import { feedEvents, type FeedEvent } from '$lib/auth'
  import { soon } from '$lib/calendar'

  let {
    version = 0,
    onmeta,
    onshown,
  }: { version?: number; onmeta?: (value: string) => void; onshown?: (shown: boolean) => void } = $props()
  let groups = $state<{ label: string; events: FeedEvent[] }[]>([])

  $effect(() => {
    void version
    const now = new Date()
    const from = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()
    // A day of slack each side: all-day events are matched on their dates, not these instants.
    feedEvents(from - 86_400_000, from + 3 * 86_400_000).then(
      (all) => show(soon(all, now)),
      () => show({ today: [], tomorrow: [], shown: false }),
    )
  })
  function show(s: ReturnType<typeof soon>) {
    groups = [
      { label: 'Today', events: s.today },
      { label: 'Tomorrow', events: s.tomorrow },
    ].filter((g) => g.events.length)
    onshown?.(s.shown)
    onmeta?.(s.shown ? `${s.today.length} today, ${s.tomorrow.length} tomorrow` : 'Nothing soon')
  }
  const time = (e: FeedEvent) =>
    e.all_day ? 'All day' : new Date(e.start_ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
</script>

{#each groups as g (g.label)}
  <p class="text-xs font-medium text-muted">{g.label}</p>
  <ul class="mb-1 grid gap-0.5" aria-label="{g.label}'s calendar events">
    {#each g.events as e (e.feed_id + e.uid + e.start_ms)}
      <li class="flex items-center gap-2 text-sm" title={e.feed_name}>
        <span class="size-2 shrink-0 rounded-full" style="background: var(--{e.color})"></span>
        <span class="w-16 shrink-0 text-xs text-muted">{time(e)}</span>
        <span class="truncate">{e.summary || '(no title)'}</span>
      </li>
    {/each}
  </ul>
{/each}
