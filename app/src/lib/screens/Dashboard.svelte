<script lang="ts">
  import { untrack } from 'svelte'
  import { KINDS, defaultLayout, place, titleOf, withCalendar, type Widget } from '$lib/dashboard'
  import LayoutBar from '$lib/LayoutBar.svelte'
  import LayoutGrid from '$lib/LayoutGrid.svelte'
  import PageHeader from '$lib/PageHeader.svelte'
  import Checklist from '$lib/Checklist.svelte'
  import OverdueBanner from '$lib/OverdueBanner.svelte'
  import { checklistDismissed, dismissChecklist, showChecklist } from '$lib/onboarding'
  import { getDashboard, setDashboard, tree, views, visibleSnapshot, type Task, type View } from '$lib/sync'
  import Glance from '$lib/widgets/Glance.svelte'
  import GoalProgress from '$lib/widgets/GoalProgress.svelte'
  import NextList from '$lib/widgets/NextList.svelte'
  import RoutineTicks from '$lib/widgets/RoutineTicks.svelte'
  import Streaks from '$lib/widgets/Streaks.svelte'
  import CalendarEvents from '$lib/widgets/CalendarEvents.svelte'
  import Upcoming from '$lib/widgets/Upcoming.svelte'
  import LabelChips from '$lib/widgets/LabelChips.svelte'
  import QuickAddTile from '$lib/widgets/QuickAddTile.svelte'
  import TaskFeed from '$lib/widgets/TaskFeed.svelte'
  import WeekProgress from '$lib/widgets/WeekProgress.svelte'
  import ViewList from '$lib/widgets/ViewList.svelte'

  let { version = 0, onchange }: { version?: number; onchange?: () => void } = $props()

  let layout = $state<Widget[]>([])
  let editing = $state(false)
  let viewChoices = $state<View[]>([])
  let summaries = $state<Record<string, string>>({})
  let calendarShown = $state(false)
  let openTasks = $state<Task[]>([])

  $effect(() => {
    void version
    getDashboard().then((w) => {
      if (saving === 0) layout = withCalendar(w as Widget[])
    })
  })
  // Open Tasks for the overdue banner; reloaded with the rest of the Home screen.
  const loadTasks = () =>
    visibleSnapshot().then((rows) => (openTasks = rows.filter((r): r is Task => r.kind === 'task')))
  $effect(() => {
    void version
    void loadTasks()
  })

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
  const setSummary = (id: string, value: string) => untrack(() => (summaries = { ...summaries, [id]: value }))

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

  // A re-read that lands while one of our own saves is still being written would put the old layout back.
  let saving = 0
  const save = () => {
    saving++
    void setDashboard(layout).finally(() => saving--)
  }

  function addKind(kind: string) {
    const k = KINDS.find((k) => k.kind === kind)!
    const existing = layout.find((w) => w.kind === kind)
    layout = place(layout, existing ?? { id: kind, kind, x: 0, y: 0, w: k.w, h: k.h, hidden: false })
    save()
  }
  function addView(view_id: string) {
    const existing = layout.find((w) => w.kind === 'view' && w.view_id === view_id)
    layout = place(
      layout,
      existing ?? { id: crypto.randomUUID(), kind: 'view', view_id, x: 0, y: 0, w: 6, h: 3, hidden: false },
    )
    save()
  }
  function hideTile(id: string) {
    layout = layout.map((w) => (w.id === id ? { ...w, hidden: true } : w))
    save()
  }
  function show(w: Widget) {
    layout = place(layout, w)
    save()
  }

  const change = (next: Widget[]) => {
    layout = next
    save()
  }
  const startEditing = () => (editing = true)
  function reset() {
    layout = defaultLayout()
    save()
  }
</script>

<PageHeader title="Home" onedit={startEditing} onreset={reset} />

{#if editing}
  <LayoutBar
    kinds={KINDS}
    widgets={layout}
    views={viewChoices}
    onadd={addKind}
    onaddview={addView}
    onhide={hideTile}
    onreset={reset}
    ondone={() => (editing = false)}
  />
{/if}

<OverdueBanner
  tasks={openTasks}
  onchange={() => {
    void loadTasks()
    onchange?.()
  }}
/>

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

<LayoutGrid
  widgets={layout}
  {editing}
  label={title}
  hideTile={(w) => w.kind === 'calendar' && !calendarShown}
  onchange={change}
>
  {#snippet renderTile(w)}
    <header class="mb-2 flex min-w-0 items-start gap-2">
      <div class="min-w-0 flex-1">
        <h2 class="truncate text-[0.9062rem] font-semibold">{title(w)}</h2>
        {#if w.kind !== 'glance'}<p class="truncate text-xs text-muted">{summaries[w.id] ?? 'Loading…'}</p>{/if}
      </div>
    </header>
    <div class="min-h-0 flex-1 overflow-auto">
      {#if w.kind === 'glance'}
        <Glance {version} />
      {:else if w.kind === 'next'}
        <NextList {version} {onchange} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'calendar'}
        <CalendarEvents {version} onshown={(v) => (calendarShown = v)} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'routines'}
        <RoutineTicks {version} {onchange} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'upcoming'}
        <Upcoming {version} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'goals'}
        <GoalProgress {version} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'streaks'}
        <Streaks {version} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'today' || w.kind === 'overdue' || w.kind === 'done'}
        <TaskFeed mode={w.kind} {version} {onchange} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'progress'}
        <WeekProgress {version} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'quickadd'}
        <QuickAddTile {onchange} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'labels'}
        <LabelChips {version} onmeta={(value) => setSummary(w.id, value)} />
      {:else if w.kind === 'view' && w.view_id}
        <ViewList viewId={w.view_id} {version} onmeta={(value) => setSummary(w.id, value)} />
      {/if}
    </div>
    {#if w.kind !== 'glance'}
      <a href={href(w)} class="tap-44 mt-2 self-start text-xs text-muted hover:text-text">Open →</a>
    {/if}
  {/snippet}
</LayoutGrid>
