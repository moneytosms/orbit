<script lang="ts">
  import type { LinkItem } from './Mention'

  let { items, command }: { items: LinkItem[]; command: (item: LinkItem) => void } = $props()

  let selected = $state(0)
  $effect(() => {
    void items
    selected = 0
  })

  /** Handles a key for the editor; true when the list used it. */
  export function onKeyDown(e: KeyboardEvent): boolean {
    if (!items.length) return false
    if (e.key === 'ArrowDown') selected = (selected + 1) % items.length
    else if (e.key === 'ArrowUp') selected = (selected - 1 + items.length) % items.length
    else if (e.key === 'Enter' || e.key === 'Tab') command(items[selected])
    else return false
    return true
  }
</script>

<ul
  role="listbox"
  aria-label="Link suggestions"
  class="min-w-48 rounded-lg border border-line bg-surface p-1 shadow-lg"
>
  {#each items as item, i (item.create ? 'create' : item.id)}
    <li role="option" aria-selected={i === selected}>
      <button
        type="button"
        class="flex w-full items-center justify-between gap-3 rounded px-2 py-1 text-left text-sm {i === selected
          ? 'bg-sunk'
          : ''}"
        onmousedown={(e) => {
          e.preventDefault()
          command(item)
        }}
      >
        <span>{item.create ? `Create Page “${item.label}”` : item.label}</span>
        {#if !item.create}<span class="text-xs text-muted">{item.kind}</span>{/if}
      </button>
    </li>
  {:else}
    <li class="px-2 py-1 text-sm text-muted">No matches</li>
  {/each}
</ul>
