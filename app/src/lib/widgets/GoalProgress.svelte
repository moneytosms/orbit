<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Target from '@lucide/svelte/icons/target'
  import { goalProgress, goals } from '$lib/sync'

  let { version = 0 }: { version?: number } = $props()
  let items = $state<{ id: string; name: string; pct: number; behind: boolean }[]>([])
  let loaded = $state(false)

  $effect(() => {
    void version
    load()
  })
  async function load() {
    const out = []
    for (const g of await goals()) {
      const { progress, behind } = await goalProgress(g.id)
      out.push({ id: g.id, name: g.name, pct: Math.round(progress * 100), behind })
    }
    items = out
    loaded = true
  }
</script>

{#if loaded && items.length === 0}
  <Empty icon={Target} text="Goals show your progress toward something you want." label="Add a goal" href="#/goals" />
{/if}
<ul class="grid gap-2">
  {#each items as g (g.id)}
    <li>
      <div class="flex items-center gap-2 text-sm">
        <span class="min-w-0 flex-1 truncate">{g.name}</span>
        {#if g.behind}<span class="rounded-full bg-warn-soft px-2 py-px text-xs text-warn-text">Behind</span>{/if}
        <span class="text-xs text-muted">{g.pct}%</span>
      </div>
      <div
        class="mt-1 h-1.5 overflow-hidden rounded-full bg-sunk"
        role="progressbar"
        aria-label="{g.name} progress"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={g.pct}
      >
        <div class="h-full rounded-full bg-accent" style="width: {g.pct}%"></div>
      </div>
    </li>
  {/each}
</ul>
