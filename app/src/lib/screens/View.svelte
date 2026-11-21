<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import SearchX from '@lucide/svelte/icons/search-x'
  import AlertTriangle from '@lucide/svelte/icons/alert-triangle'
  import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down'
  import CalendarDays from '@lucide/svelte/icons/calendar-days'
  import Check from '@lucide/svelte/icons/check'
  import Clock from '@lucide/svelte/icons/clock'
  import InboxIcon from '@lucide/svelte/icons/inbox'
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import Board, { fieldChoices, fieldTitle } from '$lib/Board.svelte'
  import FilterEditor from '$lib/FilterEditor.svelte'
  import Table from '$lib/Table.svelte'
  import Timeline from '$lib/Timeline.svelte'
  import Calendar from '$lib/Calendar.svelte'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import EntityMenu from '$lib/EntityMenu.svelte'
  import {
    allGoals,
    completeTask,
    getView,
    resetView,
    runView,
    updateView,
    type Due,
    type Node,
    type Sort,
    type View as ViewRow,
    type ViewPatch,
    type tree,
  } from '$lib/sync'
  import { inbox_id } from '../../../wasm/orbit_wasm'

  let {
    id,
    areas,
    onchange,
  }: {
    id: string
    areas: Awaited<ReturnType<typeof tree>>
    onchange: () => void
  } = $props()

  type Row = Awaited<ReturnType<typeof runView>>['groups'][number]['rows'][number]
  type Group = Awaited<ReturnType<typeof runView>>['groups'][number]

  let view = $state<ViewRow | null>(null)
  let groups = $state<Group[]>([])
  let goalNames = $state(new Map<string, string>())
  let loaded = $state(false)
  let lastId = ''
  let filterOpen = $state(false)
  let sortOpen = $state(false)
  let fieldsOpen = $state(false)
  let openId = $state<string | null>(null)

  // Reloaded whenever the shell refreshes (`areas` is a fresh array after every sync).
  $effect(() => {
    if (id !== lastId) {
      lastId = id
      loaded = false
      filterOpen = false
      sortOpen = false
      fieldsOpen = false
      openId = null
    }
    void areas
    load()
  })

  async function load() {
    const next = (await getView(id)) ?? null
    view = next
    groups = (await runView(id)).groups
    goalNames = new Map((await allGoals()).map((g) => [g.id, g.name]))
    loaded = true
  }

  async function save(patch: ViewPatch & { kind?: ViewRow['entity'] }) {
    await updateView(id, patch)
    await load()
    onchange()
  }

  const sortFields = ['title', 'priority', 'due', 'deadline', 'created_at', 'touched_at']
  // The icon keys in builtins() (crates/orbit-core/src/view.rs), plus a fallback for custom views.
  const icons: Record<string, typeof Clock> = {
    inbox: InboxIcon,
    calendar: CalendarDays,
    alert: AlertTriangle,
    clock: Clock,
  }
  const groupFields: [string, string][] = [
    ['', 'No grouping'],
    ['due', 'Due'],
    ['project', 'Project'],
    ['area', 'Area'],
    ['priority', 'Priority'],
    ['goal', 'Goal'],
  ]

  // `timeline` and `calendar` join this list with their own tickets.
  const layouts: [string, string][] = [
    ['list', 'List'],
    ['board', 'Board'],
    ['table', 'Table'],
    ['timeline', 'Timeline'],
    ['calendar', 'Calendar'],
  ]
  const layout = $derived(
    view?.layout === 'board' || view?.layout === 'table' || view?.layout === 'timeline' || view?.layout === 'calendar'
      ? view.layout
      : 'list',
  )
  // Core groups by the fields above; Label is regrouped by the Board.
  const groupChoices = $derived(
    layout === 'board' ? [...groupFields, ['label', 'Label'] as [string, string]] : groupFields,
  )
  async function toggleField(f: string) {
    if (!view) return
    await save({ fields: view.fields.includes(f) ? view.fields.filter((x) => x !== f) : [...view.fields, f] })
  }
  const allRows = $derived(groups.flatMap((g) => g.rows))
  const extra = $derived.by(() => {
    if (view?.group_by === 'project') return areas.flatMap((a) => a.projects).map((p) => ({ key: p.id, title: p.name }))
    if (view?.group_by === 'area') return areas.map((a) => ({ key: a.area.id, title: a.area.name }))
    if (view?.group_by === 'goal') return [...goalNames].map(([key, title]) => ({ key, title }))
    return []
  })
  function open(r: Row) {
    if (r.kind === 'task') openId = r.id
    else if (href(r)) location.hash = href(r)!
  }
  function closeDrawer() {
    openId = null
    load()
    onchange()
  }

  async function addSort() {
    if (!view) return
    await save({ sorts: [...view.sorts, { field: 'due', desc: false }] })
  }
  async function saveSorts(sorts: Sort[]) {
    await save({ sorts: sorts.map((s) => ({ ...s })) })
  }
  async function fieldChanged() {
    if (view) await saveSorts(view.sorts)
  }
  async function removeSort(i: number) {
    if (!view) return
    await saveSorts(view.sorts.filter((_, n) => n !== i))
  }
  async function toggleSort(i: number) {
    if (!view) return
    await saveSorts(view.sorts.map((s, n) => (n === i ? { ...s, desc: !s.desc } : s)))
  }

  async function togglePin() {
    if (!view) return
    await save({ pinned: !view.pinned })
  }

  async function hide() {
    await updateView(id, { hidden: true })
    onchange()
    location.hash = '#/inbox'
  }
  async function reset() {
    await resetView(id)
    await load()
    onchange()
  }
  const hasFilter = (n: Node): boolean => n.type === 'cond' || n.children.length > 0
  const label = (key: string) => {
    if (!view || !key) return ''
    if (view.group_by === 'priority') return `P${key}`
    if (view.group_by === 'due') {
      if (key === 'Overdue' || key === 'No date') return key
      return /^\d{4}-\d{2}-\d{2}$/.test(key) ? dueText({ date: key }) : key
    }
    if (view.group_by === 'project' || view.group_by === 'area') {
      if (key === inbox_id()) return 'Inbox'
      const hit =
        areas.find((a) => a.area.id === key)?.area ?? areas.flatMap((a) => a.projects).find((p) => p.id === key)
      return hit?.name ?? key
    }
    if (view.group_by === 'goal') return goalNames.get(key) ?? key
    return key
  }
  const empty = $derived(groups.every((g) => g.rows.length === 0))

  const pad = (n: number) => String(n).padStart(2, '0')
  const today = () => {
    const d = new Date()
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
  }
  // A floating local date, shown like "Tue 25 Nov", plus its time.
  function dueText(due: Due) {
    const [y, m, d] = due.date.split('-').map(Number)
    const day = new Date(y, m - 1, d).toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' })
    return due.time ? `${day} ${due.time}` : day
  }

  const href = (r: Row) =>
    r.kind === 'project'
      ? `#/project/${r.id}`
      : r.kind === 'goal'
        ? '#/goals'
        : r.kind === 'page'
          ? `#/page/${r.id}`
          : null
  const title = (r: Row) =>
    r.kind === 'task' || r.kind === 'page' || r.kind === 'milestone'
      ? r.title
      : r.kind === 'project' || r.kind === 'goal' || r.kind === 'area' || r.kind === 'section' || r.kind === 'view'
        ? r.name
        : r.id

  async function complete(taskId: string) {
    await completeTask(taskId)
    await load()
    onchange()
  }
</script>

{#if view}
  <div class="relative mb-3">
    <div class="flex flex-wrap items-center gap-2">
      {#if icons[view.icon]}
        {@const Icon = icons[view.icon]}
        <Icon size={20} class="shrink-0 text-icon" />
      {/if}
      <h1 class="page-title mb-0 min-w-[8rem] flex-1">{view.name}</h1>
      {#if !view.builtin}
        <EntityMenu id={view.id} name={view.name} kind="view" fallbackHref="#/home" {onchange} />
      {/if}
      <Button
        variant={filterOpen || hasFilter(view.filter) ? 'on' : 'default'}
        aria-expanded={filterOpen}
        onclick={() => {
          sortOpen = false
          fieldsOpen = false
          filterOpen = !filterOpen
        }}
      >
        <SlidersHorizontal size={14} /> Filter
      </Button>
      <Button
        variant={sortOpen || view.sorts.length ? 'on' : 'default'}
        aria-expanded={sortOpen}
        onclick={() => {
          filterOpen = false
          fieldsOpen = false
          sortOpen = !sortOpen
        }}
      >
        <ArrowUpDown size={14} /> Sort
      </Button>
      <select
        aria-label="Kind"
        value={view.entity}
        onchange={(e) => save({ kind: e.currentTarget.value as ViewRow['entity'] })}
        class="h-8 rounded-md border border-line bg-surface px-2 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
      >
        {#each ['task', 'project', 'goal', 'page', 'plan', 'roadmap', 'phase'] as k (k)}
          <option value={k}>{k[0].toUpperCase() + k.slice(1)}s</option>
        {/each}
      </select>
      <select
        aria-label="Group by"
        value={view.group_by ?? ''}
        onchange={(e) => save({ group_by: e.currentTarget.value || null })}
        class="h-8 rounded-md border border-line bg-surface px-2 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
      >
        {#each groupChoices as [value, text] (value)}
          <option {value}>{text}</option>
        {/each}
      </select>
      <div role="group" aria-label="Layout" class="flex gap-1">
        {#each layouts as [value, text] (value)}
          <Button
            size="sm"
            variant={layout === value ? 'on' : 'default'}
            aria-pressed={layout === value}
            onclick={() => save({ layout: value })}>{text}</Button
          >
        {/each}
      </div>
      <Button
        variant={fieldsOpen || view.fields.length ? 'on' : 'default'}
        aria-expanded={fieldsOpen}
        onclick={() => {
          filterOpen = false
          sortOpen = false
          fieldsOpen = !fieldsOpen
        }}
      >
        Fields
      </Button>
      <label class="flex items-center gap-1.5 text-sm text-muted">
        <Checkbox
          checked={view.show_completed}
          aria-label="Show completed"
          onCheckedChange={(c) => save({ show_completed: c === true })}
        />
        Show completed
      </label>
      <Button variant={view.pinned ? 'on' : 'default'} aria-pressed={view.pinned} onclick={togglePin}>
        {view.pinned ? 'Pinned' : 'Pin to sidebar'}
      </Button>
      <Button variant="ghost" onclick={hide}>Hide</Button>
      {#if view.builtin}
        <Button variant="ghost" aria-label="Reset view" onclick={reset}>Reset</Button>
      {/if}
    </div>

    {#if filterOpen}
      <FilterEditor
        {id}
        filter={view.filter}
        onclose={() => (filterOpen = false)}
        onapplied={async () => {
          await load()
          onchange()
        }}
      />
    {/if}

    {#if sortOpen}
      <div
        role="group"
        aria-label="Sort"
        class="absolute top-full right-0 z-30 mt-1 w-[min(360px,calc(100vw/var(--zoom,1)-2rem))] rounded-xl border border-line bg-surface p-3 shadow-2"
      >
        {#each view.sorts as s, i (i)}
          <div class="mb-2 flex items-center gap-1.5">
            <select
              bind:value={s.field}
              aria-label="Sort field {i + 1}"
              onchange={fieldChanged}
              class="h-7 min-w-0 flex-1 rounded-md border border-line bg-surface px-2 text-sm outline-none"
            >
              {#each sortFields as f (f)}
                <option value={f}>{f.replaceAll('_', ' ')}</option>
              {/each}
            </select>
            <Button size="sm" variant={s.desc ? 'on' : 'default'} onclick={() => toggleSort(i)}>
              {s.desc ? 'Desc' : 'Asc'}
            </Button>
            <Button size="sm" variant="ghost" aria-label="Remove sort {i + 1}" onclick={() => removeSort(i)}
              >Remove</Button
            >
          </div>
        {/each}
        <Button size="sm" onclick={addSort}>Add sort</Button>
      </div>
    {/if}

    {#if fieldsOpen}
      <div
        role="group"
        aria-label="Fields"
        class="absolute top-full right-0 z-30 mt-1 w-[min(280px,calc(100vw/var(--zoom,1)-2rem))] rounded-xl border border-line bg-surface p-3 shadow-2"
      >
        {#each fieldChoices as f (f)}
          <label class="flex items-center gap-2 py-1 text-sm capitalize">
            <Checkbox
              checked={view.fields.includes(f)}
              aria-label="Show {fieldTitle(f)}"
              onCheckedChange={() => toggleField(f)}
            />
            {fieldTitle(f)}
          </label>
        {/each}
      </div>
    {/if}
  </div>
{/if}

{#if !loaded}
  <p class="px-2 py-6 text-center text-muted">Loading view…</p>
{:else if empty && layout !== 'calendar'}
  <Empty icon={SearchX} text="Nothing matches this view yet. Adjust its filter to see more." />
{:else if layout === 'board'}
  <Board
    groupBy={view?.group_by ?? null}
    {groups}
    fields={view?.fields ?? []}
    entity={view?.entity}
    titleOf={label}
    {extra}
    onchange={async () => {
      await load()
      onchange()
    }}
    onopen={open}
  />
{:else if layout === 'table'}
  <Table
    rows={allRows}
    fields={view?.fields ?? []}
    sorts={view?.sorts ?? []}
    entity={view?.entity}
    onsort={(sorts) => save({ sorts })}
    onopen={open}
  />
{:else if layout === 'timeline'}
  <Timeline
    rows={allRows}
    {areas}
    onchange={async () => {
      await load()
      onchange()
    }}
    onopen={open}
  />
{:else if layout === 'calendar'}
  <Calendar
    rows={allRows}
    onchange={async () => {
      await load()
      onchange()
    }}
    onopen={(id) => (openId = id)}
  />
{:else}
  {#each groups as g (g.key)}
    {#if g.key}<h3 class="mt-4 mb-1 px-2 text-[0.7812rem] font-semibold text-muted">{label(g.key)}</h3>{/if}
    <ul>
      {#each g.rows as r (r.id)}
        <li class="flex min-h-[2.625rem] items-center gap-3 rounded-[9px] px-2 hover:bg-hover">
          {#if r.kind === 'task'}
            {#if r.completed_at != null}
              <span
                role="img"
                aria-label="Completed"
                class="grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-ok text-ok"
              >
                <Check size={12} strokeWidth={3} />
              </span>
              <span class="task-title-3 flex-1 text-muted line-through" title={r.title}>{r.title}</span>
            {:else}
              <button
                type="button"
                aria-label="Complete {r.title}"
                onclick={() => complete(r.id)}
                class="grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
              >
                <Check size={12} strokeWidth={3} />
              </button>
              <span class="task-title-3 flex-1" title={r.title}>{r.title}</span>
              {#if r.recurrence}
                <span class="text-xs text-muted" aria-label="Repeats {r.recurrence.rule}">↻</span>
              {/if}
            {/if}
            {#if r.due}
              <span class="text-xs {r.due.date < today() ? 'text-danger' : 'text-muted'}">{dueText(r.due)}</span>
            {/if}
          {:else if href(r)}
            <a class="min-w-0 flex-1 truncate" href={href(r)}>{title(r)}</a>
          {:else}
            <span class="min-w-0 flex-1 truncate">{title(r)}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/each}
{/if}

<TaskDrawer taskId={openId} onclose={closeDrawer} />
