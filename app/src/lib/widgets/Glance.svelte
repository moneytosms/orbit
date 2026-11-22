<script lang="ts">
  import { glance, type Glance } from '$lib/dashboard'
  import { goals, localDay, routines, routineStats, stardust, visibleSnapshot } from '$lib/sync'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  let { version = 0, onmeta }: { version?: number; onmeta?: (value: string) => void } = $props()
  let g = $state<Glance & { done: number; routinesDue: number }>()

  $effect(() => {
    void version
    load()
  })
  async function load() {
    const today = await localDay()
    const base = glance(await visibleSnapshot(), today)
    let routinesDue = 0
    for (const r of await routines()) {
      const day = (await routineStats(r.id, today)).days.find(([d]) => d === today)?.[1]
      if (day && day !== 'off' && day !== 'paused') routinesDue++
    }
    const activeGoals = (await goals()).length
    g = { ...base, activeGoals, done: (await stardust()).targets.today_done, routinesDue }
    onmeta?.(`${base.dueToday} due · ${base.overdue} overdue · ${activeGoals} active goals`)
  }
  const stats = $derived(
    g
      ? [
          ['Due today', g.dueToday],
          ['Overdue', g.overdue],
          ['Done today', g.done],
          ['Active goals', g.activeGoals],
          ['Routines today', g.routinesDue],
        ]
      : [],
  )
</script>

{#if g}
  <dl class="grid grid-cols-2 gap-x-2 gap-y-0 sm:grid-cols-5">
    {#each stats as [label, n] (label)}
      <div>
        <dd class="text-base font-semibold leading-tight">{n}</dd>
        <dt class="text-[0.6875rem] text-muted">{label}</dt>
      </div>
    {/each}
  </dl>
{:else}
  <WidgetSkeleton rows={1} />
{/if}
