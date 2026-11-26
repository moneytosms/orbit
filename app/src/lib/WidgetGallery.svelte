<script lang="ts">
  // The widget gallery: every tile the page can show, as a card with an icon, a line of description and a switch,
  // a bit like a phone's control centre. Switching a card on adds or shows the tile; off hides it.
  import LayoutGrid from '@lucide/svelte/icons/layout-grid'
  import * as Sheet from '$lib/components/ui/sheet'
  import { Switch } from '$lib/components/ui/switch'
  import { colsFor, type Widget } from '$lib/dashboard'
  import { metaOf } from '$lib/widgetMeta'

  let {
    open = $bindable(false),
    kinds,
    widgets,
    views = [],
    onadd,
    onaddview,
    onhide,
  }: {
    open?: boolean
    /** Tile kinds this page offers. */
    kinds: { kind: string; title: string }[]
    /** The saved layout, hidden tiles included. */
    widgets: Widget[]
    /** Saved Views that can be added as tiles (Home only). */
    views?: { id: string; name: string }[]
    onadd: (kind: string) => void
    onaddview?: (viewId: string) => void
    onhide: (id: string) => void
  } = $props()

  let width = $state(1280)
  const wide = $derived(colsFor(width) >= 6)
  const tileOf = (kind: string) => widgets.find((w) => w.kind === kind && !w.view_id)
  const viewTile = (id: string) => widgets.find((w) => w.kind === 'view' && w.view_id === id)
  const isOn = (w: Widget | undefined) => !!w && !w.hidden

  const card =
    'flex w-full cursor-pointer items-start gap-3 rounded-xl border p-3 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-line'
</script>

<svelte:window bind:innerWidth={width} />

<Sheet.Root bind:open>
  <Sheet.Content
    side={wide ? 'right' : 'bottom'}
    class="w-full max-w-md gap-0 overflow-y-auto p-0 {wide ? '' : 'max-h-[85vh] max-w-none rounded-t-2xl'}"
  >
    <div class="border-b border-line px-5 py-4">
      <Sheet.Title class="text-base font-semibold">Widgets</Sheet.Title>
      <p class="mt-0.5 text-sm text-muted">Switch on what you want on this page. Drag to arrange them.</p>
    </div>
    <ul class="grid grid-cols-1 gap-2 p-4 sm:grid-cols-2" aria-label="Widgets">
      {#each kinds as k (k.kind)}
        {@const w = tileOf(k.kind)}
        {@const on = isOn(w)}
        {@const m = metaOf(k.kind)}
        <li>
          <div
            role="button"
            tabindex="0"
            class="{card} {on ? 'border-accent-line bg-accent-soft' : 'border-line bg-surface hover:bg-hover'}"
            onclick={() => (on && w ? onhide(w.id) : onadd(k.kind))}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault()
                if (on && w) onhide(w.id)
                else onadd(k.kind)
              }
            }}
          >
            <span
              aria-hidden="true"
              class="grid size-9 shrink-0 place-items-center rounded-lg {on
                ? 'bg-accent text-accent-ink'
                : 'bg-sunk text-icon'}"
            >
              <m.icon size={18} />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-sm font-medium">{k.title}</span>
              <span class="block text-xs text-muted">{m.desc}</span>
            </span>
            <Switch checked={on} tabindex={-1} aria-label="{k.title} widget" class="pointer-events-none mt-0.5" />
          </div>
        </li>
      {/each}
    </ul>

    {#if views.length && onaddview}
      <h3 class="px-5 pt-1 pb-2 text-[0.7812rem] font-medium tracking-wide text-muted uppercase">Saved views</h3>
      <ul class="grid grid-cols-1 gap-2 px-4 pb-5 sm:grid-cols-2" aria-label="Saved views">
        {#each views as v (v.id)}
          {@const w = viewTile(v.id)}
          {@const on = isOn(w)}
          <li>
            <div
              role="button"
              tabindex="0"
              class="{card} items-center {on
                ? 'border-accent-line bg-accent-soft'
                : 'border-line bg-surface hover:bg-hover'}"
              onclick={() => (on && w ? onhide(w.id) : onaddview(v.id))}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  if (on && w) onhide(w.id)
                  else onaddview(v.id)
                }
              }}
            >
              <span
                class="grid size-9 shrink-0 place-items-center rounded-lg {on
                  ? 'bg-accent text-accent-ink'
                  : 'bg-sunk text-icon'}"
              >
                <LayoutGrid size={18} aria-hidden="true" />
              </span>
              <span class="min-w-0 flex-1 truncate text-sm font-medium">{v.name}</span>
              <Switch checked={on} tabindex={-1} aria-label="{v.name} view" class="pointer-events-none" />
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </Sheet.Content>
</Sheet.Root>
