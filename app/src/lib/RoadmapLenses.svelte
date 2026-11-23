<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import { Button } from '$lib/components/ui/button'
  import { groupByBand, hiddenNodeIds } from '$lib/roadmapGraph'
  import { setPhaseDone, type GraphEdge, type GraphNode, type Phase, type PhaseSummary } from '$lib/sync'

  let {
    mode,
    nodes,
    edges,
    bands,
    summary,
    onchange,
  }: {
    mode: 'list' | 'board'
    nodes: GraphNode[]
    edges: GraphEdge[]
    bands: Phase[]
    summary: PhaseSummary[]
    onchange?: () => void
  } = $props()

  let collapsed = $state<Set<string>>(new Set())

  const hidden = $derived(hiddenNodeIds(nodes, edges))
  const groups = $derived(
    groupByBand(
      nodes.filter((n) => !hidden.has(n.id)),
      bands,
    ),
  )
  const pct = (p: number) => `${Math.round(p * 100)}%`
  const progressOf = (bandId: string) => summary.find((p) => p.id === bandId)?.progress ?? 0
  const keyOf = (band: Phase | null) => band?.id ?? ''
  const phaseOf = (band: Phase | null) => (band ? summary.find((p) => p.id === band.id) : undefined)
  const dates = (p: PhaseSummary) => [p.start, p.end].filter(Boolean).join(' – ')

  function toggle(key: string) {
    const next = new Set(collapsed)
    if (next.has(key)) next.delete(key)
    else next.add(key)
    collapsed = next
  }

  async function toggleDone(p: PhaseSummary) {
    await setPhaseDone(p.id, !p.completed)
    onchange?.()
  }
</script>

{#snippet bar(p: number)}
  <div class="h-1.5 w-full overflow-hidden rounded-full bg-sunk" role="progressbar" aria-valuenow={Math.round(p * 100)}>
    <div class="h-full bg-accent" style="width: {pct(p)}"></div>
  </div>
{/snippet}

{#if mode === 'list'}
  <div class="flex flex-col gap-4">
    {#each groups as g (keyOf(g.band))}
      {@const key = keyOf(g.band)}
      {@const open = !collapsed.has(key)}
      {@const ps = phaseOf(g.band)}
      <section class="flex flex-col gap-1.5 rounded-lg border border-line p-3" aria-label={g.band?.name ?? 'No band'}>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center gap-1 text-left font-semibold"
            aria-expanded={open}
            onclick={() => toggle(key)}
          >
            {#if open}<ChevronDown class="size-4 shrink-0" />{:else}<ChevronRight class="size-4 shrink-0" />{/if}
            <span class="truncate">{g.band?.name ?? 'No band'}</span>
            {#if ps?.current}<span class="ml-1 rounded-sm bg-accent-line px-1.5 text-[0.7188rem] font-normal"
                >Current</span
              >{/if}
            {#if ps?.locked}<span class="ml-1 rounded-sm bg-sunk px-1.5 text-[0.7188rem] font-normal">Locked</span>{/if}
          </button>
          {#if g.band}<span class="text-[0.7812rem] text-muted">{pct(progressOf(g.band.id))}</span>{/if}
          {#if ps}
            <Button variant="ghost" size="sm" onclick={() => toggleDone(ps)}
              >{ps.completed ? 'Reopen' : 'Complete'}</Button
            >
          {/if}
        </div>
        {#if ps && dates(ps)}<p class="text-[0.7812rem] text-muted">{dates(ps)}</p>{/if}
        {#if ps?.gate_title}<p class="text-[0.7812rem] text-muted">After {ps.gate_title}</p>{/if}
        {#if g.band}{@render bar(progressOf(g.band.id))}{/if}
        {#if open}
          <ul class="flex flex-col gap-1">
            {#each g.nodes as n (n.id)}
              <li class="flex items-center gap-2 text-[0.8438rem]">
                <span class="task-title-1 flex-1" title={n.title}>{n.title}</span>
                <span class="text-[0.7188rem] text-muted">{n.node_kind}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  </div>
{:else}
  <div class="flex gap-3 overflow-x-auto pb-2">
    {#each groups as g (keyOf(g.band))}
      {@const ps = phaseOf(g.band)}
      <section
        aria-label={g.band?.name ?? 'No band'}
        class="flex w-64 shrink-0 flex-col gap-2 rounded-lg border border-line p-2.5"
      >
        <div class="flex items-center gap-2">
          <h2 class="flex-1 truncate font-semibold {g.band ? '' : 'text-muted'}">
            {g.band?.name ?? 'No band'}
            {#if ps?.current}<span class="ml-1 rounded-sm bg-accent-line px-1.5 text-[0.7188rem] font-normal"
                >Current</span
              >{/if}
            {#if ps?.locked}<span class="ml-1 rounded-sm bg-sunk px-1.5 text-[0.7188rem] font-normal">Locked</span>{/if}
          </h2>
          {#if g.band}<span class="text-[0.7812rem] text-muted">{pct(progressOf(g.band.id))}</span>{/if}
          {#if ps}
            <Button variant="ghost" size="sm" onclick={() => toggleDone(ps)}
              >{ps.completed ? 'Reopen' : 'Complete'}</Button
            >
          {/if}
        </div>
        {#if ps && dates(ps)}<p class="text-[0.7812rem] text-muted">{dates(ps)}</p>{/if}
        {#if ps?.gate_title}<p class="text-[0.7812rem] text-muted">After {ps.gate_title}</p>{/if}
        {#if g.band}{@render bar(progressOf(g.band.id))}{/if}
        {#each g.nodes as n (n.id)}
          <div class="flex flex-col gap-1 rounded-md border border-line bg-surface p-2 text-[0.8438rem] shadow-1">
            <span class="task-title-2" title={n.title}>{n.title}</span>
          </div>
        {/each}
      </section>
    {/each}
  </div>
{/if}
