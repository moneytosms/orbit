<script lang="ts">
  import EyeOff from '@lucide/svelte/icons/eye-off'
  import GripVertical from '@lucide/svelte/icons/grip-vertical'
  import MoveDiagonal2 from '@lucide/svelte/icons/move-diagonal-2'
  import type { Snippet } from 'svelte'
  import { COLS, ROW_PX, colsFor, expand, fits, move, reflow, resize, type Rect, type Widget } from '$lib/dashboard'

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

  type Drag = { id: string; mode: 'move' | 'resize'; rect: Rect; ok: boolean; dx: number; dy: number }
  let drag = $state<Drag | null>(null)

  // Edits happen in the visible column count and map back onto the one saved 12-column layout.
  function apply(fn: (list: Widget[]) => Widget[]) {
    const base = cols === COLS ? widgets : view
    const next = fn(base)
    onchange(cols === COLS ? next : expand(next, cols, widgets))
  }

  function start(ev: PointerEvent, w: Widget, mode: 'move' | 'resize') {
    ev.preventDefault()
    const el = ev.currentTarget as HTMLElement
    el.setPointerCapture(ev.pointerId)
    const cw = ((grid?.clientWidth ?? 0) + GAP) / cols
    const ch = ROW_PX + GAP
    const sx = ev.clientX
    const sy = ev.clientY
    const base = view.find((o) => o.id === w.id) ?? w
    const set = (e: PointerEvent) => {
      const gx = Math.round((e.clientX - sx) / cw)
      const gy = Math.round((e.clientY - sy) / ch)
      const rect: Rect =
        mode === 'move'
          ? { x: base.x + gx, y: base.y + gy, w: base.w, h: base.h }
          : { x: base.x, y: base.y, w: base.w + gx, h: base.h + gy }
      drag = {
        id: base.id,
        mode,
        rect,
        ok: fits(view, base.id, rect, cols),
        dx: mode === 'move' ? e.clientX - sx : 0,
        dy: mode === 'move' ? e.clientY - sy : 0,
      }
    }
    set(ev)
    const end = (e: PointerEvent) => {
      el.removeEventListener('pointermove', set)
      el.removeEventListener('pointerup', end)
      el.removeEventListener('pointercancel', end)
      const d = drag
      drag = null
      const same = d && d.rect.x === base.x && d.rect.y === base.y && d.rect.w === base.w && d.rect.h === base.h
      // A drop on an overlap or off the grid leaves the tile where it was.
      if (e.type === 'pointerup' && d?.ok && !same)
        apply((list) => list.map((o) => (o.id === base.id ? { ...o, ...d.rect } : o)))
    }
    el.addEventListener('pointermove', set)
    el.addEventListener('pointerup', end)
    el.addEventListener('pointercancel', end)
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
  /** The ghost stays on the grid's lines even when the candidate spills over (it is drawn red then). */
  const ghost = (r: Rect) => {
    const x = Math.max(0, Math.min(r.x, cols - 1))
    const y = Math.max(0, r.y)
    return place({ x, y, w: Math.max(1, Math.min(r.w, cols - x)), h: Math.max(1, r.h) })
  }
  const iconBtn =
    'grid size-7 shrink-0 place-items-center rounded-md text-icon hover:bg-hover hover:text-text focus-visible:outline-2 focus-visible:outline-accent'
</script>

<svelte:window bind:innerWidth={width} />

<div
  bind:this={grid}
  role="group"
  aria-label="Page layout"
  class="relative grid gap-4"
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

  {#each view as w (w.id)}
    {@const moving = drag?.id === w.id && drag.mode === 'move'}
    <!-- While editing a tile takes focus so arrow keys move it and Shift+arrows resize it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <section
      aria-label={label(w)}
      data-tile={w.id}
      tabindex={canEdit ? 0 : undefined}
      onkeydown={(e) => key(e, w)}
      class="panel relative z-[1] flex min-w-0 flex-col overflow-hidden {stacked ? 'min-h-24' : 'min-h-0'} {canEdit
        ? 'outline-dashed outline-2 -outline-offset-2 outline-accent focus-visible:outline-solid'
        : ''} {moving ? 'z-10 opacity-80 shadow-2' : ''}"
      style="{stacked ? 'grid-column: 1 / -1' : place(w)}{moving
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
      <div class="flex min-h-0 flex-1 flex-col">{@render renderTile(w)}</div>
      {#if canEdit}
        <button
          type="button"
          aria-label="Resize {label(w)}"
          class="absolute right-1 bottom-1 grid size-6 cursor-nwse-resize touch-none place-items-center rounded-md bg-surface text-accent hover:bg-hover"
          onpointerdown={(e) => start(e, w, 'resize')}><MoveDiagonal2 size={14} /></button
        >
      {/if}
    </section>
  {/each}

  {#if canEdit && drag}
    <!-- Where the tile will land; red when that spot overlaps another tile or leaves the grid. -->
    <div
      aria-hidden="true"
      data-testid="layout-ghost"
      data-valid={drag.ok}
      class="pointer-events-none z-[5] rounded-2xl border-2 {drag.ok
        ? 'border-accent bg-accent-soft opacity-70'
        : 'border-danger bg-danger/10'}"
      style={ghost(drag.rect)}
    ></div>
  {/if}
</div>
