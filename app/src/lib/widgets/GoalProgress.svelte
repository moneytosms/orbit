<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Target from '@lucide/svelte/icons/target'
  import { Input } from '$lib/components/ui/input'
  import { underArea } from '$lib/area'
  import { createGoal, goalProgress, goals, visibleSnapshot } from '$lib/sync'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** With `parentId` only the Goals under that Area (or Project) show, and a field adds one there. */
  let {
    version = 0,
    parentId,
    onmeta,
    onchange,
  }: { version?: number; parentId?: string; onmeta?: (value: string) => void; onchange?: () => void } = $props()
  let draft = $state('')
  let items = $state<{ id: string; name: string; pct: number; behind: boolean }[]>([])
  let loaded = $state(false)

  $effect(() => {
    void version
    load()
  })
  async function load() {
    const out = []
    const under = parentId ? underArea(await visibleSnapshot(), parentId) : null
    for (const g of (await goals()).filter((g) => !under || under.has(g.id))) {
      const { progress, behind } = await goalProgress(g.id)
      out.push({ id: g.id, name: g.name, pct: Math.round(progress * 100), behind })
    }
    items = out
    loaded = true
    onmeta?.(`${items.length} active goal${items.length === 1 ? '' : 's'}`)
  }
  async function add(e: SubmitEvent) {
    e.preventDefault()
    const name = draft.trim()
    if (!name || !parentId) return
    draft = ''
    await createGoal(parentId, name)
    await load()
    onchange?.()
  }
</script>

{#if loaded && items.length === 0}
  {#if parentId}
    <Empty icon={Target} text="Goals show your progress toward something you want here." />
  {:else}
    <Empty icon={Target} text="Goals show your progress toward something you want." label="Add a goal" href="#/goals" />
  {/if}
{:else if !loaded}
  <WidgetSkeleton />
{:else}
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
{/if}
{#if parentId && loaded}
  <form class="mt-3" onsubmit={add}>
    <Input bind:value={draft} placeholder="Add a goal" aria-label="New goal" />
  </form>
{/if}
