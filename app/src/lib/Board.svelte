<script lang="ts" module>
  import type { Label, nameIndex, runView } from '$lib/sync'

  export type BoardRow = Awaited<ReturnType<typeof runView>>['groups'][number]['rows'][number]
  export type Names = Awaited<ReturnType<typeof nameIndex>>
  export type CellCtx = { labels: Map<string, Label>; names: Names }

  export const fieldChoices = [
    'due',
    'deadline',
    'priority',
    'label',
    'project',
    'area',
    'goal',
    'created_at',
    'touched_at',
    'completed',
  ]
  /** What a card or column shows when the View picks no fields. */
  export function defaultFields(entity: string): string[] {
    if (entity === 'project') return ['area', 'touched_at']
    if (entity === 'goal') return ['due']
    if (entity === 'page') return ['project', 'area']
    if (entity === 'plan') return ['area', 'due', 'touched_at']
    return ['due', 'priority', 'label']
  }
  export const fieldTitle = (f: string) => f.replaceAll('_', ' ')
  /** The sort path that orders by field `f`. */
  export const sortPath = (f: string) => (f === 'project' || f === 'area' || f === 'goal' ? `${f}.name` : f)

  const ancestor = (row: BoardRow, kind: string, names: Names) => {
    let next: string | null = 'parent_id' in row ? row.parent_id : null
    for (let i = 0; next && i < 12; i++) {
      const hit = names.get(next)
      if (!hit) return undefined
      if (hit.kind === kind) return hit.name
      next = hit.parent_id
    }
    return undefined
  }
  const dayText = (s: string) => {
    const [y, m, d] = s.split('-').map(Number)
    return new Date(y, m - 1, d).toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' })
  }
  const stamp = (ms: unknown) =>
    typeof ms === 'number' ? new Date(ms).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' }) : ''

  /** Plain text of one field of a row; empty when the row has none. */
  export function cellText(row: BoardRow, field: string, cx: CellCtx): string {
    const r = row as Record<string, unknown>
    switch (field) {
      case 'due': {
        const d = row.kind === 'task' ? row.due : row.kind === 'goal' ? { date: row.target_date } : null
        return d?.date ? `${dayText(d.date)}${'time' in d && d.time ? ` ${d.time}` : ''}` : ''
      }
      case 'deadline':
        return row.kind === 'task' && row.deadline ? dayText(row.deadline) : ''
      case 'priority':
        return row.kind === 'task' ? `P${row.priority}` : ''
      case 'label':
        return row.kind === 'task'
          ? row.label_ids
              .map((l) => {
                const name = cx.labels.get(l)?.name
                return name ? `#${name}` : ''
              })
              .filter(Boolean)
              .join(', ')
          : ''
      case 'project':
      case 'area':
        return ancestor(row, field, cx.names) ?? ''
      case 'goal': {
        const id = row.kind === 'task' || row.kind === 'project' ? row.goal_id : row.kind === 'goal' ? row.id : null
        return id ? (cx.names.get(id)?.name ?? '') : ''
      }
      case 'created_at':
      case 'touched_at':
        return stamp(r[field])
      case 'completed':
        return r.completed_at != null ? 'Done' : ''
      default:
        return ''
    }
  }
  export const rowTitle = (r: BoardRow) =>
    r.kind === 'task' || r.kind === 'page' || r.kind === 'milestone'
      ? r.title
      : r.kind === 'project' || r.kind === 'goal' || r.kind === 'area' || r.kind === 'section' || r.kind === 'view'
        ? r.name
        : r.id
</script>

<script lang="ts">
  import { dndzone, TRIGGERS, type DndEvent } from 'svelte-dnd-action'
  import Check from '@lucide/svelte/icons/check'
  import {
    completeTask,
    labels as listLabels,
    dayFrom,
    moveItem,
    nameIndex as loadNames,
    orderBetween,
    setDue,
    setLabels,
    setPriority,
    updateProject,
    updateSection,
    updateTask,
  } from '$lib/sync'

  type Column = { key: string; title: string; rows: BoardRow[]; target: boolean }

  let {
    groupBy,
    groups,
    fields,
    entity = 'task',
    titleOf = (key: string) => key,
    extra = [],
    scopeId = '',
    sectionsEditable = false,
    onchange,
    onopen,
  }: {
    groupBy: string | null
    groups: { key: string; rows: BoardRow[] }[]
    fields: string[]
    entity?: string
    /** Column heading for a group key. */
    titleOf?: (key: string) => string
    /** Columns to show even when empty (sections of the scope, areas, goals…). */
    extra?: { key: string; title: string }[]
    /** Where a card dropped in the "no group" Section column goes. */
    scopeId?: string
    /** Section columns in an Area or Project task board can be renamed inline. */
    sectionsEditable?: boolean
    onchange: () => void
    onopen: (row: BoardRow) => void
  } = $props()

  let cx = $state<CellCtx>({ labels: new Map(), names: new Map() })
  $effect(() => {
    void groups
    Promise.all([listLabels(), loadNames()]).then(
      ([l, names]) => (cx = { labels: new Map(l.map((x) => [x.id, x])), names }),
    )
  })

  const shown = $derived(fields.length ? fields : defaultFields(entity))
  const all = $derived(groups.flatMap((g) => g.rows))
  const kinds = $derived(new Set(all.map((r) => r.kind)))
  // Dragging edits a field, so it needs a groupable field the dragged kind can change.
  const draggable = $derived.by(() => {
    if (kinds.size === 0) return false
    if (kinds.has('task') && kinds.size === 1)
      return ['section', 'priority', 'label', 'due', 'project', 'area', 'goal'].includes(groupBy ?? '')
    if (kinds.has('project') && kinds.size === 1) return groupBy === 'area' || groupBy === 'goal'
    return false
  })

  const built = $derived.by((): Column[] => {
    const col = (key: string, title: string, rows: BoardRow[], target = true): Column => ({ key, title, rows, target })
    const rowsOf = (key: string) => groups.find((g) => g.key === key)?.rows ?? []
    if (groupBy === 'priority') {
      return ['1', '2', '3', '4'].map((k) => col(k, `P${k}`, rowsOf(k)))
    }
    if (groupBy === 'due') {
      const dates = new Set([dayFrom(0), dayFrom(1), ...groups.map((g) => g.key)])
      dates.delete('Overdue')
      dates.delete('No date')
      const out = [col('Overdue', 'Overdue', rowsOf('Overdue'), false)]
      for (const d of [...dates].sort()) out.push(col(d, titleOf(d), rowsOf(d)))
      return [...out, col('No date', 'No date', rowsOf('No date'))]
    }
    if (groupBy === 'label') {
      const tasks = all.filter((r) => r.kind === 'task')
      const out = [...cx.labels.values()].map((l) =>
        col(
          l.id,
          l.name,
          tasks.filter((t) => t.kind === 'task' && t.label_ids.includes(l.id)),
        ),
      )
      return [
        ...out,
        col(
          '',
          'No label',
          tasks.filter((t) => t.kind === 'task' && t.label_ids.length === 0),
        ),
      ]
    }
    const seen = new Set<string>()
    const out: Column[] = []
    for (const e of extra) {
      seen.add(e.key)
      out.push(col(e.key, e.title, rowsOf(e.key)))
    }
    for (const g of groups) {
      if (seen.has(g.key)) continue
      const none = g.key === ''
      out.push(col(g.key, none ? 'None' : titleOf(g.key), g.rows, groupBy === 'section' || groupBy === 'goal' || !none))
    }
    return groupBy === 'section' ? out : out.sort((a, b) => Number(a.key === '') - Number(b.key === ''))
  })

  let cols = $state<Column[]>([])
  $effect(() => {
    cols = built.map((c) => ({ ...c }))
  })
  let source = ''

  function consider(c: Column, e: CustomEvent<DndEvent<BoardRow>>) {
    if (e.detail.info.trigger === TRIGGERS.DRAG_STARTED) source = c.key
    c.rows = e.detail.items
  }

  async function finalize(c: Column, e: CustomEvent<DndEvent<BoardRow>>) {
    c.rows = e.detail.items
    const { id, trigger } = e.detail.info
    if (trigger !== TRIGGERS.DROPPED_INTO_ZONE) return
    const i = c.rows.findIndex((r) => r.id === id)
    const row = c.rows[i]
    if (!row) return
    const prev = c.rows[i - 1] as { order?: string } | undefined
    const next = c.rows[i + 1] as { order?: string } | undefined
    const order = orderBetween(prev?.order, next?.order)
    const from = source
    if (from === c.key) {
      if ('parent_id' in row) await moveItem(id, row.parent_id, order)
    } else if (row.kind === 'task') {
      if (groupBy === 'priority') await setPriority(id, Number(c.key) as 1 | 2 | 3 | 4)
      else if (groupBy === 'due') await setDue(id, c.key === 'No date' ? null : c.key)
      else if (groupBy === 'label') {
        const keep = row.label_ids.filter((l) => l !== from)
        await setLabels(id, c.key && !keep.includes(c.key) ? [...keep, c.key] : keep)
      } else if (groupBy === 'goal') await updateTask(id, { goal_id: c.key || null })
      else if (groupBy === 'section') await moveItem(id, c.key || scopeId, order)
      else if (groupBy === 'project' || groupBy === 'area') await moveItem(id, c.key, order)
    } else if (row.kind === 'project') {
      if (groupBy === 'area') await moveItem(id, c.key, order)
      else if (groupBy === 'goal') await updateProject(id, { goal_id: c.key || null })
    }
    onchange()
  }

  const chips = (r: BoardRow) => shown.map((f) => ({ f, text: cellText(r, f, cx) })).filter((x) => x.text)
</script>

<div class="flex gap-3 overflow-x-auto pb-3" role="group" aria-label="Board">
  {#each cols as c (c.key)}
    <section aria-label="{c.title} column" class="flex w-[260px] shrink-0 flex-col rounded-xl bg-hover/50 p-2">
      <h3 class="mb-2 flex items-center gap-1 px-1 text-[0.7812rem] font-semibold text-muted">
        {#if sectionsEditable && c.key}
          <input
            value={c.title}
            aria-label="Rename section {c.title}"
            onblur={(e) =>
              e.currentTarget.value.trim() &&
              updateSection(c.key, { name: e.currentTarget.value.trim() }).then(onchange)}
            onkeydown={(e) => {
              if (e.key === 'Enter') e.currentTarget.blur()
              if (e.key === 'Escape') {
                e.currentTarget.value = c.title
                e.currentTarget.blur()
              }
            }}
            class="min-w-0 flex-1 bg-transparent outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        {:else}
          <span class="min-w-0 flex-1">{c.title}</span>
        {/if}
        <span class="font-normal">{c.rows.length}</span>
      </h3>
      <ul
        class="flex min-h-[3rem] flex-1 flex-col gap-2"
        use:dndzone={{
          items: c.rows,
          type: 'card',
          flipDurationMs: 150,
          delayTouchStart: true,
          dragDisabled: !draggable,
          dropFromOthersDisabled: !draggable || !c.target,
        }}
        onconsider={(e) => consider(c, e)}
        onfinalize={(e) => finalize(c, e)}
      >
        {#each c.rows as r (r.id)}
          <li
            aria-label={rowTitle(r)}
            data-task-row={r.kind === 'task' ? r.id : undefined}
            class="rounded-[9px] border border-line bg-surface p-2 shadow-1 data-[cursor=true]:bg-hover data-[cursor=true]:ring-2 data-[cursor=true]:ring-accent"
          >
            {#if r.kind === 'task'}
              <button
                type="button"
                aria-label="Complete {rowTitle(r)}"
                onpointerdown={(e) => e.stopPropagation()}
                onclick={() => completeTask(r.id)}
                class="grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
              >
                <Check size={12} strokeWidth={3} />
              </button>
            {/if}
            <button
              type="button"
              data-open
              class="task-title-2 block w-full text-left"
              title={rowTitle(r)}
              onclick={() => onopen(r)}>{rowTitle(r)}</button
            >
            {#if chips(r).length}
              <div class="mt-1 flex flex-wrap gap-1">
                {#each chips(r) as x (x.f)}
                  <span class="rounded-full border border-line px-2 py-px text-xs text-muted">{x.text}</span>
                {/each}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</div>
