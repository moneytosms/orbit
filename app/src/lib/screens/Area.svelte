<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import FileText from '@lucide/svelte/icons/file-text'
  import Empty from '$lib/Empty.svelte'
  import FolderKanban from '@lucide/svelte/icons/folder-kanban'
  import type { Snippet } from 'svelte'
  import EntityCreateMenu from '$lib/EntityCreateMenu.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import Banner from '$lib/Banner.svelte'
  import Backlinks from '$lib/Backlinks.svelte'
  import { areaStats, openCounts, roadmapsIn, upNextIn, type AreaStats } from '$lib/area'
  import {
    createGoal,
    createPage,
    goalProgress,
    localDay,
    pagesOf,
    updateArea,
    visibleSnapshot,
    whatsNext,
    type Goal,
    type NextItem,
    type Page,
    type Roadmap,
    type tree,
  } from '$lib/sync'

  let {
    id,
    areas,
    onchange,
    tasks,
    onnewtask,
  }: {
    id: string
    areas: Awaited<ReturnType<typeof tree>>
    onchange: () => void
    tasks: Snippet
    onnewtask: (prefill: string) => void
  } = $props()

  const area = $derived(areas.find((a) => a.area.id === id))
  const projects = $derived(area?.projects ?? [])

  let tab = $state<'tasks' | 'pages'>('tasks')
  let stats = $state<AreaStats>({ open: 0, completedThisWeek: 0, overdue: 0, activeGoals: 0 })
  let goalList = $state<{ goal: Goal; pct: number }[]>([])
  let next = $state<NextItem[]>([])
  let counts = $state(new Map<string, number>())
  let maps = $state<Roadmap[]>([])
  let pageList = $state<Page[]>([])

  // Reloaded whenever the shell refreshes (`areas` is a fresh array after every sync).
  $effect(() => {
    void areas
    void id
    load()
  })

  async function load() {
    const rows = await visibleSnapshot()
    const today = await localDay()
    stats = areaStats(rows, id, today, Date.now())
    const under = rows.filter((r): r is Goal => r.kind === 'goal' && r.completed_at === null && r.parent_id === id)
    goalList = await Promise.all(
      under
        .sort((a, b) => (a.order < b.order ? -1 : a.order > b.order ? 1 : 0))
        .map(async (goal) => ({ goal, pct: Math.round((await goalProgress(goal.id)).progress * 100) })),
    )
    next = upNextIn(await whatsNext(), rows, id, 5)
    counts = openCounts(
      rows,
      projects.map((p) => p.id),
    )
    maps = roadmapsIn(
      rows,
      rows.filter((r): r is Roadmap => r.kind === 'roadmap'),
      id,
    )
    pageList = await pagesOf(id)
  }

  async function rename(e: Event) {
    const input = e.currentTarget as HTMLInputElement
    const value = input.value.trim()
    if (!area) return
    if (!value) input.value = area.area.name
    else if (value !== area.area.name) {
      await updateArea(id, { name: value })
      onchange()
    }
  }

  let goalName = $state('')
  async function addGoal(e: SubmitEvent) {
    e.preventDefault()
    const name = goalName.trim()
    if (!name) return
    goalName = ''
    await createGoal(id, name)
    await load()
    onchange()
  }
  async function newPage() {
    const pid = await createPage(id, 'Untitled')
    onchange()
    location.hash = `#/page/${pid}`
  }

  const stat = (label: string, n: number) => ({ label, n })
  const tiles = $derived([
    stat('Open tasks', stats.open),
    stat('Done this week', stats.completedThisWeek),
    stat('Overdue', stats.overdue),
    stat('Active goals', stats.activeGoals),
  ])
  const tabClass = (on: boolean) =>
    `h-[2.125rem] rounded-md px-3 text-sm ${on ? 'bg-accent-soft text-accent-text' : 'text-muted hover:bg-hover'}`
</script>

{#if area}
  <Banner
    icon={area.area.icon}
    cover={area.area.cover}
    onsave={async (patch) => {
      await updateArea(id, patch)
      onchange()
    }}
  />
  <div class="flex items-center gap-1">
    <input
      aria-label="Area name"
      value={area.area.name}
      class="page-title min-w-0 flex-1 bg-transparent outline-none focus-visible:underline"
      onblur={rename}
      onkeydown={(e) => {
        if (e.key === 'Enter') e.currentTarget.blur()
      }}
    />
    <EntityCreateMenu ownerId={id} ownerName={area.area.name} ownerKind="area" {areas} {onchange} {onnewtask} />
  </div>

  <div class="flex flex-col gap-4">
    <ul class="grid grid-cols-2 gap-2 desk:grid-cols-4">
      {#each tiles as t (t.label)}
        <li class="rounded-2xl border border-line bg-surface p-3 shadow-1">
          <div class="text-xl font-semibold">{t.n}</div>
          <div class="text-xs text-muted">{t.label}</div>
        </li>
      {/each}
    </ul>

    <section class="panel">
      <h2 class="panel-title">Goals</h2>
      <ul class="grid gap-2">
        {#each goalList as g (g.goal.id)}
          <li>
            <div class="flex items-center gap-2 text-sm">
              <span class="flex-1">{g.goal.name}</span>
              <span class="text-xs text-muted">{g.pct}%</span>
            </div>
            <div
              role="progressbar"
              aria-label="{g.goal.name} progress"
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={g.pct}
              class="mt-1 h-2 overflow-hidden rounded-full bg-sunk"
            >
              <div class="h-full rounded-full bg-accent" style="width: {g.pct}%"></div>
            </div>
          </li>
        {/each}
      </ul>
      <form class="mt-3" onsubmit={addGoal}>
        <Input bind:value={goalName} placeholder="New goal, then Enter" aria-label="New goal" />
      </form>
    </section>

    <section class="panel">
      <h2 class="panel-title">Up next</h2>
      {#if next.length}
        <ul class="grid gap-1">
          {#each next as i (i.id)}
            <li class="flex items-center gap-2 text-sm">
              <span class="task-title-1 flex-1" title={i.title}>{i.title}</span>
              <span class="shrink-0 rounded-full bg-accent-soft px-2 py-px text-xs text-accent-text">{i.reason}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <Empty icon={CircleCheck} text="Nothing needs you here right now." />
      {/if}
    </section>

    <section class="panel">
      <h2 class="panel-title">Projects</h2>
      {#if projects.length}
        <ul class="grid gap-2 sm:grid-cols-2">
          {#each projects as p (p.id)}
            <li>
              <a
                href="#/project/{p.id}"
                class="flex items-center gap-2 rounded-xl border border-line bg-surface p-3 hover:bg-hover"
              >
                <span class="min-w-0 flex-1 truncate text-sm font-medium">{p.name}</span>
                <span class="text-xs text-muted">{counts.get(p.id) ?? 0} open</span>
              </a>
            </li>
          {/each}
        </ul>
      {:else}
        <Empty
          icon={FolderKanban}
          text="Projects group the tasks for one outcome in this Area. Create one from the title menu or sidebar."
        />
      {/if}
    </section>

    {#if maps.length}
      <section class="panel">
        <h2 class="panel-title">Roadmaps</h2>
        <ul class="grid gap-1">
          {#each maps as r (r.id)}
            <li><a href="#/roadmap/{r.id}" class="link text-sm">{r.name}</a></li>
          {/each}
        </ul>
      </section>
    {/if}

    <div>
      <div class="mb-3 flex gap-1" role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'tasks'}
          class={tabClass(tab === 'tasks')}
          onclick={() => (tab = 'tasks')}>Tasks</button
        >
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'pages'}
          class={tabClass(tab === 'pages')}
          onclick={() => (tab = 'pages')}>Pages</button
        >
      </div>
      {#if tab === 'tasks'}
        <div class="[&_h1]:sr-only">{@render tasks()}</div>
      {:else}
        <div class="mb-2 flex items-center">
          <h2 class="flex-1 text-sm font-medium text-muted">Pages</h2>
          <Button variant="ghost" size="sm" onclick={newPage}>New page</Button>
        </div>
        <ul class="grid gap-0.5">
          {#each pageList as p (p.id)}
            <li><a href="#/page/{p.id}" class="text-sm underline">{p.title}</a></li>
          {:else}
            <li>
              <Empty
                icon={FileText}
                text="Pages hold notes and docs for this Area."
                label="New page"
                onclick={newPage}
              />
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <Backlinks id={area.area.id} />
  </div>
{/if}
