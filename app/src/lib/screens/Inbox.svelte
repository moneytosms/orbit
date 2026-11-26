<script lang="ts">
  import {
    groupItems,
    groupOptions,
    isCustom,
    loadDisplay,
    saveDisplay,
    sortOptions,
    type Display,
  } from '$lib/listDisplay'
  import { tick, untrack } from 'svelte'
  import Check from '@lucide/svelte/icons/check'
  import Inbox from '@lucide/svelte/icons/inbox'
  import { dndzone, TRIGGERS, type DndEvent } from 'svelte-dnd-action'
  import Flag from '@lucide/svelte/icons/flag'
  import { Button } from '$lib/components/ui/button'
  import EntityMenu from '$lib/EntityMenu.svelte'
  import Board, { cellText, type BoardRow, type CellCtx } from '$lib/Board.svelte'
  import Table from '$lib/Table.svelte'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import Composer from '$lib/Composer.svelte'
  import OverdueBanner from '$lib/OverdueBanner.svelte'
  import { planMoves } from '$lib/overdue'
  import { applyMoves, projectOccurrences } from '$lib/postpone'
  import {
    completeTask,
    createSection,
    createPage,
    labels,
    moveItem,
    orderBetween,
    nameIndex,
    pagesOf,
    sectionsOf,
    tasksIn,
    updateSection,
    type Due,
    type Sort,
    type Page,
    type Task,
    type Section,
  } from '$lib/sync'
  import { inbox_id } from '../../../wasm/orbit_wasm'

  type Entry = { task: Task; depth: number }
  type Group = { id: string; task: Task; subs: Entry[] }

  let {
    parentId,
    title,
    rows,
    onchange,
    layout: forced,
    bare = false,
  }: {
    parentId: string
    title: string
    rows: Entry[]
    onchange: () => void
    /** An Area or Project page picks List, Board or Table in its own tabs and passes it here. */
    layout?: 'list' | 'board' | 'table'
    /** No title and no layout pills: the page around the list has its own header. */
    bare?: boolean
  } = $props()
  let openId = $state<string | null>(null)
  let names = $state(new Map<string, string>())
  let pageList = $state<Page[]>([])

  // List | Board | Table, remembered per list on this device.
  const layoutKey = (id: string) => `orbit.layout.${id}`
  const readLayout = (id: string) => {
    try {
      const v = localStorage.getItem(layoutKey(id))
      return v === 'board' || v === 'table' ? v : 'list'
    } catch {
      return 'list'
    }
  }
  let layout = $state<string>('list')
  let layoutFor = ''
  $effect(() => {
    if (layoutFor !== parentId) {
      layoutFor = parentId
      layout = readLayout(parentId)
    }
    if (forced) layout = forced
  })
  function setLayout(next: string) {
    layout = next
    try {
      localStorage.setItem(layoutKey(parentId), next)
    } catch {
      // Storage unavailable: the choice only lasts this session.
    }
  }
  let boardGroups = $state<{ key: string; rows: BoardRow[] }[]>([])
  let sectionCols = $state<{ key: string; title: string }[]>([])
  let sectionList = $state<Section[]>([])
  let unsectionedItems = $state<Group[]>([])
  let sectionItems = $state(new Map<string, Group[]>())
  let sectionDraft = $state<string | null>(null)
  let sectionInput = $state<HTMLInputElement | undefined>()

  function groupsOf(entries: Entry[]): Group[] {
    const out: Group[] = []
    for (const r of entries) {
      if (r.depth === 0) out.push({ id: r.task.id, task: r.task, subs: [] })
      else out.at(-1)?.subs.push(r)
    }
    return out
  }

  $effect(() => {
    void rows
    const id = parentId
    sectionsOf(id).then(async (secs) => {
      const taskRows = await Promise.all(secs.map(async (section) => [section.id, await tasksIn(section.id)] as const))
      if (id !== parentId) return
      sectionList = secs
      sectionItems = new Map(taskRows.map(([sectionId, entries]) => [sectionId, groupsOf(entries)]))
      unsectionedItems = groupsOf(rows).filter((group) => group.task.parent_id === id)
    })
  })
  // The Board's columns: tasks directly under the list, then one column per Section.
  $effect(() => {
    void rows
    if (layout !== 'board') return
    const id = parentId
    sectionsOf(id).then(async (secs) => {
      const out = [{ key: '', rows: rows.filter((r) => r.depth === 0).map((r) => r.task) as BoardRow[] }]
      for (const sec of secs)
        out.push({ key: sec.id, rows: (await tasksIn(sec.id)).filter((r) => r.depth === 0).map((r) => r.task) })
      if (id !== parentId) return
      sectionCols = [{ key: '', title: 'No section' }, ...secs.map((x) => ({ key: x.id, title: x.name }))]
      boardGroups = out
    })
  })
  let display = $state<Display>(untrack(() => loadDisplay(parentId)))
  $effect(() => {
    display = loadDisplay(parentId)
  })
  function setDisplay(patch: Partial<Display>) {
    display = { ...display, ...patch }
    saveDisplay(parentId, display)
  }

  let sorts = $state<Sort[]>([])
  const sortValue = (t: Task, f: string, cx: CellCtx): string | number => {
    if (f === 'title') return t.title.toLowerCase()
    if (f === 'priority') return t.priority
    if (f === 'due') return t.due?.date ?? '\uffff'
    if (f === 'deadline') return t.deadline ?? '\uffff'
    if (f === 'created_at') return t.created_at ?? 0
    return cellText(t, f.replace('.name', ''), cx).toLowerCase()
  }
  let tableCx = $state<CellCtx>({ labels: new Map(), names: new Map() })
  $effect(() => {
    void rows
    Promise.all([labels(), nameIndex()]).then(
      ([l, n]) => (tableCx = { labels: new Map(l.map((x) => [x.id, x])), names: n }),
    )
  })
  const tableRows = $derived.by(() => {
    const out = rows.map((r) => r.task)
    return out.sort((a, b) => {
      for (const s of sorts) {
        const x = sortValue(a, s.field, tableCx)
        const y = sortValue(b, s.field, tableCx)
        if (x !== y) return (x < y ? -1 : 1) * (s.desc ? -1 : 1)
      }
      return 0
    })
  })
  function openRow(r: BoardRow) {
    if (r.kind === 'task') openId = r.id
  }

  // Pages of an Area or Project; the Inbox has none.
  $effect(() => {
    void rows
    if (parentId === inbox_id()) pageList = []
    else pagesOf(parentId).then((p) => (pageList = p))
  })
  async function newPage() {
    const id = await createPage(parentId, 'Untitled')
    location.hash = `#/page/${id}`
  }

  // Label names for the chips; reloaded whenever the rows change.
  $effect(() => {
    void rows
    labels().then((all) => (names = new Map(all.map((l) => [l.id, l.name]))))
  })

  const flagClass: Record<number, string> = { 1: 'text-danger', 2: 'text-warn', 3: 'text-accent' }
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
  function closeDrawer() {
    openId = null
    onchange()
  }
  // The row's one-click reschedule for an overdue Task (a recurring one goes to its next occurrence from today).
  async function rescheduleRow(task: Task) {
    await applyMoves(planMoves([task], today(), projectOccurrences), `"${task.title}" moved to today`)
    onchange()
  }

  // Top-level Tasks are the draggable units; each carries its open subtasks.
  const groups = $derived.by(() => {
    const out: Group[] = []
    for (const r of rows) {
      if (r.depth === 0) out.push({ id: r.task.id, task: r.task, subs: [] })
      else out.at(-1)?.subs.push(r)
    }
    return out
  })
  // Grouped or re-sorted lists are flat views of the Tasks; the manual list keeps sections and dragging.
  const displaySections = $derived(groupItems(groups, display, today(), names))
  function taskConsider(target: string, e: CustomEvent<DndEvent<Group>>) {
    if (target) sectionItems = new Map(sectionItems).set(target, e.detail.items)
    else unsectionedItems = e.detail.items
  }
  async function taskFinalize(target: string, e: CustomEvent<DndEvent<Group>>) {
    if (target) sectionItems = new Map(sectionItems).set(target, e.detail.items)
    else unsectionedItems = e.detail.items
    const { id, trigger } = e.detail.info
    if (trigger !== TRIGGERS.DROPPED_INTO_ZONE) return
    const i = e.detail.items.findIndex((g) => g.id === id)
    await moveItem(
      id,
      target || parentId,
      orderBetween(e.detail.items[i - 1]?.task.order, e.detail.items[i + 1]?.task.order),
    )
    onchange()
  }
  function sectionConsider(e: CustomEvent<DndEvent<Section>>) {
    sectionList = e.detail.items
  }
  async function sectionFinalize(e: CustomEvent<DndEvent<Section>>) {
    sectionList = e.detail.items
    const { id, trigger } = e.detail.info
    if (trigger !== TRIGGERS.DROPPED_INTO_ZONE) return
    const i = sectionList.findIndex((section) => section.id === id)
    await moveItem(id, parentId, orderBetween(sectionList[i - 1]?.order, sectionList[i + 1]?.order))
    onchange()
  }
  async function createSectionFromRow(e: SubmitEvent) {
    e.preventDefault()
    const name = sectionDraft?.trim() ?? ''
    if (!name) return
    sectionDraft = null
    await createSection(parentId, name)
    onchange()
  }
  async function startSectionCreate() {
    sectionDraft = ''
    await tick()
    sectionInput?.focus()
  }
  async function renameSection(section: Section, name: string) {
    const next = name.trim()
    if (next && next !== section.name) await updateSection(section.id, { name: next })
    onchange()
  }
</script>

{#snippet line(task: Task, depth: number)}
  <div
    data-task-row={task.id}
    class="group relative flex min-h-[2.625rem] min-w-0 items-center gap-3 rounded-[9px] pr-2 hover:bg-hover data-[cursor=true]:bg-hover data-[cursor=true]:ring-2 data-[cursor=true]:ring-accent"
    style:padding-left="{8 + depth * 24}px"
  >
    <button
      type="button"
      aria-label="Complete {task.title}"
      onclick={() => completeTask(task.id)}
      class="peer grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
    >
      <Check size={12} strokeWidth={3} />
    </button>
    <button
      type="button"
      data-open
      onclick={() => (openId = task.id)}
      class="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-0.5 py-2 text-left"
    >
      <span class="task-title-3 flex-1" title={task.title}>{task.title}</span>
      {#if task.priority <= 3}
        <Flag size={12} class={flagClass[task.priority]} aria-label="Priority {task.priority}" />
      {/if}
      {#if task.due}
        <span class="text-xs {task.due.date < today() ? 'font-medium text-danger-text' : 'text-muted'}"
          >{dueText(task.due)}</span
        >
      {/if}
      {#if task.recurrence}
        <span class="text-xs text-muted" aria-label="Repeats {task.recurrence.rule}">↻</span>
      {/if}
      {#each task.label_ids as id (id)}
        {#if names.has(id)}
          <span class="rounded-full border border-line px-2 py-px text-xs text-muted">{names.get(id)}</span>
        {/if}
      {/each}
    </button>
    {#if task.due && task.due.date < today()}
      <button
        type="button"
        aria-label="Reschedule {task.title} to today"
        title="Reschedule to today"
        onclick={() => rescheduleRow(task)}
        class="mr-1 shrink-0 rounded-md border border-line bg-surface px-2 py-0.5 text-xs font-medium text-text shadow-1 opacity-0 hover:border-accent-line hover:text-accent-text focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-accent pointer-coarse:opacity-100 group-focus-within:opacity-100 group-hover:opacity-100"
        >Reschedule</button
      >
    {/if}
    <span class="absolute right-2 bottom-0 h-px bg-line group-hover:hidden" style:left="{36 + depth * 24}px"></span>
  </div>
{/snippet}

{#if !bare}
  <div class="mb-2 flex items-center gap-2">
    <h1 class="page-title flex-1">{title}</h1>
  </div>
{/if}
<OverdueBanner tasks={rows.map((r) => r.task)} {onchange} />
<div class="mb-4">
  <Composer {parentId} autofocus={false} />
</div>

{#if !bare}
  <div role="group" aria-label="Layout" class="mb-3 flex gap-1">
    {#each [['list', 'List'], ['board', 'Board'], ['table', 'Table']] as [value, text] (value)}
      <Button
        size="sm"
        variant={layout === value ? 'on' : 'default'}
        aria-pressed={layout === value}
        onclick={() => setLayout(value)}>{text}</Button
      >
    {/each}
  </div>
{/if}

{#if layout === 'list'}
  <div role="group" aria-label="Display" class="mb-3 flex flex-wrap items-center gap-2 text-sm text-muted">
    <label class="flex items-center gap-1">
      Group
      <select
        aria-label="Group by"
        value={display.group}
        onchange={(e) => setDisplay({ group: e.currentTarget.value as Display['group'] })}
        class="h-8 rounded-md border border-line bg-surface px-2 text-text"
      >
        {#each groupOptions as [value, text] (value)}<option {value}>{text}</option>{/each}
      </select>
    </label>
    <label class="flex items-center gap-1">
      Sort
      <select
        aria-label="Sort by"
        value={display.sort}
        onchange={(e) => setDisplay({ sort: e.currentTarget.value as Display['sort'] })}
        class="h-8 rounded-md border border-line bg-surface px-2 text-text"
      >
        {#each sortOptions as [value, text] (value)}<option {value}>{text}</option>{/each}
      </select>
    </label>
  </div>
{/if}

{#if layout === 'board'}
  <Board
    groupBy="section"
    groups={boardGroups}
    fields={[]}
    extra={sectionCols}
    scopeId={parentId}
    sectionsEditable={parentId !== inbox_id()}
    {onchange}
    onopen={openRow}
  />
{:else if layout === 'table'}
  <Table rows={tableRows as BoardRow[]} fields={[]} {sorts} onsort={(s) => (sorts = s)} onopen={openRow} />
{:else if isCustom(display)}
  {#if !groups.length}
    <div class="flex flex-col items-center gap-2 px-3 py-6 text-center text-muted">
      <Inbox size={20} class="text-icon" />
      <p>Nothing here. Type a task above and press Enter.</p>
    </div>
  {/if}
  {#each displaySections as section (section.key)}
    <section class="mt-2" aria-label={section.title || 'Tasks'}>
      {#if section.title}
        <h2 class="mb-1 px-2 py-1 text-sm font-semibold text-muted">{section.title} · {section.items.length}</h2>
      {/if}
      <ul>
        {#each section.items as g (g.id)}
          <li>
            {@render line(g.task, 0)}
            {#each g.subs as s (s.task.id)}{@render line(s.task, s.depth)}{/each}
          </li>
        {/each}
      </ul>
    </section>
  {/each}
{:else}
  {#if !groups.length && !sectionList.length}
    <div class="flex flex-col items-center gap-2 px-3 py-6 text-center text-muted">
      <Inbox size={20} class="text-icon" />
      <p>Nothing here. Type a task above and press Enter.</p>
    </div>
  {/if}
  <ul
    use:dndzone={{
      items: unsectionedItems,
      type: 'task',
      flipDurationMs: 150,
      delayTouchStart: 200,
      useCursorForDetection: true,
    }}
    onconsider={(e) => taskConsider('', e)}
    onfinalize={(e) => taskFinalize('', e)}
  >
    {#each unsectionedItems as g (g.id)}
      <li>
        {@render line(g.task, 0)}
        {#each g.subs as s (s.task.id)}{@render line(s.task, s.depth)}{/each}
      </li>
    {/each}
  </ul>
  <ul
    class="mt-4"
    use:dndzone={{
      items: sectionList,
      type: 'section',
      flipDurationMs: 150,
      delayTouchStart: 200,
      useCursorForDetection: true,
    }}
    onconsider={sectionConsider}
    onfinalize={sectionFinalize}
  >
    {#each sectionList as section (section.id)}
      <li>
        <section class="mt-2" aria-label="{section.name} section">
          <div class="mb-1 flex items-center gap-1">
            <input
              value={section.name}
              aria-label="Rename section {section.name}"
              onblur={(e) => renameSection(section, e.currentTarget.value)}
              onkeydown={(e) => {
                if (e.key === 'Enter') e.currentTarget.blur()
                if (e.key === 'Escape') {
                  e.currentTarget.value = section.name
                  e.currentTarget.blur()
                }
              }}
              class="min-w-0 flex-1 bg-transparent px-2 py-1 text-sm font-semibold text-muted outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
            <EntityMenu id={section.id} name={section.name} kind="section" fallbackHref="#/inbox" {onchange} />
          </div>
          <ul
            class="min-h-[3rem]"
            use:dndzone={{
              items: sectionItems.get(section.id) ?? [],
              type: 'task',
              flipDurationMs: 150,
              delayTouchStart: 200,
              useCursorForDetection: true,
            }}
            onconsider={(e) => taskConsider(section.id, e)}
            onfinalize={(e) => taskFinalize(section.id, e)}
          >
            {#each sectionItems.get(section.id) ?? [] as g (g.id)}
              <li>
                {@render line(g.task, 0)}
                {#each g.subs as s (s.task.id)}{@render line(s.task, s.depth)}{/each}
              </li>
            {/each}
          </ul>
        </section>
      </li>
    {/each}
  </ul>
  <div class="mt-3">
    {#if sectionDraft !== null}
      <form onsubmit={createSectionFromRow}>
        <input
          bind:this={sectionInput}
          bind:value={sectionDraft}
          aria-label="New section name"
          placeholder="Section name"
          onkeydown={(e) => e.key === 'Escape' && (sectionDraft = null)}
          class="h-9 w-full rounded-md border border-line bg-surface px-2.5 text-sm outline-none focus-visible:border-accent-line"
        />
      </form>
    {:else}
      <button type="button" class="px-2 py-2 text-sm text-muted hover:text-text" onclick={startSectionCreate}>
        + New section
      </button>
    {/if}
  </div>
{/if}

{#if parentId !== inbox_id()}
  <section class="mt-6" aria-label="Pages">
    <div class="mb-1 flex items-center gap-2">
      <h2 class="panel-title flex-1">Pages</h2>
      <Button variant="ghost" onclick={newPage}>New page</Button>
    </div>
    <ul>
      {#each pageList as p (p.id)}
        <li><a href="#/page/{p.id}" class="block rounded-[9px] px-2 py-2 hover:bg-hover">{p.title}</a></li>
      {/each}
    </ul>
  </section>
{/if}

<TaskDrawer taskId={openId} onclose={closeDrawer} />
