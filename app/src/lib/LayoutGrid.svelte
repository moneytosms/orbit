<script lang="ts">
  import EyeOff from '@lucide/svelte/icons/eye-off'
  import GripVertical from '@lucide/svelte/icons/grip-vertical'
  import type { Snippet } from 'svelte'
  import {
    COLS,
    ROW_PX,
    colsFor,
    compact,
    dragRect,
    expand,
    minHeight,
    move,
    reflow,
    resize,
    setRect,
    type Rect,
    type Widget,
  } from '$lib/dashboard'

  let {
    widgets,
    editing = false,
    renderTile,
    onchange,
    label = (w: Widget) => w.kind,
    hideTile,
  }: {
    /** The saved layout, hidden tiles included. */
    widgets: Widget[]
    editing?: boolean
    /** The tile's content; the grid draws the frame and, while editing, the move, resize and hide controls. */
    renderTile: Snippet<[Widget]>
    /** The whole new saved layout after a drag, resize, keyboard move or hide. */
    onchange: (next: Widget[]) => void
    /** Accessible name of a tile. */
    label?: (w: Widget) => string
    /** Tiles to leave out of the page while not editing (an empty Calendar, say). */
    hideTile?: (w: Widget) => boolean
  } = $props()

  const GAP = 16
  let width = $state(1280)
  let grid: HTMLDivElement | undefined = $state()
  const cols = $derived(colsFor(width))
  const stacked = $derived(cols < 4)
  // Editing is for tablet and desktop; a phone shows the layout only.
  const canEdit = $derived(editing && !stacked)
  const view = $derived(reflow(widgets, cols))
  const rows = $derived(Math.max(4, ...view.map((w) => w.y + w.h)) + 2)

  /** `move` drags the whole tile; the rest resize from that edge or corner. */
  type Handle = 'move' | 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'
  type Drag = { id: string; handle: Handle; base: Rect; rect: Rect; dx: number; dy: number }
  let drag = $state<Drag | null>(null)
  // While dragging, the other tiles already make room, so the drop is what you see.
  const shown = $derived(drag ? setRect(view, drag.id, drag.rect, cols) : view)

  // Edits happen in the visible column count and map back onto the one saved 12-column layout.
  function apply(fn: (list: Widget[]) => Widget[]) {
    const base = cols === COLS ? widgets : view
    const next = fn(base)
    onchange(cols === COLS ? compact(next) : expand(next, cols, widgets))
  }

  function start(ev: PointerEvent, w: Widget, handle: Handle) {
    if (ev.button > 0) return
    ev.preventDefault()
    ev.stopPropagation()
    const el = ev.currentTarget as HTMLElement
    el.setPointerCapture(ev.pointerId)
    const cw = ((grid?.clientWidth ?? 0) + GAP) / cols
    const ch = ROW_PX + GAP
    const sx = ev.clientX
    const sy = ev.clientY
    const t = view.find((o) => o.id === w.id) ?? w
    const base: Rect = { x: t.x, y: t.y, w: t.w, h: t.h }
    const set = (e: PointerEvent) => {
      const gx = Math.round((e.clientX - sx) / cw)
      const gy = Math.round((e.clientY - sy) / ch)
      const rect: Rect =
        handle === 'move'
          ? { ...base, x: base.x + gx, y: base.y + gy }
          : dragRect(base, handle, gx, gy, cols, minHeight(w.kind))
      drag = {
        id: w.id,
        handle,
        base,
        rect,
        dx: handle === 'move' ? e.clientX - sx : 0,
        dy: handle === 'move' ? e.clientY - sy : 0,
      }
    }
    set(ev)
    const end = (e: PointerEvent) => {
      el.removeEventListener('pointermove', set)
      el.removeEventListener('pointerup', end)
      el.removeEventListener('pointercancel', end)
      const d = drag
      drag = null
      // A cancelled gesture, or a drop where it started, changes nothing.
      if (e.type === 'pointerup' && d) {
        const r = d.rect
        const same = r.x === base.x && r.y === base.y && r.w === base.w && r.h === base.h
        if (!same) apply((list) => setRect(list, w.id, r, cols))
      }
    }
    el.addEventListener('pointermove', set)
    el.addEventListener('pointerup', end)
    el.addEventListener('pointercancel', end)
  }

  // Drag the tile from anywhere on its face, but let its own buttons and handles do their jobs.
  function faceDown(ev: PointerEvent, w: Widget) {
    if (!canEdit || (ev.target as HTMLElement).closest('button, [data-handle]')) return
    start(ev, w, 'move')
  }

  function key(ev: KeyboardEvent, w: Widget) {
    if (!canEdit || ev.target !== ev.currentTarget) return
    const d: Record<string, [number, number]> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    }
    const step = d[ev.key]
    if (!step) return
    ev.preventDefault()
    apply((list) => {
      const t = list.find((o) => o.id === w.id) ?? w
      return ev.shiftKey
        ? resize(list, w.id, t.w + step[0], t.h + step[1], cols)
        : move(list, w.id, t.x + step[0], t.y + step[1], cols)
    })
  }

  const hide = (w: Widget) => onchange(widgets.map((o) => (o.id === w.id ? { ...o, hidden: true } : o)))

  const place = (r: Rect) => `grid-column: ${r.x + 1} / span ${r.w}; grid-row: ${r.y + 1} / span ${r.h}`
  const iconBtn =
    'grid size-7 shrink-0 place-items-center rounded-md text-icon hover:bg-hover hover:text-text focus-visible:outline-2 focus-visible:outline-accent'

  // Eight handles on the tile's border, like an image: a small dot on each corner and a thin bar on each edge.
  // Each sits centred on the border, so only a sliver lands inside the tile's padding and no content is covered.
  // The button around it is a larger transparent hit area that reaches across the border, so it stays easy to grab.
  const HANDLES: { h: Handle; spot: string; hit: string; cursor: string; bar: string }[] = [
    {
      h: 'nw',
      spot: 'top-0 left-0 -translate-x-1/2 -translate-y-1/2',
      hit: 'size-6',
      cursor: 'cursor-nwse-resize',
      bar: 'size-2.5 rounded-full',
    },
    {
      h: 'n',
      spot: 'top-0 left-1/2 -translate-x-1/2 -translate-y-1/2',
      hit: 'h-5 w-14',
      cursor: 'cursor-ns-resize',
      bar: 'h-1 w-7 rounded-full',
    },
    {
      h: 'ne',
      spot: 'top-0 right-0 translate-x-1/2 -translate-y-1/2',
      hit: 'size-6',
      cursor: 'cursor-nesw-resize',
      bar: 'size-2.5 rounded-full',
    },
    {
      h: 'e',
      spot: 'top-1/2 right-0 translate-x-1/2 -translate-y-1/2',
      hit: 'h-14 w-5',
      cursor: 'cursor-ew-resize',
      bar: 'h-7 w-1 rounded-full',
    },
    {
      h: 'se',
      spot: 'right-0 bottom-0 translate-x-1/2 translate-y-1/2',
      hit: 'size-6',
      cursor: 'cursor-nwse-resize',
      bar: 'size-2.5 rounded-full',
    },
    {
      h: 's',
      spot: 'bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2',
      hit: 'h-5 w-14',
      cursor: 'cursor-ns-resize',
      bar: 'h-1 w-7 rounded-full',
    },
    {
      h: 'sw',
      spot: 'bottom-0 left-0 -translate-x-1/2 translate-y-1/2',
      hit: 'size-6',
      cursor: 'cursor-nesw-resize',
      bar: 'size-2.5 rounded-full',
    },
    {
      h: 'w',
      spot: 'top-1/2 left-0 -translate-x-1/2 -translate-y-1/2',
      hit: 'h-14 w-5',
      cursor: 'cursor-ew-resize',
      bar: 'h-7 w-1 rounded-full',
    },
  ]
</script>

<svelte:window bind:innerWidth={width} />

<div
  bind:this={grid}
  role="group"
  aria-label="Page layout"
  class="relative grid gap-4 {drag ? 'select-none' : ''}"
  style="grid-template-columns: repeat({cols}, minmax(0, 1fr)); {stacked
    ? ''
    : `grid-auto-rows: ${ROW_PX}px; ${canEdit ? `min-height: ${rows * ROW_PX + (rows - 1) * GAP}px` : ''}`}"
>
  {#if canEdit}
    <!-- Faint cell guides, only while editing. -->
    <div
      aria-hidden="true"
      data-testid="layout-guides"
      class="pointer-events-none absolute inset-0 grid gap-4"
      style="grid-template-columns: repeat({cols}, minmax(0, 1fr)); grid-auto-rows: {ROW_PX}px"
    >
      {#each { length: cols * rows } as _, i (i)}
        <div class="rounded-lg border border-dashed border-line opacity-60"></div>
      {/each}
    </div>
  {/if}

  {#each shown as w (w.id)}
    {@const mine = drag?.id === w.id}
    {@const moving = mine && drag?.handle === 'move'}
    <!-- While editing a tile takes focus so arrow keys move it and Shift+arrows resize it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <section
      aria-label={label(w)}
      data-tile={w.id}
      tabindex={canEdit ? 0 : undefined}
      onkeydown={(e) => key(e, w)}
      onpointerdown={(e) => faceDown(e, w)}
      class="panel relative z-[1] flex min-w-0 flex-col {canEdit ? '' : 'overflow-hidden'} {stacked
        ? 'min-h-24'
        : 'min-h-0'} {canEdit
        ? `cursor-grab touch-none outline-dashed outline-2 -outline-offset-2 outline-accent focus-visible:outline-solid ${mine ? 'z-10 shadow-2' : ''}`
        : ''} {moving ? '!cursor-grabbing opacity-90' : ''}"
      style="{stacked ? 'grid-column: 1 / -1' : place(moving && drag ? drag.base : w)}{moving
        ? `; transform: translate(${drag?.dx}px, ${drag?.dy}px)`
        : ''}{hideTile?.(w) && !canEdit ? '; display: none' : ''}"
    >
      {#if canEdit}
        <div class="mb-1 flex items-center justify-between">
          <button
            type="button"
            aria-label="Move {label(w)}"
            class="{iconBtn} -ml-1 cursor-grab touch-none"
            onpointerdown={(e) => start(e, w, 'move')}><GripVertical size={15} /></button
          >
          <button type="button" aria-label="Hide {label(w)}" class={iconBtn} onclick={() => hide(w)}
            ><EyeOff size={15} /></button
          >
        </div>
      {/if}
      <!-- Content is inert while editing so a press anywhere on the face drags the tile. -->
      <div class="flex min-h-0 flex-1 flex-col overflow-hidden {canEdit ? 'pointer-events-none' : ''}">
        {@render renderTile(w)}
      </div>
      {#if canEdit}
        {#each HANDLES as h (h.h)}
          <!-- Only the south-east handle is a tab stop; the keyboard resizes with Shift+arrows. -->
          <button
            type="button"
            data-handle={h.h}
            tabindex={h.h === 'se' ? 0 : -1}
            aria-label={h.h === 'se' ? `Resize ${label(w)}` : undefined}
            aria-hidden={h.h === 'se' ? undefined : 'true'}
            class="absolute z-20 grid touch-none place-items-center {h.hit} {h.spot} {h.cursor}"
            onpointerdown={(e) => start(e, w, h.h)}
          >
            <span class="block border-2 border-accent bg-surface shadow-1 {h.bar}"></span>
          </button>
        {/each}
        {#if mine && drag && drag.handle !== 'move'}
          <span
            class="pointer-events-none absolute top-1/2 left-1/2 z-30 -translate-x-1/2 -translate-y-1/2 rounded-full bg-text px-2.5 py-1 text-xs font-medium text-bg shadow-2"
            data-testid="layout-size">{drag.rect.w} × {drag.rect.h}</span
          >
        {/if}
      {/if}
    </section>
  {/each}

  {#if canEdit && drag}
    <!-- Where the tile will land; the other tiles have already moved aside to make room. -->
    <div
      aria-hidden="true"
      data-testid="layout-ghost"
      class="pointer-events-none z-[5] rounded-2xl border-2 border-accent bg-accent-soft opacity-70"
      style={place(shown.find((o) => o.id === drag?.id) ?? drag.rect)}
    ></div>
  {/if}
</div>
