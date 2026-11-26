<script lang="ts">
  import Tag from '@lucide/svelte/icons/tag'
  import Empty from '$lib/Empty.svelte'
  import { labelUsage, labelView } from '$lib/sync'
  import { topLabels } from '$lib/widgetData'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** The most used Labels with their open Task counts. A chip opens a View of the Label's Tasks, made on first use. */
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
  const open = async (id: string, name: string) => (location.hash = `#/view/${await labelView(id, name)}`)
</script>

{#if !loaded}
  <WidgetSkeleton rows={1} />
{:else if items.length === 0}
  <Empty icon={Tag} text="No labels in use yet. Add #label to a task to see it here." />
{:else}
  <ul class="flex flex-wrap gap-1.5">
    {#each items as { label, tasks } (label.id)}
      <li>
        <button
          type="button"
          onclick={() => open(label.id, label.name)}
          class="flex items-center gap-1.5 rounded-full bg-accent-soft px-2.5 py-0.5 text-xs text-accent-text hover:brightness-95"
        >
          <span>#{label.name}</span><span class="text-muted">{tasks}</span>
        </button>
      </li>
    {/each}
  </ul>
{/if}
