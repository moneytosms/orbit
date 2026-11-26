<script lang="ts">
  import { areaStats, type AreaStats } from '$lib/area'
  import { localDay, visibleSnapshot } from '$lib/sync'

  /** Open, done this week, overdue and active Goals under an Area or Project. */
  let { parentId, version = 0 }: { parentId: string; version?: number } = $props()
  let stats = $state<AreaStats>({ open: 0, completedThisWeek: 0, overdue: 0, activeGoals: 0 })
  $effect(() => {
    void version
    const id = parentId
    Promise.all([visibleSnapshot(), localDay()]).then(([rows, today]) => {
      if (id === parentId) stats = areaStats(rows, id, today, Date.now())
    })
  })
  const tiles = $derived([
    ['Open tasks', stats.open],
    ['Done this week', stats.completedThisWeek],
    ['Overdue', stats.overdue],
    ['Active goals', stats.activeGoals],
  ] as const)
</script>

<ul class="grid h-full grid-cols-2 items-center gap-2 desk:grid-cols-4">
  {#each tiles as [label, n] (label)}
    <li>
      <div class="text-xl font-semibold">{n}</div>
      <div class="text-xs text-muted">{label}</div>
    </li>
  {/each}
</ul>
