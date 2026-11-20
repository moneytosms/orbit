<script lang="ts">
  import { glance, type Glance } from '$lib/dashboard'
  import { goals, localDay, routines, routineStats, stardust, visibleSnapshot } from '$lib/sync'

  let { version = 0 }: { version?: number } = $props()
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

<dl class="grid grid-cols-2 gap-x-4 gap-y-1 sm:grid-cols-5">
  {#each stats as [label, n] (label)}
    <div>
      <dd class="text-xl font-semibold leading-tight">{n}</dd>
      <dt class="text-xs text-muted">{label}</dt>
    </div>
  {/each}
</dl>
