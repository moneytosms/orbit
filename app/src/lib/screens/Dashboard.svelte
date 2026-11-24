<script lang="ts">
  import EyeOff from '@lucide/svelte/icons/eye-off'
  import GripVertical from '@lucide/svelte/icons/grip-vertical'
  import MoveDiagonal2 from '@lucide/svelte/icons/move-diagonal-2'
  import Plus from '@lucide/svelte/icons/plus'
  import { Button } from '$lib/components/ui/button'
  import { COLS, KINDS, ROW_PX, move, phoneOrder, place, resize, titleOf, type Widget } from '$lib/dashboard'
  import Checklist from '$lib/Checklist.svelte'
  import { checklistDismissed, dismissChecklist, showChecklist } from '$lib/onboarding'
  import { getDashboard, setDashboard, tree, views, visibleSnapshot, type View } from '$lib/sync'
  import Glance from '$lib/widgets/Glance.svelte'
  import GoalProgress from '$lib/widgets/GoalProgress.svelte'
  import NextList from '$lib/widgets/NextList.svelte'
  import RoutineTicks from '$lib/widgets/RoutineTicks.svelte'
  import Streaks from '$lib/widgets/Streaks.svelte'
  import Upcoming from '$lib/widgets/Upcoming.svelte'
  import ViewList from '$lib/widgets/ViewList.svelte'

  let { version = 0, onchange }: { version?: number; onchange?: () => void } = $props()

  const GAP = 16
  let layout = $state<Widget[]>([])
  let editing = $state(false)
  let adding = $state(false)
  let viewChoices = $state<View[]>([])
  let summaries = $state<Record<string, string>>({})
  let dragging = false
  let grid: HTMLDivElement | undefined = $state()

  $effect(() => {
    void version
    if (dragging) return
    getDashboard().then((w) => (layout = w as Widget[]))
  })

  const shown = $derived(phoneOrder(layout))
  const hidden = $derived(layout.filter((w) => w.hidden))
  const viewName = (w: Widget) => viewChoices.find((v) => v.id === w.view_id)?.name ?? 'View'
  const title = (w: Widget) => (w.kind === 'view' ? viewName(w) : titleOf(w.kind))
  const href = (w: Widget) =>
    w.kind === 'routines' || w.kind === 'streaks'
      ? '#/routines'
      : w.kind === 'goals'
        ? '#/goals'
        : w.kind === 'upcoming'
          ? '#/upcoming'
          : w.kind === 'view'
            ? `#/view/${w.view_id}`
            : '#/upcoming'
  const setSummary = (id: string, value: string) => (summaries = { ...summaries, [id]: value })

  async function loadViews() {
    viewChoices = await views()
  }
  $effect(() => {
    void version
    loadViews()
  })

  let areaCount = $state(1)
  let taskCount = $state(0)
  let dismissed = $state(checklistDismissed())
  $effect(() => {
    void version
    Promise.all([tree(), visibleSnapshot()]).then(([t, rows]) => {
      areaCount = t.length
      taskCount = rows.filter((r) => r.kind === 'task').length
    })
  })

  const save = () => setDashboard(layout)

  function toggleAdd() {
    adding = !adding
    if (adding) loadViews()
  }
  function addKind(kind: string) {
    const k = KINDS.find((k) => k.kind === kind)!
    const existing = layout.find((w) => w.kind === kind)
    layout = place(layout, existing ?? { id: kind, kind, x: 0, y: 0, w: k.w, h: k.h, hidden: false })
    adding = false
    save()
  }
  function addView(e: Event) {
    const select = e.currentTarget as HTMLSelectElement
    const view_id = select.value
    if (!view_id) return
    layout = place(layout, { id: crypto.randomUUID(), kind: 'view', view_id, x: 0, y: 0, w: 6, h: 3, hidden: false })
    adding = false
    save()
  }
  function hide(w: Widget) {
    layout = layout.map((o) => (o.id === w.id ? { ...o, hidden: true } : o))
    save()
  }
  function show(w: Widget) {
    layout = place(layout, w)
    save()
  }

  // Pointer drags snap to grid cells.
  function cell() {
    const width = grid?.clientWidth ?? 0
    return { cw: (width + GAP) / COLS, ch: ROW_PX + GAP }
  }
  function drag(ev: PointerEvent, w: Widget, mode: 'move' | 'resize') {
    ev.preventDefault()
    const el = ev.currentTarget as HTMLElement
    el.setPointerCapture(ev.pointerId)
    const { cw, ch } = cell()
    const sx = ev.clientX
    const sy = ev.clientY
    const base = { ...w }
    dragging = true
    const onmove = (e: PointerEvent) => {
      const dx = Math.round((e.clientX - sx) / cw)
      const dy = Math.round((e.clientY - sy) / ch)
      layout =
        mode === 'move'
          ? move(layout, base.id, base.x + dx, base.y + dy)
          : resize(layout, base.id, base.w + dx, base.h + dy)
    }
    const end = () => {
      el.removeEventListener('pointermove', onmove)
      el.removeEventListener('pointerup', end)
      el.removeEventListener('pointercancel', end)
      dragging = false
      save()
    }
    el.addEventListener('pointermove', onmove)
    el.addEventListener('pointerup', end)
    el.addEventListener('pointercancel', end)
  }
  function key(ev: KeyboardEvent, w: Widget) {
    if (!editing || ev.target !== ev.currentTarget) return
    const d: Record<string, [number, number]> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    }
    const step = d[ev.key]
    if (!step) return
    ev.preventDefault()
    layout = ev.shiftKey
      ? resize(layout, w.id, w.w + step[0], w.h + step[1])
      : move(layout, w.id, w.x + step[0], w.y + step[1])
    save()
  }

  const iconBtn =
    'grid size-7 shrink-0 place-items-center rounded-md text-icon hover:bg-hover hover:text-text focus-visible:outline-2'
</script>

<div class="mb-4 flex flex-wrap items-center gap-2">
  <h1 class="page-title mb-0 flex-1">Home</h1>
  {#if editing}
    <div class="relative">
      <Button variant="default" size="sm" onclick={toggleAdd} aria-expanded={adding}
        ><Plus size={15} /> Add widget</Button
      >
      {#if adding}
        <div class="panel absolute right-0 z-20 mt-1 grid w-60 gap-0.5 p-2" role="menu" aria-label="Add widget">
          {#each KINDS as k (k.kind)}
            <button
              type="button"
              role="menuitem"
              class="rounded-md px-2 py-1.5 text-left hover:bg-hover"
              onclick={() => addKind(k.kind)}>{k.title}</button
            >
          {/each}
          <label class="mt-1 grid gap-1 px-2 text-sm text-muted">
            View…
            <select
              class="h-8 rounded-md border border-line bg-surface px-2 text-text"
              aria-label="Add a View widget"
              onchange={addView}
            >
              <option value="">Pick a View</option>
              {#each viewChoices as v (v.id)}
                <option value={v.id}>{v.name}</option>
              {/each}
            </select>
          </label>
        </div>
      {/if}
    </div>
  {/if}
  <Button
    variant={editing ? 'on' : 'default'}
    size="sm"
    aria-pressed={editing}
    onclick={() => {
      editing = !editing
      adding = false
    }}>{editing ? 'Done' : 'Edit layout'}</Button
  >
</div>

{#if showChecklist(areaCount, dismissed)}
  <Checklist
    hasArea={areaCount > 0}
    hasTask={taskCount > 0}
    ondismiss={() => {
      dismissChecklist()
      dismissed = true
    }}
  />
{/if}

<div bind:this={grid} class="grid gap-4 desk:grid-cols-12 desk:[grid-auto-rows:var(--row)]" style="--row: {ROW_PX}px">
  {#each shown as w (w.id)}
    <!-- In edit mode a widget takes focus so arrow keys move it and Shift+arrows resize it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <section
      aria-label={title(w)}
      tabindex={editing ? 0 : undefined}
      onkeydown={(e) => key(e, w)}
      class="panel relative flex min-h-24 min-w-0 flex-col overflow-hidden desk:min-h-0 desk:[grid-column:var(--c)] desk:[grid-row:var(--r)] {editing
        ? 'outline-dashed outline-1 outline-line-strong focus-visible:outline-2 focus-visible:outline-accent'
        : ''}"
      style="--c: {w.x + 1} / span {w.w}; --r: {w.y + 1} / span {w.h}"
    >
      <header class="mb-2 flex min-w-0 items-start gap-2">
        {#if editing}
          <button
            type="button"
            aria-label="Move {title(w)}"
            class="{iconBtn} -ml-1 cursor-grab touch-none desk:grid hidden"
            onpointerdown={(e) => drag(e, w, 'move')}><GripVertical size={15} /></button
          >
        {/if}
        <div class="min-w-0 flex-1">
          <h2 class="truncate text-[0.9062rem] font-semibold">{title(w)}</h2>
          {#if w.kind !== 'glance'}<p class="truncate text-xs text-muted">{summaries[w.id] ?? 'Loading…'}</p>{/if}
        </div>
        {#if editing}
          <button type="button" aria-label="Hide {title(w)}" class={iconBtn} onclick={() => hide(w)}
            ><EyeOff size={15} /></button
          >
        {/if}
      </header>
      <div class="min-h-0 flex-1 overflow-auto">
        {#if w.kind === 'glance'}
          <Glance {version} />
        {:else if w.kind === 'next'}
          <NextList {version} {onchange} onmeta={(value) => setSummary(w.id, value)} />
        {:else if w.kind === 'routines'}
          <RoutineTicks {version} {onchange} onmeta={(value) => setSummary(w.id, value)} />
        {:else if w.kind === 'upcoming'}
          <Upcoming {version} onmeta={(value) => setSummary(w.id, value)} />
        {:else if w.kind === 'goals'}
          <GoalProgress {version} onmeta={(value) => setSummary(w.id, value)} />
        {:else if w.kind === 'streaks'}
          <Streaks {version} onmeta={(value) => setSummary(w.id, value)} />
        {:else if w.kind === 'view' && w.view_id}
          <ViewList viewId={w.view_id} {version} onmeta={(value) => setSummary(w.id, value)} />
        {/if}
      </div>
      {#if w.kind !== 'glance'}
        <a href={href(w)} class="mt-2 self-start text-xs text-muted hover:text-text">Open →</a>
      {/if}
      {#if editing}
        <button
          type="button"
          aria-label="Resize {title(w)}"
          class="absolute right-1 bottom-1 hidden size-6 cursor-nwse-resize touch-none place-items-center rounded-md text-icon hover:bg-hover desk:grid"
          onpointerdown={(e) => drag(e, w, 'resize')}><MoveDiagonal2 size={14} /></button
        >
      {/if}
    </section>
  {/each}
</div>

{#if editing}
  <p class="mt-3 text-xs text-muted">Focus a widget and use the arrow keys to move it, Shift+arrows to resize.</p>
  {#if hidden.length}
    <section class="mt-4" aria-label="Hidden widgets">
      <h2 class="mb-2 text-sm font-semibold">Hidden</h2>
      <ul class="flex flex-wrap gap-2">
        {#each hidden as w (w.id)}
          <li>
            <Button variant="default" size="sm" onclick={() => show(w)}><Plus size={14} /> {title(w)}</Button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
{/if}
