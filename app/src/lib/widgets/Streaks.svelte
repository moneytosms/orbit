<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Flame from '@lucide/svelte/icons/flame'
  import { routines, routineStats } from '$lib/sync'

  let { version = 0 }: { version?: number } = $props()
  let items = $state<{ id: string; name: string; current: number; best: number }[]>([])
  let loaded = $state(false)

  $effect(() => {
    void version
    load()
  })
  async function load() {
    const out = []
    for (const r of await routines()) {
      const s = await routineStats(r.id)
      out.push({ id: r.id, name: r.name, current: s.current_streak, best: s.best_streak })
    }
    items = out
    loaded = true
  }
</script>

{#if loaded && items.length === 0}
  <Empty
    icon={Flame}
    text="Streaks count the days in a row you keep a routine."
    label="New routine"
    href="#/routines"
  />
{/if}
<ul class="grid gap-0.5">
  {#each items as r (r.id)}
    <li class="flex min-h-[32px] items-center gap-3 px-1">
      <span class="min-w-0 flex-1 truncate">{r.name}</span>
      <span class="shrink-0 text-sm font-semibold">{r.current}d</span>
      <span class="shrink-0 text-xs text-muted">best {r.best}d</span>
    </li>
  {/each}
</ul>
