<script lang="ts">
  import Route from '@lucide/svelte/icons/route'
  import Empty from '$lib/Empty.svelte'
  import { roadmapsIn } from '$lib/area'
  import { visibleSnapshot, type Roadmap } from '$lib/sync'

  /** Roadmaps owned by an Area, or by one of its Projects or Goals. */
  let {
    parentId,
    version = 0,
    onmeta,
  }: { parentId: string; version?: number; onmeta?: (value: string) => void } = $props()
  let maps = $state<Roadmap[]>([])
  $effect(() => {
    void version
    const id = parentId
    visibleSnapshot().then((rows) => {
      if (id !== parentId) return
      maps = roadmapsIn(
        rows,
        rows.filter((r): r is Roadmap => r.kind === 'roadmap'),
        id,
      )
      onmeta?.(`${maps.length} roadmap${maps.length === 1 ? '' : 's'}`)
    })
  })
</script>

{#if maps.length}
  <ul class="grid gap-1">
    {#each maps as r (r.id)}
      <li><a href="#/roadmap/{r.id}" class="link text-sm">{r.name}</a></li>
    {/each}
  </ul>
{:else}
  <Empty icon={Route} text="No roadmaps here yet." />
{/if}
