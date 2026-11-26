<script lang="ts">
  import { untrack, type Snippet } from 'svelte'
  import Backlinks from '$lib/Backlinks.svelte'
  import Banner from '$lib/Banner.svelte'
  import EntityCreateMenu from '$lib/EntityCreateMenu.svelte'
  import LayoutBar from '$lib/LayoutBar.svelte'
  import LayoutGrid from '$lib/LayoutGrid.svelte'
  import PageHeader from '$lib/PageHeader.svelte'
  import { defaultPageLayout, pageKinds, pageTitleOf, place, type PageKind, type Widget } from '$lib/dashboard'
  import { getPageLayout, setPageLayout, updateArea, updateProject, type tree } from '$lib/sync'
  import GoalProgress from '$lib/widgets/GoalProgress.svelte'
  import NextList from '$lib/widgets/NextList.svelte'
  import PagePages from '$lib/widgets/PagePages.svelte'
  import PageProjects from '$lib/widgets/PageProjects.svelte'
  import PageRoadmaps from '$lib/widgets/PageRoadmaps.svelte'
  import PageStats from '$lib/widgets/PageStats.svelte'
  import PageTasks from '$lib/widgets/PageTasks.svelte'
  import LabelChips from '$lib/widgets/LabelChips.svelte'
  import QuickAddTile from '$lib/widgets/QuickAddTile.svelte'
  import TaskFeed from '$lib/widgets/TaskFeed.svelte'
  import WeekProgress from '$lib/widgets/WeekProgress.svelte'
  import RoutineTicks from '$lib/widgets/RoutineTicks.svelte'

  /** The page of an Area or a Project: header, view tabs, then the Overview widgets or the task list. */
  let {
    id,
    kind = 'area',
    areas,
    onchange,
    tasks,
    onnewtask,
  }: {
    id: string
    kind?: PageKind
    areas: Awaited<ReturnType<typeof tree>>
    onchange: () => void
    /** The task list for the List, Board and Table tabs; it is told which one to show. */
    tasks: Snippet<['list' | 'board' | 'table']>
    onnewtask: (prefill: string) => void
  } = $props()

  const areaRow = $derived(kind === 'area' ? areas.find((a) => a.area.id === id) : undefined)
  const projectRow = $derived(
    kind === 'project'
      ? (() => {
          const a = areas.find((a) => a.projects.some((p) => p.id === id))
          return a ? { area: a.area, project: a.projects.find((p) => p.id === id)! } : undefined
        })()
      : undefined,
  )
  const item = $derived(areaRow?.area ?? projectRow?.project)
  const projects = $derived(areaRow?.projects ?? [])

  type Tab = 'overview' | 'list' | 'board' | 'table'
  const TABS: { value: Tab; label: string }[] = [
    { value: 'overview', label: 'Overview' },
    { value: 'list', label: 'List' },
    { value: 'board', label: 'Board' },
    { value: 'table', label: 'Table' },
  ]
  // Remembered per page on this device; List, Board and Table share the key the list itself used before.
  const tabKey = (pid: string) => `orbit.layout.${pid}`
  const readTab = (pid: string): Tab => {
    try {
      const v = localStorage.getItem(tabKey(pid))
      return v === 'list' || v === 'board' || v === 'table' || v === 'overview' ? v : 'overview'
    } catch {
      return 'overview'
    }
  }
  let tab = $state<Tab>('overview')
  let tabFor = ''
  $effect(() => {
    if (tabFor !== id) {
      tabFor = id
      tab = readTab(id)
    }
  })
  function setTab(next: string) {
    tab = next as Tab
    try {
      localStorage.setItem(tabKey(id), next)
    } catch {
      // Storage unavailable: the choice only lasts this session.
    }
  }

  // Tiles reload whenever the shell refreshes (`areas` is a fresh array after every sync).
  let version = $state(0)
  $effect(() => {
    void areas
    // `version++` reads as well as writes, so it must not count as a dependency of this effect.
    untrack(() => version++)
  })

  let layout = $state<Widget[]>(defaultPageLayout('area'))
  let editing = $state(false)
  let summaries = $state<Record<string, string>>({})
  $effect(() => {
    void version
    const pid = id
    getPageLayout(pid, kind).then((w) => {
      if (pid === id) layout = w as Widget[]
    })
  })
  $effect(() => {
    void id
    editing = false
  })

  const save = (next: Widget[]) => {
    layout = next
    void setPageLayout(id, next)
  }
  const offered = $derived(pageKinds(kind))
  const hidden = $derived(layout.filter((w) => w.hidden && offered.some((k) => k.kind === w.kind)))
  function addKind(k: string) {
    const spec = offered.find((o) => o.kind === k)!
    const existing = layout.find((w) => w.kind === k)
    save(place(layout, existing ?? { id: k, kind: k, x: 0, y: 0, w: spec.w, h: spec.h, hidden: false }))
  }
  function reset() {
    layout = defaultPageLayout(kind)
    void setPageLayout(id, null)
  }
  function startEditing() {
    setTab('overview')
    editing = true
  }
  const setSummary = (wid: string, value: string) => (summaries = { ...summaries, [wid]: value })

  const crumbs = $derived(projectRow ? [{ label: projectRow.area.name, href: `#/area/${projectRow.area.id}` }] : [])
  const bannerSave = async (patch: { icon?: string; cover?: string }) => {
    if (kind === 'area') await updateArea(id, patch)
    else await updateProject(id, patch)
    onchange()
  }
</script>

{#if item}
  <Banner icon={item.icon} cover={item.cover} onsave={bannerSave} />
  <PageHeader
    title={item.name}
    {crumbs}
    newTaskText="@{item.name}"
    entity={{
      id,
      kind,
      name: item.name,
      fallbackHref: projectRow ? `#/area/${projectRow.area.id}` : '#/home',
      archiveable: true,
    }}
    {onchange}
    onedit={startEditing}
    onreset={reset}
    tabs={TABS}
    {tab}
    ontab={setTab}
  >
    {#snippet actions()}
      <EntityCreateMenu ownerId={id} ownerName={item.name} ownerKind={kind} {areas} {onchange} {onnewtask} />
    {/snippet}
  </PageHeader>

  {#if tab === 'overview'}
    {#if editing}
      <LayoutBar
        kinds={offered}
        hidden={hidden.map((w) => ({ id: w.id, title: pageTitleOf(w.kind) }))}
        onadd={addKind}
        onshow={(wid) => {
          const w = layout.find((o) => o.id === wid)
          if (w) save(place(layout, w))
        }}
        onreset={reset}
        ondone={() => (editing = false)}
      />
    {/if}
    <LayoutGrid
      widgets={layout.filter((w) => offered.some((k) => k.kind === w.kind))}
      {editing}
      label={(w) => pageTitleOf(w.kind)}
      onchange={(next) => save([...next, ...layout.filter((w) => !next.some((n) => n.id === w.id))])}
    >
      {#snippet renderTile(w)}
        {#if w.kind === 'stats'}
          <PageStats parentId={id} {version} />
        {:else}
          <header class="mb-2 min-w-0">
            <h2 class="truncate text-[0.9062rem] font-semibold">{pageTitleOf(w.kind)}</h2>
            <p class="truncate text-xs text-muted">{summaries[w.id] ?? 'Loading…'}</p>
          </header>
          <div class="min-h-0 flex-1 overflow-auto">
            {#if w.kind === 'tasks'}
              <PageTasks parentId={id} name={item.name} {version} {onchange} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'next'}
              <NextList parentId={id} {version} {onchange} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'goals'}
              <GoalProgress parentId={id} {version} {onchange} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'routines'}
              <RoutineTicks parentId={id} {version} {onchange} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'pages'}
              <PagePages parentId={id} {version} {onchange} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'projects'}
              <PageProjects {projects} {version} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'roadmaps'}
              <PageRoadmaps parentId={id} {version} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'today' || w.kind === 'overdue' || w.kind === 'done'}
              <TaskFeed mode={w.kind} parentId={id} {version} {onchange} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'progress'}
              <WeekProgress parentId={id} {version} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'quickadd'}
              <QuickAddTile parentId={id} {onchange} onmeta={(v) => setSummary(w.id, v)} />
            {:else if w.kind === 'labels'}
              <LabelChips {version} onmeta={(v) => setSummary(w.id, v)} />
            {/if}
          </div>
        {/if}
      {/snippet}
    </LayoutGrid>
  {:else}
    {@render tasks(tab)}
  {/if}

  <div class="mt-4"><Backlinks {id} /></div>
{/if}
