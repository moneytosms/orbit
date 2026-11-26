<script lang="ts">
  import LayoutGrid from '@lucide/svelte/icons/layout-grid'
  import { Button } from '$lib/components/ui/button'
  import WidgetGallery from '$lib/WidgetGallery.svelte'
  import { colsFor, type Widget } from '$lib/dashboard'

  let {
    kinds,
    widgets,
    views,
    onadd,
    onaddview,
    onhide,
    onreset,
    ondone,
  }: {
    /** Tile kinds the page offers in the gallery. */
    kinds: { kind: string; title: string }[]
    /** The saved layout, hidden tiles included. */
    widgets: Widget[]
    /** Saved Views to add as tiles (Home only). */
    views?: { id: string; name: string }[]
    onadd: (kind: string) => void
    onaddview?: (viewId: string) => void
    onhide: (id: string) => void
    onreset: () => void
    ondone: () => void
  } = $props()

  let width = $state(1280)
  let gallery = $state(false)
  const phone = $derived(colsFor(width) < 4)
</script>

<svelte:window bind:innerWidth={width} />

<div
  role="toolbar"
  aria-label="Edit layout"
  class="mb-3 flex flex-wrap items-center gap-2 rounded-xl border border-line bg-sunk px-3 py-2"
>
  {#if phone}
    <p class="min-w-0 flex-1 text-sm text-muted">Choose what shows on this page. Arrange tiles on a larger screen.</p>
  {:else}
    <p class="min-w-0 flex-1 text-sm text-muted">
      Drag a tile to move it, pull its edges or corners to resize. Everything snaps to the grid.
    </p>
  {/if}
  <Button variant="default" size="sm" aria-haspopup="dialog" onclick={() => (gallery = true)}
    ><LayoutGrid size={15} /> Widgets</Button
  >
  <Button variant="default" size="sm" onclick={onreset}>Reset</Button>
  <Button variant="primary" size="sm" onclick={ondone}>Done</Button>
</div>

<WidgetGallery bind:open={gallery} {kinds} {widgets} {views} {onadd} {onaddview} {onhide} />
