<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Map from '@lucide/svelte/icons/map'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import * as Dialog from '$lib/components/ui/dialog'
  import MermaidImport from '$lib/MermaidImport.svelte'
  import RoadmapBuilder from '$lib/RoadmapBuilder.svelte'
  import Backlinks from '$lib/Backlinks.svelte'
  import EntityMenu from '$lib/EntityMenu.svelte'
import RoadmapGraphCanvas from '$lib/RoadmapGraphCanvas.svelte'
  import RoadmapTimeline from '$lib/RoadmapTimeline.svelte'
  import {
    createPhase,
    createRoadmap,
    getRoadmap,
    importMermaid,
    localDay,
    placeable,
    placeItem,
    roadmaps,
    roadmapSummary,
    setPhaseDone,
    type MermaidPlan,
    type PhaseSummary,
    type Roadmap,
    type tree,
  } from '$lib/sync'

  let { id, areas, onchange }: { id?: string; areas: Awaited<ReturnType<typeof tree>>; onchange: () => void } = $props()

  let list = $state<Roadmap[]>([])
  let roadmap = $state<Roadmap | undefined>()
  let phases = $state<PhaseSummary[]>([])
  let choices = $state<{ id: string; kind: string; title: string }[]>([])
  let mode = $state<'canvas' | 'list' | 'board' | 'timeline'>('canvas')
  let today = $state('')
  let building = $state(false)
  let importing = $state(false)
  let loaded = $state(false)
  let newName = $state('')
  let newPhase = $state('')
  let parentId = $state('')
  let dragging = $state<string | null>(null)
  let over = $state<string | null>(null)

  async function load() {
    if (id) {
      roadmap = await getRoadmap(id)
      today = await localDay()
      phases = roadmap ? await roadmapSummary(id, today) : []
      choices = roadmap ? await placeable() : []
    } else {
      list = await roadmaps()
      if (!parentId) parentId = areas[0]?.area.id ?? ''
    }
    loaded = true
  }
  $effect(() => {
    void id
    void areas
    load()
  })

  const placed = $derived(new Set(phases.flatMap((p) => p.items.map((i) => i.id))))
  const unplaced = $derived(choices.filter((c) => !placed.has(c.id)))
  const pct = (p: number) => `${Math.round(p * 100)}%`
  const span = (p: PhaseSummary) => [p.start, p.end].filter(Boolean).join(' – ')

  async function addRoadmap(e: SubmitEvent) {
    e.preventDefault()
    if (!newName.trim() || !parentId) return
    const made = await createRoadmap(parentId, newName.trim())
    newName = ''
    onchange()
    location.hash = `#/roadmap/${made}`
  }
  async function addPhase(e: SubmitEvent) {
    e.preventDefault()
    if (!id || !newPhase.trim()) return
    await createPhase(id, newPhase.trim())
    newPhase = ''
    await load()
  }
  async function runImport(plan: MermaidPlan) {
    if (!id) return
    await importMermaid(id, plan)
    importing = false
    onchange()
    await load()
  }
  async function runCodeImport() {
    importing = false
    onchange()
    await load()
  }
  async function place(itemId: string, phaseId: string | null) {
    if (!id) return
    await placeItem(id, itemId, phaseId)
    await load()
  }
  async function toggle(p: PhaseSummary) {
    await setPhaseDone(p.id, !p.completed)
    await load()
  }
  function drop(e: DragEvent, phaseId: string | null) {
    e.preventDefault()
    const item = dragging
    dragging = over = null
    if (item) place(item, phaseId)
  }
</script>

{#snippet bar(p: number)}
  <div class="h-1.5 w-full overflow-hidden rounded-full bg-sunk" role="progressbar" aria-valuenow={Math.round(p * 100)}>
    <div class="h-full bg-accent" style="width: {pct(p)}"></div>
  </div>
{/snippet}

{#if !id}
  <h1 class="page-title">Roadmaps</h1>
  <ul class="flex flex-col gap-1">
    {#each list as r (r.id)}
      <li>
        <a href="#/roadmap/{r.id}" class="flex h-9 items-center rounded-[7px] px-2.5 hover:bg-hover">{r.name}</a>
      </li>
    {/each}
  </ul>
  {#if loaded && !list.length}
    <Empty
      icon={Map}
      text="Roadmaps plan work in phases toward a goal. Add one below."
      label="Start from 30-day plan"
      onclick={() => (building = true)}
    />
  {/if}
  <form onsubmit={addRoadmap} class="mt-3 flex flex-wrap gap-2">
    <Input bind:value={newName} placeholder="New roadmap" aria-label="New roadmap" class="min-w-0 flex-1" />
    <select
      bind:value={parentId}
      aria-label="Roadmap owner"
      class="h-9 rounded-md border border-line bg-surface px-2 text-sm"
    >
      {#each areas as a (a.area.id)}
        <option value={a.area.id}>{a.area.name}</option>
      {/each}
    </select>
    <Button type="submit" variant="primary">Add</Button>
  </form>
  <div class="mt-3">
    <Button variant="ghost" onclick={() => (building = true)}>Start from 30-day plan</Button>
  </div>
  <RoadmapBuilder bind:open={building} {areas} {onchange} />
{:else if roadmap}
  {#key roadmap.id}
    <div class="mb-3 flex items-center gap-1">
      <h1 class="page-title min-w-0 flex-1">{roadmap.name}</h1>
      <EntityMenu id={roadmap.id} name={roadmap.name} kind="roadmap" fallbackHref="#/roadmaps" archiveable {onchange} />
    </div>
  {/key}
  <div class="mb-3 hidden gap-1 desk:flex" role="group" aria-label="Layout">
    <Button variant={mode === 'canvas' ? 'primary' : 'ghost'} size="sm" onclick={() => (mode = 'canvas')}>Canvas</Button
    >
    <Button variant={mode === 'list' ? 'primary' : 'ghost'} size="sm" onclick={() => (mode = 'list')}>List</Button>
    <Button variant={mode === 'board' ? 'primary' : 'ghost'} size="sm" onclick={() => (mode = 'board')}>Board</Button>
    <Button variant={mode === 'timeline' ? 'primary' : 'ghost'} size="sm" onclick={() => (mode = 'timeline')}
      >Timeline</Button
    >
  </div>

  {#snippet phaseHead(p: PhaseSummary)}
    <div class="flex items-center gap-2">
      <h2 class="flex-1 truncate font-semibold">
        {p.name}
        {#if p.current}<span class="ml-1 rounded-sm bg-accent-line px-1.5 text-[0.7188rem] font-normal">Current</span
          >{/if}
        {#if p.locked}<span class="ml-1 rounded-sm bg-sunk px-1.5 text-[0.7188rem] font-normal">Locked</span>{/if}
      </h2>
      <span class="text-[0.7812rem] text-muted">{pct(p.progress)}</span>
      <Button variant="ghost" size="sm" onclick={() => toggle(p)}>{p.completed ? 'Reopen' : 'Complete'}</Button>
    </div>
    {#if span(p)}<p class="text-[0.7812rem] text-muted">{span(p)}</p>{/if}
    {#if p.gate_title}<p class="text-[0.7812rem] text-muted">After {p.gate_title}</p>{/if}
    {@render bar(p.progress)}
  {/snippet}

  {#snippet move(itemId: string, phaseId: string | null)}
    <select
      aria-label="Move to phase"
      value={phaseId ?? ''}
      onchange={(e) => place(itemId, e.currentTarget.value || null)}
      class="h-6 max-w-28 rounded-md border border-line bg-surface px-1 text-[0.75rem]"
    >
      <option value="">Unplaced</option>
      {#each phases as q (q.id)}
        <option value={q.id}>{q.name}</option>
      {/each}
    </select>
  {/snippet}

  {#if mode === 'canvas'}
    <RoadmapGraphCanvas {id} onchange={load} />
  {/if}

  {#if mode === 'timeline'}
    <div class="hidden desk:block">
      <RoadmapTimeline {phases} {today} />
    </div>
  {/if}

  <div class={mode === 'list' ? '' : 'hidden'}>
    <div class="flex flex-col gap-4">
      {#each phases as p (p.id)}
        <section class="flex flex-col gap-1.5 rounded-lg border border-line p-3" aria-label={p.name}>
          {@render phaseHead(p)}
          <ul class="flex flex-col gap-1">
            {#each p.items as i (i.id)}
              <li class="flex flex-col gap-1">
                <div class="flex items-center gap-2 text-[0.8438rem]">
                  <span class="task-title-1 flex-1" title={i.title}>{i.title}</span>
                  <span class="text-[0.7188rem] text-muted">{i.kind}</span>
                  {@render move(i.id, p.id)}
                </div>
                {@render bar(i.progress)}
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  </div>

  {#if mode === 'board'}
    <div class="hidden gap-3 overflow-x-auto pb-2 desk:flex">
      {#each [...phases, null] as p (p?.id ?? 'unplaced')}
        <section
          aria-label={p?.name ?? 'Unplaced'}
          ondragover={(e) => {
            e.preventDefault()
            over = p?.id ?? ''
          }}
          ondragleave={() => (over = null)}
          ondrop={(e) => drop(e, p?.id ?? null)}
          class="flex w-64 shrink-0 flex-col gap-2 rounded-lg border p-2.5 {over === (p?.id ?? '')
            ? 'border-accent'
            : 'border-line'}"
        >
          {#if p}
            {@render phaseHead(p)}
          {:else}
            <h2 class="font-semibold text-muted">Unplaced</h2>
          {/if}
          {#each p?.items ?? unplaced as i (i.id)}
            <div
              role="listitem"
              draggable="true"
              ondragstart={() => (dragging = i.id)}
              ondragend={() => (dragging = over = null)}
              class="flex cursor-grab flex-col gap-1 rounded-md border border-line bg-surface p-2 text-[0.8438rem] shadow-1"
            >
              <span class="task-title-2" title={i.title}>{i.title}</span>
              <span class="text-[0.7188rem] text-muted">{i.kind}</span>
              {#if p}{@render bar((i as PhaseSummary['items'][number]).progress)}{/if}
            </div>
          {/each}
        </section>
      {/each}
    </div>
  {/if}

  {#if unplaced.length && mode !== 'board'}
    <section class="mt-4 {mode === 'canvas' ? 'desk:hidden' : ''}">
      <h2 class="pb-1 text-[0.7812rem] font-semibold text-muted">Add to a phase</h2>
      <ul class="flex flex-col gap-1">
        {#each unplaced as c (c.id)}
          <li class="flex items-center gap-2 text-[0.8438rem]">
            <span class="task-title-1 flex-1" title={c.title}>{c.title}</span>
            <span class="text-[0.7188rem] text-muted">{c.kind}</span>
            {@render move(c.id, null)}
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if mode !== 'canvas'}
    <form onsubmit={addPhase} class="mt-4 flex gap-2">
      <Input bind:value={newPhase} placeholder="New phase" aria-label="New phase" class="min-w-0 flex-1" />
      <Button type="submit" variant="primary">Add phase</Button>
    </form>
  {/if}
  <div class="mt-3 flex gap-2">
    <Button variant="ghost" onclick={() => (importing = true)}>Import Mermaid…</Button>
  </div>
  <Dialog.Root bind:open={importing}>
    <Dialog.Content class="max-h-[85vh] overflow-y-auto">
      <Dialog.Title>Import Mermaid</Dialog.Title>
      <MermaidImport onimport={runImport} roadmapId={id} onimportcode={runCodeImport} />
    </Dialog.Content>
  </Dialog.Root>
  <Backlinks id={roadmap.id} />
{:else if loaded}
  <p class="text-sm text-muted">This roadmap no longer exists.</p>
{/if}
