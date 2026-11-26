<script lang="ts">
  import Tag from '@lucide/svelte/icons/tag'
  import Empty from '$lib/Empty.svelte'
  import { labelUsage } from '$lib/sync'
  import { topLabels } from '$lib/widgetData'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** The most used Labels with their open Task counts. Chips are plain: there is no per-label view to open. */
  let { version = 0, onmeta }: { version?: number; onmeta?: (value: string) => void } = $props()
  let items = $state<Awaited<ReturnType<typeof labelUsage>>>([])
  let loaded = $state(false)
  $effect(() => {
    void version
    labelUsage().then((u) => {
      items = topLabels(u)
      loaded = true
      onmeta?.(`${items.length} label${items.length === 1 ? '' : 's'} in use`)
    })
  })
</script>

{#if !loaded}
  <WidgetSkeleton rows={1} />
{:else if items.length === 0}
  <Empty icon={Tag} text="No labels in use yet. Add #label to a task to see it here." />
{:else}
  <ul class="flex flex-wrap gap-1.5">
    {#each items as { label, tasks } (label.id)}
      <li class="flex items-center gap-1.5 rounded-full bg-accent-soft px-2.5 py-0.5 text-xs text-accent-text">
        <span>#{label.name}</span><span class="text-muted">{tasks}</span>
      </li>
    {/each}
  </ul>
{/if}
