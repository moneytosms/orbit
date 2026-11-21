<script lang="ts">
  import { tick } from 'svelte'
  import Plus from '@lucide/svelte/icons/plus'
  import { DropdownMenu } from 'bits-ui'
  import { Button } from '$lib/components/ui/button'
  import RoadmapBuilder from '$lib/RoadmapBuilder.svelte'
  import { createGoal, createPage, createProject, type tree } from '$lib/sync'

  let {
    ownerId,
    ownerName,
    ownerKind,
    areas,
    onchange,
    onnewtask,
  }: {
    ownerId: string
    ownerName: string
    ownerKind: 'area' | 'project'
    areas: Awaited<ReturnType<typeof tree>>
    onchange: () => void
    onnewtask: (prefill: string) => void
  } = $props()

  let open = $state(false)
  let prompt = $state<'project' | 'goal' | null>(null)
  let name = $state('')
  let input: HTMLInputElement | null = $state(null)
  let wrapper: HTMLDivElement | null = $state(null)
  let promptStyle = $state('')
  let building = $state(false)

  function choosePrompt(kind: 'project' | 'goal') {
    prompt = kind
    name = ''
    const trigger = wrapper?.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')
    const rect = trigger?.getBoundingClientRect()
    if (rect) {
      const left = Math.min(Math.max(8, rect.left), window.innerWidth - 232)
      promptStyle = `left:${left}px;top:${rect.bottom + 4}px`
    }
    setTimeout(() => {
      void tick().then(() => input?.focus())
    })
  }

  async function create(e: SubmitEvent) {
    e.preventDefault()
    const value = name.trim()
    if (!value || !prompt) return
    if (prompt === 'project') await createProject(ownerId, value)
    else await createGoal(ownerId, value)
    name = ''
    prompt = null
    open = false
    onchange()
    await tick()
    wrapper?.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.focus()
  }

  async function newPage() {
    const id = await createPage(ownerId, 'Untitled')
    onchange()
    location.hash = `#/page/${id}`
  }
</script>

<svelte:window
  onpointerdown={(event) => {
    if (prompt && !wrapper?.contains(event.target as Node)) prompt = null
  }}
  onkeydown={(event) => {
    if (prompt && event.key === 'Escape') {
      prompt = null
      wrapper?.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')?.focus()
    }
  }}
/>

<div bind:this={wrapper} class="relative inline-flex">
  <DropdownMenu.Root bind:open>
    <DropdownMenu.Trigger aria-label="Create" onclick={() => (prompt = null)}>
      {#snippet child({ props })}
        <Button {...props} variant="ghost" size="icon-sm" aria-label="Create">
          <Plus size={16} />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Portal>
      <DropdownMenu.Content
        align="start"
        sideOffset={5}
        class="z-50 min-w-44 rounded-xl border border-line bg-surface p-1.5 shadow-2"
      >
        {#if !prompt}
          <DropdownMenu.Item
            class="flex h-9 cursor-default items-center rounded-md px-2 text-sm outline-none data-[highlighted]:bg-hover"
            onSelect={() => {
              open = false
              onnewtask(`@${ownerName} `)
            }}>New task</DropdownMenu.Item
          >
          {#if ownerKind === 'area'}
            <DropdownMenu.Item
              class="flex h-9 cursor-default items-center rounded-md px-2 text-sm outline-none data-[highlighted]:bg-hover"
              onSelect={() => choosePrompt('project')}>New project</DropdownMenu.Item
            >
          {/if}
          <DropdownMenu.Item
            class="flex h-9 cursor-default items-center rounded-md px-2 text-sm outline-none data-[highlighted]:bg-hover"
            onSelect={() => choosePrompt('goal')}>New goal</DropdownMenu.Item
          >
          <DropdownMenu.Item
            class="flex h-9 cursor-default items-center rounded-md px-2 text-sm outline-none data-[highlighted]:bg-hover"
            onSelect={() => (building = true)}>New roadmap</DropdownMenu.Item
          >
          <DropdownMenu.Item
            class="flex h-9 cursor-default items-center rounded-md px-2 text-sm outline-none data-[highlighted]:bg-hover"
            onSelect={newPage}>New page</DropdownMenu.Item
          >
        {/if}
      </DropdownMenu.Content>
    </DropdownMenu.Portal>
  </DropdownMenu.Root>

  {#if prompt}
    <form
      class="fixed z-50 grid w-[min(14rem,calc(100vw-1rem))] gap-1.5 rounded-xl border border-line bg-surface p-2 shadow-2"
      style={promptStyle}
      onsubmit={create}
    >
      <input
        bind:this={input}
        bind:value={name}
        aria-label={prompt === 'project' ? 'Project name' : 'Goal name'}
        placeholder={prompt === 'project' ? 'Project name' : 'Goal name'}
        class="h-[2.125rem] w-full min-w-0 rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none transition-colors placeholder:text-muted focus-visible:border-accent-line focus-visible:shadow-[0_0_0_3px_var(--accent-soft)]"
      />
      <Button type="submit" size="sm" variant="primary">Create</Button>
    </form>
  {/if}
</div>

<RoadmapBuilder bind:open={building} {areas} {ownerId} {ownerName} {onchange} />
