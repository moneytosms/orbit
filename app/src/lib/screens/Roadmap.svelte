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
  import RoadmapLenses from '$lib/RoadmapLenses.svelte'
  import RoadmapTimeline from '$lib/RoadmapTimeline.svelte'
  import { hiddenNodeIds } from '$lib/roadmapGraph'
  import {
    createPhase,
    createRoadmap,
    getRoadmap,
    importMermaid,
    localDay,
    roadmapGraph,
    roadmaps,
    roadmapSummary,
    type GraphEdge,
    type GraphNode,
    type MermaidPlan,
    type Phase,
    type PhaseSummary,
    type Roadmap,
    type tree,
  } from '$lib/sync'

  let { id, areas, onchange }: { id?: string; areas: Awaited<ReturnType<typeof tree>>; onchange: () => void } = $props()

  let list = $state<Roadmap[]>([])
  let roadmap = $state<Roadmap | undefined>()
  let phases = $state<PhaseSummary[]>([])
  let nodes = $state<GraphNode[]>([])
  let edges = $state<GraphEdge[]>([])
  let bands = $state<Phase[]>([])
  let mode = $state<'canvas' | 'list' | 'board' | 'timeline'>('canvas')
  let today = $state('')
  let building = $state(false)
  let importing = $state(false)
  let loaded = $state(false)
  let newName = $state('')
  let newPhase = $state('')
  let parentId = $state('')

  async function load() {
    if (id) {
      roadmap = await getRoadmap(id)
      today = await localDay()
      phases = roadmap ? await roadmapSummary(id, today) : []
      const graph = roadmap ? await roadmapGraph(id) : null
      nodes = graph?.nodes ?? []
      edges = graph?.edges ?? []
      bands = graph?.bands ?? []
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

  const unscheduled = $derived.by(() => {
    const hidden = hiddenNodeIds(nodes, edges)
    const dated = new Set(bands.filter((b) => b.start && b.end).map((b) => b.id))
    return nodes.filter((n) => !hidden.has(n.id) && !(n.band_id && dated.has(n.band_id)))
  })

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
</script>

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

  {#if mode === 'canvas'}
    <RoadmapGraphCanvas {id} onchange={load} />
  {/if}

  {#if mode === 'timeline'}
    <div class="hidden desk:block">
      <RoadmapTimeline {phases} {today} />
      {#if unscheduled.length}
        <section class="mt-4" aria-label="Unscheduled">
          <h2 class="pb-1 text-[0.7812rem] font-semibold text-muted">Unscheduled</h2>
          <ul class="flex flex-col gap-1">
            {#each unscheduled as n (n.id)}
              <li class="flex items-center gap-2 text-[0.8438rem]">
                <span class="task-title-1 flex-1" title={n.title}>{n.title}</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  {/if}

  {#if mode === 'list' || mode === 'board'}
    <RoadmapLenses {mode} {nodes} {edges} {bands} summary={phases} onchange={load} />
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
