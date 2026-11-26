<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Plus from '@lucide/svelte/icons/plus'
  import Target from '@lucide/svelte/icons/target'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import RoadmapBuilder from '$lib/RoadmapBuilder.svelte'
  import EntityMenu from '$lib/EntityMenu.svelte'
  import {
    addEntry,
    completeGoal,
    createGoal,
    createMilestone,
    createPage,
    goalProgress,
    goals,
    milestonesOf,
    pagesOf,
    setMilestoneDone,
    type Goal,
    type Milestone,
    type Page,
    type Progress,
    type tree,
  } from '$lib/sync'

  let { areas }: { areas: Awaited<ReturnType<typeof tree>> } = $props()

  type View = { goal: Goal; pct: number; behind: boolean; milestones: Milestone[]; pages: Page[] }
  let views = $state(new Map<string, View>())
  let list = $state<Goal[]>([])
  let building = $state<Goal | null>(null)
  let buildOpen = $state(false)

  // Reloaded whenever the shell refreshes (`areas` is a fresh array after every sync).
  $effect(() => {
    void areas
    load()
  })

  async function load() {
    const active = await goals()
    const next = new Map<string, View>()
    for (const goal of active) {
      const { progress, behind } = await goalProgress(goal.id)
      next.set(goal.id, {
        goal,
        pct: Math.round(progress * 100),
        behind,
        milestones: await milestonesOf(goal.id),
        pages: await pagesOf(goal.id),
      })
    }
    views = next
    list = active
  }

  const topLevel = (areaId: string) => list.filter((g) => g.parent_id === areaId)
  const subsOf = (goalId: string) => list.filter((g) => g.parent_id === goalId)
  const dateText = (d: string) => {
    const [y, m, day] = d.split('-').map(Number)
    return new Date(y, m - 1, day).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })
  }

  let name = $state('')
  let areaId = $state('')
  let kind = $state<'count' | 'target' | 'manual'>('count')
  let target = $state(10)
  let start = $state('')
  let targetDate = $state('')
  let parentId = $derived(areaId || areas[0]?.area.id || '')

  async function add(e: SubmitEvent) {
    e.preventDefault()
    const title = name.trim()
    if (!title || !parentId) return
    const progress: Progress =
      kind === 'target' ? { kind: 'target', target: Number(target) || 1, entries: [] } : { kind }
    await createGoal(parentId, title, {
      progress,
      ...(start ? { start } : {}),
      ...(targetDate ? { target_date: targetDate } : {}),
    })
    name = ''
    start = ''
    targetDate = ''
    await load()
  }

  const inputClass =
    'h-[2.125rem] rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line'
  let amounts = $state<Record<string, number>>({})
  let newMilestone = $state<Record<string, string>>({})

  async function addAmount(id: string) {
    const n = Number(amounts[id])
    if (!Number.isFinite(n) || n === 0) return
    amounts[id] = 0
    await addEntry(id, n)
    await load()
  }
  async function addSubGoal(goalId: string) {
    await createGoal(goalId, 'New sub-goal')
    await load()
  }
  async function newPage(goalId: string) {
    const id = await createPage(goalId, 'Untitled')
    location.hash = `#/page/${id}`
  }
  async function addMilestone(e: SubmitEvent, id: string) {
    e.preventDefault()
    const title = (newMilestone[id] ?? '').trim()
    if (!title) return
    newMilestone[id] = ''
    await createMilestone(id, title)
    await load()
  }
</script>

{#snippet card(goal: Goal, sub: boolean)}
  {@const v = views.get(goal.id)}
  {#if v}
    <article class="rounded-2xl border border-line bg-surface p-4 {sub ? 'ml-6 mt-2' : ''}">
      <div class="flex flex-wrap items-center gap-2">
        <h3 class="flex-1 text-[0.9375rem] font-medium">{goal.name}</h3>
        <EntityMenu id={goal.id} name={goal.name} kind="goal" fallbackHref="#/goals" archiveable onchange={load} />
        {#if v.behind}
          <span class="rounded-full bg-warn-soft px-2 py-px text-xs text-warn-text">Behind pace</span>
        {/if}
        {#if goal.target_date}
          <span class="text-xs text-muted">by {dateText(goal.target_date)}</span>
        {/if}
        <Button variant="default" size="sm" onclick={() => completeGoal(goal.id).then(load)}>Complete</Button>
        {#if !sub}
          <Button
            variant="ghost"
            size="sm"
            aria-label="Add sub-goal to {goal.name}"
            onclick={() => addSubGoal(goal.id)}
          >
            <Plus />
            Add sub-goal
          </Button>
        {/if}
        <Button
          variant="ghost"
          size="sm"
          aria-label="New roadmap for {goal.name}"
          onclick={() => {
            building = goal
            buildOpen = true
          }}
        >
          New roadmap
        </Button>
      </div>
      {#if goal.done_condition}
        <p class="mt-1 text-sm text-muted">{goal.done_condition}</p>
      {/if}
      <div
        role="progressbar"
        aria-label="{goal.name} progress"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={v.pct}
        class="mt-3 h-2 overflow-hidden rounded-full bg-sunk"
      >
        <div class="h-full rounded-full bg-accent" style="width: {v.pct}%"></div>
      </div>
      <p class="mt-1 text-xs text-muted">{v.pct}%</p>

      {#if goal.progress.kind === 'target'}
        <form
          class="mt-2 flex items-center gap-2"
          onsubmit={(e) => {
            e.preventDefault()
            addAmount(goal.id)
          }}
        >
          <input
            type="number"
            step="any"
            aria-label="Amount for {goal.name}"
            bind:value={amounts[goal.id]}
            class="{inputClass} w-28"
          />
          <Button type="submit" variant="default" size="sm">Add entry</Button>
        </form>
      {/if}

      <ul class="mt-2 grid gap-1">
        {#each v.milestones as m (m.id)}
          <li class="flex items-center gap-2 text-sm">
            <Checkbox
              checked={m.done}
              aria-label={m.title}
              onCheckedChange={(c) => setMilestoneDone(m.id, c === true).then(load)}
            />
            <span class={m.done ? 'text-muted line-through' : ''}>{m.title}</span>
            {#if m.date}<span class="text-xs text-muted">{dateText(m.date)}</span>{/if}
          </li>
        {/each}
      </ul>
      <form class="mt-2" onsubmit={(e) => addMilestone(e, goal.id)}>
        <Input
          bind:value={newMilestone[goal.id]}
          placeholder="Add a milestone"
          aria-label="New milestone for {goal.name}"
        />
      </form>
      <div class="mt-3 flex items-center gap-2">
        <h4 class="flex-1 text-xs font-medium text-muted">Pages</h4>
        <Button variant="ghost" size="sm" onclick={() => newPage(goal.id)}>New page</Button>
      </div>
      <ul class="grid gap-0.5">
        {#each v.pages as p (p.id)}
          <li><a href="#/page/{p.id}" class="text-sm underline">{p.title}</a></li>
        {/each}
      </ul>
    </article>
    {#each subsOf(goal.id) as s (s.id)}
      {@render card(s, true)}
    {/each}
  {/if}
{/snippet}

<h1 class="page-title">Goals</h1>
<div class="flex flex-col gap-4">
  <section class="panel">
    <h2 class="panel-title">New goal</h2>
    <form class="grid gap-2 sm:grid-cols-2" onsubmit={add}>
      <Input bind:value={name} placeholder="Goal name" aria-label="Goal name" class="sm:col-span-2" />
      <select bind:value={areaId} aria-label="Area" class="{inputClass} w-full">
        {#each areas as a (a.area.id)}
          <option value={a.area.id}>{a.area.name}</option>
        {/each}
      </select>
      <select bind:value={kind} aria-label="Progress" class="{inputClass} w-full">
        <option value="count">Count</option>
        <option value="target">Target</option>
        <option value="manual">Manual</option>
      </select>
      {#if kind === 'target'}
        <input
          type="number"
          min="0"
          step="any"
          bind:value={target}
          aria-label="Target amount"
          class="{inputClass} w-full"
        />
      {/if}
      <label class="grid gap-1 text-xs text-muted">
        Start
        <input type="date" bind:value={start} class="{inputClass} w-full" />
      </label>
      <label class="grid gap-1 text-xs text-muted">
        Target date
        <input type="date" bind:value={targetDate} class="{inputClass} w-full" />
      </label>
      <Button type="submit" class="sm:col-span-2" disabled={!name.trim() || !parentId}>Add goal</Button>
    </form>
  </section>

  {#each areas as a (a.area.id)}
    {@const top = topLevel(a.area.id)}
    {#if top.length}
      <section class="grid gap-2">
        <h2 class="panel-title">{a.area.name}</h2>
        {#each top as g (g.id)}
          {@render card(g, false)}
        {/each}
      </section>
    {/if}
  {:else}
    <Empty
      icon={Target}
      text="Goals track progress toward something you want. Create an Area first, then add goals to it."
    />
  {/each}
</div>

<RoadmapBuilder
  bind:open={buildOpen}
  onchange={load}
  {areas}
  ownerId={building?.id ?? ''}
  ownerName={building?.name ?? ''}
/>
