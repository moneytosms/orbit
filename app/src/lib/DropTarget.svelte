<script lang="ts">
  // Lets a Task dragged from a list land on a sidebar entry: an overlay dnd zone of type 'task'.
  // The lib finds zones by geometry, so the overlay ignores pointer events and links still click.
  import type { Snippet } from 'svelte'
  import { dndzone, TRIGGERS, type DndEvent, type Item } from 'svelte-dnd-action'
  import { moveTo } from '$lib/sync'

  let { parentId, children }: { parentId: string; children: Snippet } = $props()
  let items = $state<Item[]>([])

  function consider(e: CustomEvent<DndEvent<Item>>) {
    items = e.detail.items
  }
  function finalize(e: CustomEvent<DndEvent<Item>>) {
    items = []
    const { id, trigger } = e.detail.info
    if (trigger === TRIGGERS.DROPPED_INTO_ZONE) moveTo(String(id), parentId)
  }
</script>

<div class="relative rounded-[7px] {items.length ? 'bg-accent-soft' : ''}">
  {@render children()}
  <div
    class="pointer-events-none absolute inset-0"
    aria-hidden="true"
    use:dndzone={{ items, type: 'task', dragDisabled: true, flipDurationMs: 0, dropTargetStyle: {} }}
    onconsider={consider}
    onfinalize={finalize}
  >
    {#each items as i (i.id)}<div class="h-0 overflow-hidden"></div>{/each}
  </div>
</div>
