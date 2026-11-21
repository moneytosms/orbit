<script lang="ts">
  import Circle from '@lucide/svelte/icons/circle'
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import CircleDot from '@lucide/svelte/icons/circle-dot'
  import Diamond from '@lucide/svelte/icons/diamond'
  import Folder from '@lucide/svelte/icons/folder'
  import Lock from '@lucide/svelte/icons/lock'
  import LockOpen from '@lucide/svelte/icons/lock-open'
  import Maximize from '@lucide/svelte/icons/maximize'
  import Minus from '@lucide/svelte/icons/minus'
  import Plus from '@lucide/svelte/icons/plus'
  import Target from '@lucide/svelte/icons/target'
  import X from '@lucide/svelte/icons/x'
  import { Button } from '$lib/components/ui/button'
  import { fit, frameAt, gatePath, layout, linkPath, zoomAt, clampZoom, type StepBox } from '$lib/canvas'
  import {
    gateChoices,
    linkTarget,
    setLinks,
    setPhaseDone,
    setPhaseGate,
    updatePhase,
    type PhaseSummary,
  } from '$lib/sync'

  let {
    id,
    phases,
    unplaced,
    onplace,
    onchange,
  }: {
    id: string
    phases: PhaseSummary[]
    unplaced: { id: string; kind: string; title: string }[]
    onplace: (itemId: string, phaseId: string | null) => void
    onchange: () => void
  } = $props()

  const lay = $derived(layout(phases))
  const boxes = $derived(new Map(lay.steps.map((s) => [s.id, s])))
  const frames = $derived(new Map(lay.frames.map((f) => [f.id, f])))

  let view = $state({ x: 40, y: 76, z: 0.9 })
  let el = $state<HTMLDivElement>()
  let sel = $state<{ kind: 'phase' | 'step'; id: string } | null>(null)
  let gates = $state<{ id: string; title: string; done: boolean }[]>([])
  let panning = $state(false)
  let drag = $state<{
    id: string
    from: string | null
    sx: number
    sy: number
    dx: number
    dy: number
    live: boolean
  } | null>(null)
  let fitted = false

  $effect(() => {
    void phases
    gateChoices().then((g) => (gates = g))
  })
  $effect(() => {
    if (!el || fitted || !phases.length) return
    fitted = true
    view = fit(lay.frames, el.clientWidth, el.clientHeight)
  })
  $effect(() => {
    if (sel?.kind === 'phase' && !frames.has(sel.id)) sel = null
    if (sel?.kind === 'step' && !boxes.has(sel.id)) sel = null
  })

  const selPhase = $derived(sel?.kind === 'phase' ? phases.find((p) => p.id === sel!.id) : undefined)
  const selStep = $derived(sel?.kind === 'step' ? boxes.get(sel.id) : undefined)
  const overFrame = $derived.by(() => {
    if (!drag?.live || !el) return null
    const r = el.getBoundingClientRect()
    return frameAt(
      lay.frames,
      (drag.sx + drag.dx - r.left - view.x) / view.z,
      (drag.sy + drag.dy - r.top - view.y) / view.z,
    )
  })
  const pct = (p: number) => `${Math.round(p * 100)}%`
  const span = (p: PhaseSummary) => [p.start, p.end].filter(Boolean).join(' – ')
  const world = (e: { clientX: number; clientY: number }) => {
    const r = el!.getBoundingClientRect()
    return { x: (e.clientX - r.left - view.x) / view.z, y: (e.clientY - r.top - view.y) / view.z }
  }

  function down(e: PointerEvent) {
    if (e.button !== 0 || (e.target as HTMLElement).closest('[data-node],[data-ui]')) return
    sel = null
    panning = true
    let last = { x: e.clientX, y: e.clientY }
    const move = (m: PointerEvent) => {
      view = { ...view, x: view.x + m.clientX - last.x, y: view.y + m.clientY - last.y }
      last = { x: m.clientX, y: m.clientY }
    }
    const up = () => {
      panning = false
      removeEventListener('pointermove', move)
      removeEventListener('pointerup', up)
    }
    addEventListener('pointermove', move)
    addEventListener('pointerup', up)
  }
  function wheel(e: WheelEvent) {
    e.preventDefault()
    if (e.ctrlKey || e.metaKey) {
      const r = el!.getBoundingClientRect()
      view = zoomAt(view, Math.exp(-e.deltaY * 0.01), e.clientX - r.left, e.clientY - r.top)
    } else view = { ...view, x: view.x - e.deltaX, y: view.y - e.deltaY }
  }
  function zoom(f: number) {
    if (el) view = zoomAt(view, f, el.clientWidth / 2, el.clientHeight / 2)
    else view = { ...view, z: clampZoom(view.z * f) }
  }
  function key(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest('[data-ui],input,select,textarea')) return
    const step = 60
    if (e.key === '+' || e.key === '=') zoom(1.2)
    else if (e.key === '-') zoom(1 / 1.2)
    else if (e.key === '0' && el) view = fit(lay.frames, el.clientWidth, el.clientHeight)
    else if (e.key === 'ArrowLeft') view = { ...view, x: view.x + step }
    else if (e.key === 'ArrowRight') view = { ...view, x: view.x - step }
    else if (e.key === 'ArrowUp') view = { ...view, y: view.y + step }
    else if (e.key === 'ArrowDown') view = { ...view, y: view.y - step }
    else if (e.key === 'Escape') sel = null
    else return
    e.preventDefault()
  }

  function grab(e: PointerEvent, itemId: string, from: string | null) {
    if (e.button !== 0) return
    const node = e.currentTarget as HTMLElement
    node.setPointerCapture(e.pointerId)
    drag = { id: itemId, from, sx: e.clientX, sy: e.clientY, dx: 0, dy: 0, live: false }
  }
  function carry(e: PointerEvent) {
    if (!drag) return
    const dx = e.clientX - drag.sx
    const dy = e.clientY - drag.sy
    drag = { ...drag, dx, dy, live: drag.live || Math.hypot(dx, dy) > 5 }
  }
  function drop(e: PointerEvent) {
    const d = drag
    drag = null
    if (!d) return
    if (!d.live) {
      if (d.from) sel = { kind: 'step', id: d.id }
      return
    }
    const w = world(e)
    const target = frameAt(lay.frames, w.x, w.y)
    if (target && target.id !== d.from) onplace(d.id, target.id)
  }

  async function rename(e: FocusEvent, p: PhaseSummary) {
    const input = e.currentTarget as HTMLInputElement
    const value = input.value.trim()
    if (!value) input.value = p.name
    else if (value !== p.name) {
      await updatePhase(p.id, { name: value })
      onchange()
    }
  }
  async function setDate(p: PhaseSummary, field: 'start' | 'end', value: string) {
    await updatePhase(p.id, { [field]: value || null })
    onchange()
  }
  async function gate(p: PhaseSummary, value: string) {
    await setPhaseGate(p.id, value || null)
    onchange()
  }
  async function complete(p: PhaseSummary) {
    await setPhaseDone(p.id, !p.completed)
    onchange()
  }
  async function links(s: StepBox, next: string[]) {
    await setLinks(id, s.id, s.phaseId, next)
    onchange()
  }
  async function open(itemId: string) {
    const t = await linkTarget(itemId)
    if (t?.href) location.hash = t.href
  }
  const titleOf = (itemId: string) => boxes.get(itemId)?.item.title ?? ''
  const tone = (p: PhaseSummary) => (p.completed ? 'done' : p.locked ? 'locked' : p.current ? 'active' : 'todo')
</script>

{#snippet icon(kind: string, progress: number)}
  {#if kind === 'milestone'}<Diamond class="size-[0.9375rem] shrink-0 text-warn" />
  {:else if kind === 'goal'}<Target class="size-[0.9375rem] shrink-0 text-accent" />
  {:else if kind === 'project'}<Folder class="size-[0.9375rem] shrink-0 text-icon" />
  {:else if progress >= 1}<CircleCheck class="size-[0.9375rem] shrink-0 text-ok" />
  {:else if progress > 0}<CircleDot class="size-[0.9375rem] shrink-0 text-accent" />
  {:else}<Circle class="size-[0.9375rem] shrink-0 text-icon" />{/if}
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  bind:this={el}
  role="application"
  tabindex="0"
  aria-label="Roadmap canvas. Drag steps between phases, drag the background to pan, Ctrl and scroll to zoom."
  onpointerdown={down}
  onwheel={wheel}
  onkeydown={key}
  class="relative h-[calc(100vh/var(--zoom,1)-190px)] min-h-[28.75rem] touch-none overflow-hidden rounded-2xl border border-line bg-bg outline-none focus-visible:ring-2 focus-visible:ring-accent {panning
    ? 'cursor-grabbing'
    : 'cursor-grab'}"
  style="background-image: radial-gradient(color-mix(in srgb, var(--border-strong) 75%, transparent) 1px, transparent 1px); background-size: {22 *
    view.z}px {22 * view.z}px; background-position: {view.x}px {view.y}px"
>
  <div
    class="absolute top-0 left-0 origin-top-left"
    style="transform: translate({view.x}px, {view.y}px) scale({view.z})"
  >
    <svg class="pointer-events-none absolute top-0 left-0 h-px w-px overflow-visible" aria-hidden="true">
      <defs>
        <marker id="rc-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto">
          <path d="M0 1L9 5L0 9z" fill="var(--icon)" />
        </marker>
        <marker id="rc-gate" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto">
          <path d="M0 1L9 5L0 9z" fill="var(--accent)" />
        </marker>
      </defs>
      {#each lay.links as l (l.from + l.to)}
        {@const a = boxes.get(l.from)}
        {@const b = boxes.get(l.to)}
        {#if a && b}
          <path
            d={linkPath(a, b)}
            fill="none"
            stroke="var(--icon)"
            stroke-width="1.5"
            opacity=".75"
            stroke-dasharray={l.locked ? '5 4' : undefined}
            marker-end="url(#rc-arrow)"
            data-link="{l.from}>{l.to}"
          />
        {/if}
      {/each}
      {#each lay.gates as g (g.from + g.to)}
        {@const m = boxes.get(g.from)}
        {@const f = frames.get(g.to)}
        {#if m && f}
          <path
            d={gatePath(m, f)}
            fill="none"
            stroke="var(--accent)"
            stroke-width="1.5"
            stroke-dasharray="6 5"
            opacity=".85"
            marker-end="url(#rc-gate)"
            data-gate={g.to}
          />
        {/if}
      {/each}
    </svg>

    {#each phases as p (p.id)}
      {@const f = frames.get(p.id)}
      {#if f}
        {@const st = tone(p)}
        <section
          data-node
          aria-label="{p.name}, {st === 'locked'
            ? 'locked'
            : st === 'done'
              ? 'done'
              : st === 'active'
                ? 'current'
                : 'upcoming'}"
          class="absolute rounded-[18px] border px-4 py-3.5 transition-[border-color,box-shadow] {st === 'locked'
            ? 'border-dashed border-border-strong'
            : st === 'active'
              ? 'border-accent-line'
              : st === 'done'
                ? 'border-ok/40'
                : 'border-border-strong'} {sel?.kind === 'phase' && sel.id === p.id
            ? 'ring-[1.5px] ring-accent'
            : ''} {overFrame?.id === p.id && drag?.from !== p.id ? 'ring-2 ring-accent' : ''}"
          style="left: {f.x}px; top: {f.y}px; width: {f.w}px; height: {f.h}px; background: {st === 'locked'
            ? 'repeating-linear-gradient(135deg, transparent 0 10px, color-mix(in srgb, var(--sunk) 85%, transparent) 10px 20px)'
            : st === 'active'
              ? 'color-mix(in srgb, var(--accent-soft) 40%, transparent)'
              : 'color-mix(in srgb, var(--surface) 60%, transparent)'}"
        >
          <button
            type="button"
            onclick={() => (sel = { kind: 'phase', id: p.id })}
            class="flex w-full cursor-pointer items-center gap-2 text-left text-[0.9375rem] font-semibold"
          >
            {#if st === 'done'}<CircleCheck class="size-4 text-ok" />
            {:else if st === 'locked'}<Lock class="size-4 text-icon" />
            {:else if st === 'active'}<CircleDot class="size-4 text-accent" />
            {:else}<Circle class="size-4 text-icon" />{/if}
            <span class="min-w-0 flex-1 truncate">{p.name}</span>
            <span class="text-[0.7812rem] font-medium text-muted tabular-nums">{pct(p.progress)}</span>
          </button>
          <div class="mt-2 flex flex-wrap items-center gap-1.5 text-[0.75rem] text-muted">
            {#if span(p)}<span class="rounded-full bg-sunk px-2 py-0.5">{span(p)}</span>{/if}
            {#if p.gate_title}
              <span
                class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 {p.locked
                  ? 'bg-accent-soft text-accent-text'
                  : 'bg-ok-soft text-ok-text'}"
              >
                {#if p.locked}<Lock class="size-3" />{:else}<LockOpen class="size-3" />{/if}
                After {p.gate_title}
              </span>
            {/if}
            {#if !p.items.length}<span>Empty. Drag a step here.</span>{/if}
          </div>
          <div class="absolute right-4 bottom-3 left-4 h-1 overflow-hidden rounded-full bg-sunk">
            <div class="h-full rounded-full bg-ok" style="width: {pct(p.progress)}"></div>
          </div>
        </section>
      {/if}
    {/each}

    {#each lay.steps as s (s.id)}
      {@const locked = frames.get(s.phaseId)?.state === 'locked'}
      {@const moving = drag?.live && drag.id === s.id}
      <div
        data-node
        data-step={s.id}
        role="button"
        tabindex="0"
        aria-label="{s.item.title}, {s.item.kind}, {pct(s.item.progress)}"
        onpointerdown={(e) => grab(e, s.id, s.phaseId)}
        onpointermove={carry}
        onpointerup={drop}
        onkeydown={(e) =>
          (e.key === 'Enter' || e.key === ' ') && ((sel = { kind: 'step', id: s.id }), e.preventDefault())}
        class="absolute flex touch-none flex-col justify-center border bg-surface px-3 text-[0.8438rem] select-none {s.item
          .kind === 'milestone'
          ? 'rounded-full border-warn/40 bg-warn-soft'
          : 'rounded-xl'} {locked ? 'border-dashed border-border-strong' : 'border-line shadow-1'} {sel?.kind ===
          'step' && sel.id === s.id
          ? 'ring-[1.5px] ring-accent'
          : ''} {moving ? 'z-20 cursor-grabbing opacity-90 shadow-2' : 'cursor-grab'}"
        style="left: {s.x}px; top: {s.y}px; width: {s.w}px; height: {s.h}px; {moving
          ? `transform: translate(${drag!.dx / view.z}px, ${drag!.dy / view.z}px)`
          : ''}"
      >
        <div class="flex items-center gap-2 font-medium">
          {@render icon(s.item.kind, s.item.progress)}
          <span class="min-w-0 flex-1 truncate">{s.item.title}</span>
        </div>
        {#if s.item.kind !== 'milestone'}
          <div class="mt-1.5 flex items-center gap-2 text-[0.75rem] text-muted">
            <span class="h-1 flex-1 overflow-hidden rounded-full bg-sunk">
              <span
                class="block h-full rounded-full {s.item.progress >= 1 ? 'bg-ok' : 'bg-accent'}"
                style="width: {pct(s.item.progress)}"
              ></span>
            </span>
            <span class="tabular-nums">{pct(s.item.progress)}</span>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  {#if !phases.length}
    <div class="absolute inset-0 grid place-items-center text-center text-sm text-muted">
      <p>Start with a phase. Phases hold steps, and a milestone can unlock the next one.</p>
    </div>
  {/if}

  {#if unplaced.length}
    <div
      data-ui
      class="absolute top-3 left-3 z-10 flex max-h-[60%] w-52 flex-col gap-1 overflow-auto rounded-xl border border-line bg-surface p-2 shadow-1"
    >
      <p class="px-1 text-[0.75rem] font-semibold text-muted">Unplaced · drag onto a phase</p>
      {#each unplaced as c (c.id)}
        {@const moving = drag?.live && drag.id === c.id}
        <div
          role="listitem"
          onpointerdown={(e) => grab(e, c.id, null)}
          onpointermove={carry}
          onpointerup={drop}
          class="flex cursor-grab touch-none items-center gap-2 rounded-md border border-line bg-surface px-2 py-1 text-[0.8125rem] select-none {moving
            ? 'relative z-30 opacity-80 shadow-2'
            : ''}"
          style={moving ? `transform: translate(${drag!.dx}px, ${drag!.dy}px)` : ''}
        >
          {@render icon(c.kind, 0)}
          <span class="task-title-1 flex-1" title={c.title}>{c.title}</span>
        </div>
      {/each}
    </div>
  {/if}

  <div
    data-ui
    class="absolute bottom-3 left-3 z-10 flex items-center gap-0.5 rounded-[10px] border border-line bg-surface p-1 shadow-1"
  >
    <Button variant="ghost" size="sm" aria-label="Zoom out" onclick={() => zoom(1 / 1.2)}
      ><Minus class="size-4" /></Button
    >
    <span class="w-12 text-center text-[0.7812rem] tabular-nums" aria-live="polite">{Math.round(view.z * 100)}%</span>
    <Button variant="ghost" size="sm" aria-label="Zoom in" onclick={() => zoom(1.2)}><Plus class="size-4" /></Button>
    <Button
      variant="ghost"
      size="sm"
      aria-label="Fit to screen"
      onclick={() => el && (view = fit(lay.frames, el.clientWidth, el.clientHeight))}
      ><Maximize class="size-4" /></Button
    >
  </div>

  {#if selPhase || selStep}
    <aside
      data-ui
      aria-label="Edit {selPhase ? 'phase' : 'step'}"
      class="absolute top-3 right-3 z-20 max-h-[calc(100%-24px)] w-[300px] overflow-auto rounded-[14px] border border-line bg-surface p-3.5 shadow-2"
    >
      <div class="flex items-center gap-2">
        <h2 class="flex-1 text-[0.7812rem] font-semibold text-muted">{selPhase ? 'Phase' : 'Step'}</h2>
        <Button variant="ghost" size="sm" aria-label="Close panel" onclick={() => (sel = null)}
          ><X class="size-4" /></Button
        >
      </div>
      {#if selPhase}
        {@const p = selPhase}
        {#key p.id}
          <input
            aria-label="Phase name"
            value={p.name}
            onblur={(e) => rename(e, p)}
            onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
            class="mt-1.5 w-full rounded-md border border-transparent bg-transparent px-1.5 py-1 text-[1rem] font-semibold outline-none focus:border-accent-line"
          />
        {/key}
        <dl class="my-3 grid grid-cols-[72px_1fr] items-center gap-x-2.5 gap-y-2 text-[0.8438rem]">
          <dt class="text-muted"><label for="rc-start">Start</label></dt>
          <dd>
            <input
              id="rc-start"
              type="date"
              value={p.start ?? ''}
              onchange={(e) => setDate(p, 'start', e.currentTarget.value)}
              class="h-8 w-full rounded-md border border-line bg-surface px-2"
            />
          </dd>
          <dt class="text-muted"><label for="rc-end">End</label></dt>
          <dd>
            <input
              id="rc-end"
              type="date"
              value={p.end ?? ''}
              onchange={(e) => setDate(p, 'end', e.currentTarget.value)}
              class="h-8 w-full rounded-md border border-line bg-surface px-2"
            />
          </dd>
          <dt class="text-muted"><label for="rc-gate">Opens</label></dt>
          <dd>
            <select
              id="rc-gate"
              value={p.gate_id ?? ''}
              onchange={(e) => gate(p, e.currentTarget.value)}
              class="h-8 w-full rounded-md border border-line bg-surface px-2"
            >
              <option value="">Right away</option>
              {#each gates as m (m.id)}
                <option value={m.id}>After “{m.title}”{m.done ? ' ✓' : ''}</option>
              {/each}
            </select>
          </dd>
        </dl>
        {#if p.locked}
          <p class="mb-3 flex items-start gap-2 rounded-lg bg-accent-soft p-2 text-[0.7812rem] text-accent-text">
            <Lock class="mt-0.5 size-3.5 shrink-0" /> Locked until “{p.gate_title}” is done. Its items stay workable.
          </p>
        {/if}
        <Button variant={p.completed ? 'ghost' : 'primary'} size="sm" onclick={() => complete(p)}>
          {p.completed ? 'Reopen phase' : 'Complete phase'}
        </Button>
      {:else if selStep}
        {@const s = selStep}
        <p class="mt-1.5 truncate text-[1rem] font-semibold">{s.item.title}</p>
        <p class="text-[0.7812rem] text-muted">{s.item.kind} · {pct(s.item.progress)}</p>
        <dl class="my-3 grid grid-cols-[72px_1fr] items-start gap-x-2.5 gap-y-2 text-[0.8438rem]">
          <dt class="pt-1.5 text-muted"><label for="rc-phase">Phase</label></dt>
          <dd>
            <select
              id="rc-phase"
              value={s.phaseId}
              onchange={(e) => onplace(s.id, e.currentTarget.value || null)}
              class="h-8 w-full rounded-md border border-line bg-surface px-2"
            >
              {#each phases as q (q.id)}
                <option value={q.id}>{q.name}</option>
              {/each}
              <option value="">Unplaced</option>
            </select>
          </dd>
          <dt class="pt-1.5 text-muted">Then</dt>
          <dd class="flex flex-col gap-1.5">
            <ul class="flex flex-col gap-1">
              {#each s.item.links as to (to)}
                <li class="flex items-center gap-1 rounded-md bg-sunk px-2 py-1 text-[0.7812rem]">
                  <span class="min-w-0 flex-1 truncate">{titleOf(to)}</span>
                  <button
                    type="button"
                    aria-label="Remove link to {titleOf(to)}"
                    class="grid size-5 place-items-center rounded hover:bg-hover"
                    onclick={() =>
                      links(
                        s,
                        s.item.links.filter((x) => x !== to),
                      )}><X class="size-3.5" /></button
                  >
                </li>
              {/each}
            </ul>
            <select
              aria-label="Add link"
              value=""
              onchange={(e) => {
                const v = e.currentTarget.value
                e.currentTarget.value = ''
                if (v) links(s, [...s.item.links, v])
              }}
              class="h-8 w-full rounded-md border border-line bg-surface px-2 text-muted"
            >
              <option value="">Add a next step…</option>
              {#each lay.steps.filter((o) => o.id !== s.id && !s.item.links.includes(o.id)) as o (o.id)}
                <option value={o.id}>{o.item.title}</option>
              {/each}
            </select>
          </dd>
        </dl>
        <Button variant="ghost" size="sm" onclick={() => open(s.id)}>Open item</Button>
      {/if}
    </aside>
  {/if}
</div>

<style>
  [data-node] {
    transition: box-shadow 0.12s;
  }
</style>
