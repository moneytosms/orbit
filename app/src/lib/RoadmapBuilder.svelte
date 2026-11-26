<script lang="ts">
  import { Select } from '$lib/components/ui/select'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import * as Dialog from '$lib/components/ui/dialog'
  import MermaidImport from '$lib/MermaidImport.svelte'
  import { dayDate, dayNumber } from '$lib/roadmapTimeline'
  import { createPhase, createRoadmap, dayFrom, importMermaid, type MermaidPlan, type tree } from '$lib/sync'

  let {
    open = $bindable(false),
    areas,
    ownerId = '',
    ownerName = '',
    onchange,
  }: {
    open: boolean
    areas: Awaited<ReturnType<typeof tree>>
    ownerId?: string
    ownerName?: string
    onchange: () => void
  } = $props()

  const defaults = ['Week 1', 'Week 2', 'Week 3', 'Final stretch']
  const days: [number, number][] = [
    [0, 6],
    [7, 13],
    [14, 20],
    [21, 29],
  ]

  let tab = $state<'plan' | 'mermaid'>('plan')
  let owner = $state('')
  let name = $state('')
  let start = $state(dayFrom(0))
  let names = $state([...defaults])
  let busy = $state(false)

  $effect(() => {
    if (open) owner = ownerId || areas[0]?.area.id || ''
  })
  const extra = $derived(ownerId && !areas.some((a) => a.area.id === ownerId))
  const phases = $derived(
    start
      ? days.map(([a, b], i) => ({
          name: names[i],
          start: dayDate(dayNumber(start) + a),
          end: dayDate(dayNumber(start) + b),
        }))
      : [],
  )

  async function open_(id: string) {
    onchange()
    open = false
    location.hash = `#/roadmap/${id}`
  }
  async function create(e: SubmitEvent) {
    e.preventDefault()
    if (!owner || !name.trim() || !phases.length || busy) return
    busy = true
    try {
      const id = await createRoadmap(owner, name.trim())
      for (const p of phases) await createPhase(id, p.name.trim() || 'Phase', p.start, p.end)
      await open_(id)
    } finally {
      busy = false
    }
  }
  async function fromMermaid(plan: MermaidPlan) {
    if (!owner) return
    const id = await createRoadmap(owner, name.trim() || plan.title || 'Imported roadmap')
    await importMermaid(id, plan)
    await open_(id)
  }
</script>

{#snippet ownerField()}
  <label class="flex flex-col gap-1 text-[0.8125rem]">
    Owner
    <Select
      bind:value={owner}
      segmented={false}
      class="w-full"
      options={[
        ...(extra ? [{ value: ownerId, label: ownerName || 'This item' }] : []),
        ...areas.map((a) => ({ value: a.area.id, label: a.area.name })),
      ]}
    />
  </label>
{/snippet}

<Dialog.Root bind:open>
  <Dialog.Content class="max-h-[85vh] overflow-y-auto">
    <Dialog.Title>New roadmap</Dialog.Title>
    <div class="flex gap-1" role="tablist">
      <Button
        variant={tab === 'plan' ? 'primary' : 'ghost'}
        size="sm"
        role="tab"
        aria-selected={tab === 'plan'}
        onclick={() => (tab = 'plan')}>Start from 30-day plan</Button
      >
      <Button
        variant={tab === 'mermaid' ? 'primary' : 'ghost'}
        size="sm"
        role="tab"
        aria-selected={tab === 'mermaid'}
        onclick={() => (tab = 'mermaid')}>From Mermaid</Button
      >
    </div>
    {#if tab === 'plan'}
      <form onsubmit={create} class="flex flex-col gap-3">
        {@render ownerField()}
        <label class="flex flex-col gap-1 text-[0.8125rem]">
          Name
          <Input bind:value={name} placeholder="30-day plan" />
        </label>
        <label class="flex flex-col gap-1 text-[0.8125rem]">
          Start date
          <Input type="date" bind:value={start} />
        </label>
        <ul class="flex flex-col gap-1" aria-label="Phases">
          {#each phases as p, i (i)}
            <li class="flex items-center gap-2 text-[0.8125rem]">
              <Input bind:value={names[i]} aria-label="Phase {i + 1} name" class="min-w-0 flex-1" />
              <span class="text-muted">{p.start} – {p.end}</span>
            </li>
          {/each}
        </ul>
        <div class="flex justify-end">
          <Button type="submit" variant="primary" disabled={!owner || !name.trim() || busy}>Create</Button>
        </div>
      </form>
    {:else}
      {@render ownerField()}
      <label class="flex flex-col gap-1 text-[0.8125rem]">
        Name
        <Input bind:value={name} placeholder="Imported roadmap" />
      </label>
      <MermaidImport onimport={fromMermaid} />
    {/if}
  </Dialog.Content>
</Dialog.Root>
