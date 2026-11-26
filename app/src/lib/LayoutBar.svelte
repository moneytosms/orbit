<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus'
  import { Button } from '$lib/components/ui/button'
  import { colsFor } from '$lib/dashboard'

  let {
    kinds,
    hidden = [],
    views,
    onadd,
    onaddview,
    onshow,
    onreset,
    ondone,
  }: {
    /** Tile kinds the page offers in the Add widget menu. */
    kinds: { kind: string; title: string }[]
    /** Tiles taken off the page, to put back. */
    hidden?: { id: string; title: string }[]
    /** Saved Views to add as tiles (Home only). */
    views?: { id: string; name: string }[]
    onadd: (kind: string) => void
    onaddview?: (viewId: string) => void
    onshow: (id: string) => void
    onreset: () => void
    ondone: () => void
  } = $props()

  let width = $state(1280)
  let adding = $state(false)
  const phone = $derived(colsFor(width) < 4)
</script>

<svelte:window bind:innerWidth={width} />

<div
  role="toolbar"
  aria-label="Edit layout"
  class="mb-3 flex flex-wrap items-center gap-2 rounded-xl border border-line bg-sunk px-3 py-2"
>
  {#if phone}
    <p class="min-w-0 flex-1 text-sm text-muted">Edit layouts on a larger screen</p>
  {:else}
    <p class="min-w-0 flex-1 text-sm text-muted">Drag to move, pull the corner to resize. Tiles snap to the grid.</p>
    <div class="relative">
      <Button variant="default" size="sm" aria-expanded={adding} aria-haspopup="menu" onclick={() => (adding = !adding)}
        ><Plus size={15} /> Add widget</Button
      >
      {#if adding}
        <div class="panel absolute right-0 z-20 mt-1 grid w-60 gap-0.5 p-2" role="menu" aria-label="Add widget">
          {#each kinds as k (k.kind)}
            <button
              type="button"
              role="menuitem"
              class="rounded-md px-2 py-1.5 text-left hover:bg-hover"
              onclick={() => {
                adding = false
                onadd(k.kind)
              }}>{k.title}</button
            >
          {/each}
          {#if views && onaddview}
            <label class="mt-1 grid gap-1 px-2 text-sm text-muted">
              View…
              <select
                class="h-8 rounded-md border border-line bg-surface px-2 text-text"
                aria-label="Add a View widget"
                onchange={(e) => {
                  const id = e.currentTarget.value
                  if (!id) return
                  adding = false
                  onaddview(id)
                }}
              >
                <option value="">Pick a View</option>
                {#each views as v (v.id)}
                  <option value={v.id}>{v.name}</option>
                {/each}
              </select>
            </label>
          {/if}
        </div>
      {/if}
    </div>
    <Button variant="default" size="sm" onclick={onreset}>Reset</Button>
  {/if}
  <Button variant="primary" size="sm" onclick={ondone}>Done</Button>
  {#if hidden.length && !phone}
    <ul class="flex w-full flex-wrap items-center gap-2" aria-label="Hidden widgets">
      <li class="text-xs text-muted">Hidden</li>
      {#each hidden as h (h.id)}
        <li>
          <Button variant="default" size="sm" onclick={() => onshow(h.id)}><Plus size={14} /> {h.title}</Button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
