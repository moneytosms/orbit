<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Flame from '@lucide/svelte/icons/flame'
  import { routines, routineIcon, routineStats } from '$lib/sync'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  let { version = 0, onmeta }: { version?: number; onmeta?: (value: string) => void } = $props()
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
      out.push({ id: r.id, name: `${routineIcon(r)} ${r.name}`, current: s.current_streak, best: s.best_streak })
    }
    items = out
    loaded = true
    onmeta?.(
      `${items.filter((r) => r.current > 0).length} active streaks · 🔥 ${Math.max(0, ...items.map((r) => r.current))} days best`,
    )
  }
</script>

{#if loaded && items.length === 0}
  <Empty
    icon={Flame}
    text="Streaks count the days in a row you keep a routine."
    label="New routine"
    href="#/routines"
  />
{:else if !loaded}
  <WidgetSkeleton />
{:else}
  <ul class="grid gap-0.5">
    {#each items as r (r.id)}
      <li class="flex min-h-[2rem] items-center gap-3 px-1">
        <span class="min-w-0 flex-1 truncate">{r.name}</span>
        <span class="shrink-0 text-sm font-semibold">{r.current}d</span>
        <span class="shrink-0 text-xs text-muted">best {r.best}d</span>
      </li>
    {/each}
  </ul>
{/if}
