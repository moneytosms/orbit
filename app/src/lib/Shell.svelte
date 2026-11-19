<script lang="ts">
  import Menu from '@lucide/svelte/icons/menu'
  import Plus from '@lucide/svelte/icons/plus'
  import LayoutGrid from '@lucide/svelte/icons/layout-grid'
  import type { Snippet } from 'svelte'
  import { dndzone, TRIGGERS, type DndEvent } from 'svelte-dnd-action'
  import * as Sheet from '$lib/components/ui/sheet'
  import { Button } from '$lib/components/ui/button'
  import { routes, type Route, type Screen } from '$lib/routes'
  import { createArea, createProject, moveItem, orderBetween, type Area, type Project } from '$lib/sync'

  type AreaItem = { id: string; area: Area; projects: Project[] }

  let {
    screen,
    title,
    areas,
    inboxCount,
    onNew,
    children,
  }: {
    screen: Screen
    title: string
    areas: { area: Area; projects: Project[] }[]
    inboxCount: number
    onNew: () => void
    children: Snippet
  } = $props()
  let drawer = $state(false)

  let areaOrder: AreaItem[] = $derived(areas.map((t) => ({ id: t.area.id, ...t })))
  let newArea = $state('')
  let newProject = $state<Record<string, string>>({})

  function newTask() {
    drawer = false
    onNew()
  }
  function onkeydown(e: KeyboardEvent) {
    const t = e.target as HTMLElement
    if (e.key === 'q' && !e.ctrlKey && !e.metaKey && !e.altKey && !t.closest('input, textarea, [contenteditable]')) {
      e.preventDefault()
      newTask()
    }
  }
  const tabs: (Route | 'add' | 'browse')[] = ['home', 'upcoming', 'add', 'routines', 'browse']

  // A drop takes the order between its new neighbours.
  function considerAreas(e: CustomEvent<DndEvent<AreaItem>>) {
    areaOrder = e.detail.items
  }
  function finalizeAreas(e: CustomEvent<DndEvent<AreaItem>>) {
    areaOrder = e.detail.items
    const { id, trigger } = e.detail.info
    if (trigger !== TRIGGERS.DROPPED_INTO_ZONE) return
    const i = areaOrder.findIndex((a) => a.id === id)
    moveItem(id, null, orderBetween(areaOrder[i - 1]?.area.order, areaOrder[i + 1]?.area.order))
  }

  async function addArea(e: SubmitEvent) {
    e.preventDefault()
    if (!newArea.trim()) return
    await createArea(newArea)
    newArea = ''
  }
  async function addProject(e: SubmitEvent, areaId: string) {
    e.preventDefault()
    const name = newProject[areaId] ?? ''
    if (!name.trim()) return
    await createProject(areaId, name)
    newProject[areaId] = ''
  }
</script>

<svelte:window {onkeydown} />

{#snippet row(r: Route, count?: number)}
  {@const Icon = routes[r].icon}
  <a
    href="#/{r}"
    onclick={() => (drawer = false)}
    aria-current={screen.name === r ? 'page' : undefined}
    class="group flex h-8 items-center gap-[9px] rounded-[7px] px-2.5 hover:bg-hover aria-[current=page]:bg-press aria-[current=page]:font-semibold"
  >
    <Icon size={16} class="text-icon group-aria-[current=page]:text-accent" />
    <span class="flex-1">{routes[r].label}</span>
    {#if count}<span class="text-[12.5px] text-muted">{count}</span>{/if}
  </a>
{/snippet}

{#snippet areaTree()}
  <section class="flex flex-col gap-1">
    <h2 class="px-2.5 pb-1 text-[12.5px] font-semibold text-muted">Areas</h2>
    <ul
      use:dndzone={{ items: areaOrder, flipDurationMs: 150, delayTouchStart: 200 }}
      onconsider={considerAreas}
      onfinalize={finalizeAreas}
      class="flex flex-col gap-1"
    >
      {#each areaOrder as a (a.id)}
        <li class="flex flex-col gap-0.5">
          <a
            href="#/area/{a.id}"
            aria-current={screen.name === 'area' && screen.id === a.id ? 'page' : undefined}
            class="flex h-8 items-center rounded-[7px] px-2.5 font-medium hover:bg-hover aria-[current=page]:bg-press"
          >
            {a.area.name}
          </a>
          {#each a.projects.slice(0, 4) as p (p.id)}
            <a
              href="#/project/{p.id}"
              aria-current={screen.name === 'project' && screen.id === p.id ? 'page' : undefined}
              class="flex h-7 items-center rounded-[7px] pr-2.5 pl-6 text-[13.5px] hover:bg-hover aria-[current=page]:bg-press aria-[current=page]:font-semibold"
            >
              {p.name}
            </a>
          {/each}
          {#if a.projects.length > 4}
            <span class="pl-6 text-[12.5px] text-muted">+{a.projects.length - 4} more</span>
          {/if}
          <form onsubmit={(e) => addProject(e, a.id)} class="px-2">
            <input
              bind:value={newProject[a.id]}
              placeholder="New project"
              aria-label="New project in {a.area.name}"
              class="h-7 w-full bg-transparent pl-4 text-[13px] outline-none placeholder:text-muted"
            />
          </form>
        </li>
      {/each}
    </ul>
    <form onsubmit={addArea} class="px-2">
      <input
        bind:value={newArea}
        placeholder="New area"
        aria-label="New area"
        class="h-7 w-full bg-transparent text-[13.5px] outline-none placeholder:text-muted"
      />
    </form>
  </section>
{/snippet}

{#snippet nav()}
  <div class="flex h-full flex-col gap-4 overflow-y-auto p-3">
    <div class="flex items-center gap-2 px-1.5">
      <img src="/icon.svg" alt="" class="size-6" />
      <span class="flex-1 text-[15px] font-semibold">Orbit</span>
      <Button variant="primary" size="icon" aria-label="New task" onclick={newTask}>
        <Plus class="text-accent-ink" />
      </Button>
    </div>
    <nav class="flex flex-col gap-0.5" aria-label="Main">
      {@render row('home')}
      {@render row('inbox', inboxCount)}
      {@render row('upcoming')}
    </nav>
    <nav class="flex flex-col gap-0.5" aria-label="Plans">
      {@render row('goals')}
      {@render row('roadmaps')}
      {@render row('routines')}
    </nav>
    {@render areaTree()}
    <section>
      <h2 class="px-2.5 pb-1 text-[12.5px] font-semibold text-muted">Views</h2>
      <p class="px-2.5 text-[12.5px] text-muted">Saved views show up here.</p>
    </section>
    <nav class="mt-auto flex flex-col gap-0.5" aria-label="Account">
      {@render row('profile')}
      {@render row('settings')}
    </nav>
  </div>
{/snippet}

<div class="flex h-full">
  <aside class="hidden w-64 shrink-0 border-r border-line bg-side desk:block">{@render nav()}</aside>

  <div class="flex min-w-0 flex-1 flex-col">
    <header class="flex h-12 shrink-0 items-center gap-2 px-2 desk:px-4">
      <Button variant="ghost" size="icon" class="desk:hidden" aria-label="Open menu" onclick={() => (drawer = true)}>
        <Menu />
      </Button>
      <span class="flex-1 text-center font-medium text-muted desk:text-left">{title}</span>
      <Button variant="ghost" class="hidden desk:inline-flex" onclick={newTask}>
        <Plus /> New task <kbd class="rounded-sm bg-sunk px-1.5 text-[11.5px] text-muted">Q</kbd>
      </Button>
      <span class="size-8 desk:hidden"></span>
    </header>
    <main class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-[860px] px-4 pt-2 pb-28 desk:px-10 desk:pb-16">{@render children()}</div>
    </main>
  </div>
</div>

<nav
  aria-label="Tabs"
  class="fixed inset-x-0 bottom-0 grid h-[72px] grid-cols-5 border-t border-line bg-surface/92 pb-[env(safe-area-inset-bottom)] backdrop-blur desk:hidden"
>
  {#each tabs as t (t)}
    {#if t === 'add'}
      <div class="grid place-items-center">
        <button
          type="button"
          aria-label="New task"
          onclick={newTask}
          class="grid size-12 place-items-center rounded-[15px] bg-accent text-accent-ink shadow-[0_4px_12px_rgba(0,0,0,.18)]"
        >
          <Plus size={22} />
        </button>
      </div>
    {:else if t === 'browse'}
      <button type="button" onclick={() => (drawer = true)} class="flex flex-col items-center justify-center gap-1 text-[11.5px] font-medium text-muted">
        <LayoutGrid size={20} class="text-icon" /> Browse
      </button>
    {:else}
      {@const Icon = routes[t].icon}
      <a
        href="#/{t}"
        aria-current={screen.name === t ? 'page' : undefined}
        class="group flex flex-col items-center justify-center gap-1 text-[11.5px] font-medium text-muted aria-[current=page]:text-accent-text"
      >
        <Icon size={20} class="text-icon group-aria-[current=page]:text-accent" />
        {routes[t].label}
      </a>
    {/if}
  {/each}
</nav>

<Sheet.Root bind:open={drawer}>
  <Sheet.Content side="left" class="w-64 max-w-[85vw] bg-side p-0" showCloseButton={false}>
    <Sheet.Title class="sr-only">Menu</Sheet.Title>
    {@render nav()}
  </Sheet.Content>
</Sheet.Root>
