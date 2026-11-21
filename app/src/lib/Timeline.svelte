<script lang="ts">
  import type { BoardRow } from '$lib/Board.svelte'
  import { timelineData, updateGoal } from '$lib/sync'

  let {
    rows,
    areas = [],
    onchange,
    onopen,
  }: {
    rows: BoardRow[]
    areas?: unknown
    onchange: () => void
    onopen: (row: BoardRow) => void
  } = $props()

  type Data = Awaited<ReturnType<typeof timelineData>>
  const zooms = [
    ['week', 'Week', 48, 1],
    ['month', 'Month', 16, 1],
    ['quarter', 'Quarter', 6, 7],
    ['year', 'Year', 2, 7],
  ] as const
  let zoom = $state<(typeof zooms)[number][0]>('month')
  const px = $derived(zooms.find((z) => z[0] === zoom)![2])
  const step = $derived(zooms.find((z) => z[0] === zoom)![3])
  let data = $state<Data>(new Map())
  // A bar being dragged: which end, and its day offset so far.
  let drag = $state<{ id: string; end: 'start' | 'end'; delta: number } | null>(null)
  const goals = $derived(rows.filter((r) => r.kind === 'goal'))

  $effect(() => {
    void areas
    const ids = goals.map((g) => g.id)
    timelineData(ids).then((d) => (data = d))
  })

  const DAY = 86_400_000
  const num = (d: string) => {
    const [y, m, dd] = d.split('-').map(Number)
    return Math.round(Date.UTC(y, m - 1, dd) / DAY)
  }
  const str = (n: number) => new Date(n * DAY).toISOString().slice(0, 10)
  const today = () => {
    const d = new Date()
    return num(
      `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`,
    )
  }
  const fmt = (n: number) =>
    new Date(n * DAY).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', timeZone: 'UTC' })
  const fmtMonth = (n: number) =>
    new Date(n * DAY).toLocaleDateString('en-GB', { month: 'long', year: 'numeric', timeZone: 'UTC' })

  type G = Extract<BoardRow, { kind: 'goal' }>
  type Item = { id: string; title: string; day: number }
  type Line = {
    row: BoardRow
    depth: number
    start?: number
    end?: number
    open?: boolean
    milestones: Item[]
    ticks: Item[]
    point?: number
  }

  const lines = $derived.by<Line[]>(() => {
    const t = today()
    const ids = new Set(goals.map((g) => g.id))
    const kids = (pid: string) => goals.filter((g) => g.parent_id === pid)
    const out: Line[] = []
    const goalLine = (g: G, depth: number) => {
      const d = data.get(g.id)
      const end = g.target_date ? num(g.target_date) : undefined
      const start = g.start ? num(g.start) : end !== undefined ? end - 30 : t
      out.push({
        row: g,
        depth,
        start,
        end,
        open: end === undefined,
        milestones: (d?.milestones ?? []).flatMap((m) =>
          m.date ? [{ id: m.id, title: m.title, day: num(m.date) }] : [],
        ),
        ticks: (d?.ticks ?? []).map((k) => ({ ...k, day: num(k.date) })),
      })
      for (const c of kids(g.id)) goalLine(c as G, depth + 1)
    }
    for (const g of goals) if (!ids.has(g.parent_id)) goalLine(g as G, 0)
    for (const r of rows) {
      if (r.kind === 'goal') continue
      const day =
        r.kind === 'task' && r.due
          ? num(r.due.date)
          : 'created_at' in r && typeof r.created_at === 'number'
            ? Math.floor(r.created_at / DAY)
            : undefined
      out.push({ row: r, depth: 0, milestones: [], ticks: [], point: day })
    }
    return out
  })

  const span = $derived.by(() => {
    const t = today()
    const days = [t]
    for (const l of lines) {
      if (l.start !== undefined) days.push(l.start)
      if (l.end !== undefined) days.push(l.end)
      for (const m of [...l.milestones, ...l.ticks]) days.push(m.day)
      if (l.point !== undefined) days.push(l.point)
    }
    const lo = Math.min(...days) - 7
    const hi = Math.max(...days) + 21
    return { lo, hi }
  })
  const x = (day: number) => (day - span.lo) * px
  const width = $derived((span.hi - span.lo) * px)

  const ticks = $derived.by(() => {
    const out: { day: number; label: string }[] = []
    for (let d = span.lo; d <= span.hi; d++) {
      const dt = new Date(d * DAY)
      const first = dt.getUTCDate() === 1
      if (zoom === 'week' ? dt.getUTCDay() === 1 : zoom === 'year' ? first && dt.getUTCMonth() % 3 === 0 : first)
        out.push({ day: d, label: zoom === 'week' ? fmt(d) : fmtMonth(d) })
    }
    return out
  })

  function title(r: BoardRow) {
    if (r.kind === 'task' || r.kind === 'page' || r.kind === 'milestone') return r.title
    return 'name' in r ? r.name : r.id
  }

  function bounds(l: Line) {
    let s = l.start!
    let e = l.end ?? span.hi
    if (drag && drag.id === l.row.id) {
      if (drag.end === 'start') s += drag.delta
      else e += drag.delta
    }
    return { s, e }
  }
  const label = (l: Line) => {
    const { s, e } = bounds(l)
    return `Goal ${title(l.row)}, ${fmt(s)} to ${l.open ? 'open end' : fmt(e)}`
  }

  async function commit(l: Line, end: 'start' | 'end', delta: number) {
    if (!delta) return
    if (end === 'start') await updateGoal(l.row.id, { start: str(l.start! + delta) })
    else await updateGoal(l.row.id, { target_date: str((l.end ?? span.hi) + delta) })
    onchange()
  }
  function down(e: PointerEvent, l: Line, end: 'start' | 'end') {
    const el = e.currentTarget as HTMLElement
    el.setPointerCapture(e.pointerId)
    const x0 = e.clientX
    drag = { id: l.row.id, end, delta: 0 }
    const move = (ev: PointerEvent) => (drag = { id: l.row.id, end, delta: Math.round((ev.clientX - x0) / px) })
    const up = async () => {
      el.removeEventListener('pointermove', move)
      el.removeEventListener('pointerup', up)
      el.removeEventListener('pointercancel', up)
      const d = drag?.delta ?? 0
      drag = null
      await commit(l, end, d)
    }
    el.addEventListener('pointermove', move)
    el.addEventListener('pointerup', up)
    el.addEventListener('pointercancel', up)
  }
  function key(e: KeyboardEvent, l: Line, end: 'start' | 'end') {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
    e.preventDefault()
    commit(l, end, e.key === 'ArrowLeft' ? -step : step)
  }

  // Phone fallback: every dated thing, grouped by month.
  const months = $derived.by(() => {
    const items: { day: number; text: string; row: BoardRow; kind: string }[] = []
    for (const l of lines) {
      if (l.point !== undefined) items.push({ day: l.point, text: title(l.row), row: l.row, kind: l.row.kind })
      if (l.end !== undefined) items.push({ day: l.end, text: `${title(l.row)} target`, row: l.row, kind: 'goal' })
      for (const m of l.milestones) items.push({ day: m.day, text: `◆ ${m.title}`, row: l.row, kind: 'milestone' })
    }
    items.sort((a, b) => a.day - b.day)
    const groups: { key: string; items: typeof items }[] = []
    for (const i of items) {
      const k = fmtMonth(i.day)
      const g = groups.at(-1)
      if (g?.key === k) g.items.push(i)
      else groups.push({ key: k, items: [i] })
    }
    return groups
  })
</script>

<div role="group" aria-label="Timeline zoom" class="mb-2 hidden gap-1 desk:flex">
  {#each zooms as [value, text] (value)}
    <button
      type="button"
      aria-pressed={zoom === value}
      onclick={() => (zoom = value)}
      class="h-7 rounded-md border px-2.5 text-sm {zoom === value
        ? 'border-accent-line bg-accent-soft text-text'
        : 'border-line bg-surface text-muted hover:bg-hover'}">{text}</button
    >
  {/each}
</div>

<div class="hidden overflow-x-auto rounded-xl border border-line bg-surface desk:block" aria-label="Timeline">
  <div class="relative" style="width:{width + 200}px">
    <div class="relative h-7 border-b border-line text-xs text-muted" style="margin-left:200px">
      {#each ticks as t (t.day)}
        <span class="absolute top-1.5 border-l border-line pl-1 whitespace-nowrap" style="left:{x(t.day)}px"
          >{t.label}</span
        >
      {/each}
    </div>
    {#each lines as l (l.row.id)}
      <div class="relative flex h-10 items-center border-b border-line/50">
        <button
          type="button"
          onclick={() => onopen(l.row)}
          class="sticky left-0 z-10 w-[200px] shrink-0 truncate bg-surface px-2 text-left text-sm hover:bg-hover"
          style="padding-left:{8 + l.depth * 16}px">{title(l.row)}</button
        >
        <div class="relative h-full" style="width:{width}px">
          <div
            class="absolute top-0 bottom-0 w-px bg-accent"
            style="left:{x(today())}px"
            aria-hidden="true"
            title="Today"
          ></div>
          {#if l.start !== undefined}
            {@const b = bounds(l)}
            <div
              class="absolute top-3 h-4 rounded-md bg-accent-soft {l.open
                ? 'border border-dashed border-accent-line'
                : 'border border-accent-line'}"
              style="left:{x(b.s)}px;width:{Math.max(4, (b.e - b.s) * px)}px"
              role="img"
              aria-label={label(l)}
            ></div>
            <button
              type="button"
              aria-label="Start of {title(l.row)}, {fmt(b.s)}"
              onpointerdown={(e) => down(e, l, 'start')}
              onkeydown={(e) => key(e, l, 'start')}
              class="absolute top-2.5 h-5 w-2 cursor-ew-resize touch-none rounded-sm bg-accent"
              style="left:{x(b.s) - 1}px"
            ></button>
            {#if !l.open}
              <button
                type="button"
                aria-label="End of {title(l.row)}, {fmt(b.e)}"
                onpointerdown={(e) => down(e, l, 'end')}
                onkeydown={(e) => key(e, l, 'end')}
                class="absolute top-2.5 h-5 w-2 cursor-ew-resize touch-none rounded-sm bg-accent"
                style="left:{x(b.e) - 7}px"
              ></button>
            {/if}
          {/if}
          {#each l.milestones as m (m.id)}
            <span
              role="img"
              aria-label="Milestone {m.title}, {fmt(m.day)}"
              title={m.title}
              class="absolute top-[13px] size-3.5 rotate-45 bg-ok"
              style="left:{x(m.day) - 7}px"
            ></span>
          {/each}
          {#each l.ticks as k (k.id)}
            <span
              role="img"
              aria-label="Task {k.title}, due {fmt(k.day)}"
              title={k.title}
              class="absolute top-2 h-6 w-0.5 bg-muted"
              style="left:{x(k.day)}px"
            ></span>
          {/each}
          {#if l.point !== undefined}
            <span
              role="img"
              aria-label="{title(l.row)}, {fmt(l.point)}"
              class="absolute top-[15px] size-2.5 rounded-full bg-accent"
              style="left:{x(l.point) - 5}px"
            ></span>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<div class="desk:hidden" aria-label="Timeline list">
  {#each months as g (g.key)}
    <h3 class="mt-4 mb-1 px-2 text-[0.7812rem] font-semibold text-muted">{g.key}</h3>
    <ul>
      {#each g.items as i, n (n)}
        <li>
          <button
            type="button"
            onclick={() => onopen(i.row)}
            class="flex min-h-[2.625rem] w-full min-w-0 items-center gap-3 rounded-[9px] px-2 text-left hover:bg-hover"
          >
            <span class="task-title-1 flex-1" title={i.text}>{i.text}</span>
            <span class="text-xs text-muted">{fmt(i.day)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/each}
</div>
