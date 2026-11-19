<script lang="ts">
  import Menu from '@lucide/svelte/icons/menu'
  import Plus from '@lucide/svelte/icons/plus'
  import LayoutGrid from '@lucide/svelte/icons/layout-grid'
  import type { Snippet } from 'svelte'
  import * as Sheet from '$lib/components/ui/sheet'
  import { Button } from '$lib/components/ui/button'
  import { routes, type Route } from '$lib/routes'

  let {
    route,
    inboxCount,
    onNew,
    children,
  }: { route: Route; inboxCount: number; onNew: () => void; children: Snippet } = $props()
  let drawer = $state(false)

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
</script>

<svelte:window {onkeydown} />

{#snippet row(r: Route, count?: number)}
  {@const Icon = routes[r].icon}
  <a
    href="#/{r}"
    onclick={() => (drawer = false)}
    aria-current={route === r ? 'page' : undefined}
    class="group flex h-8 items-center gap-[9px] rounded-[7px] px-2.5 hover:bg-hover aria-[current=page]:bg-press aria-[current=page]:font-semibold"
  >
    <Icon size={16} class="text-icon group-aria-[current=page]:text-accent" />
    <span class="flex-1">{routes[r].label}</span>
    {#if count}<span class="text-[12.5px] text-muted">{count}</span>{/if}
  </a>
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
    <section>
      <h2 class="px-2.5 pb-1 text-[12.5px] font-semibold text-muted">Areas</h2>
      <p class="px-2.5 text-[12.5px] text-muted">Areas you add show up here.</p>
    </section>
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
      <span class="flex-1 text-center font-medium text-muted desk:text-left">{routes[route].label}</span>
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
        aria-current={route === t ? 'page' : undefined}
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
