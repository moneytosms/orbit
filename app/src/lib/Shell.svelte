<script lang="ts">
  import Menu from '@lucide/svelte/icons/menu'
  import Search from '@lucide/svelte/icons/search'
  import Plus from '@lucide/svelte/icons/plus'
  import Check from '@lucide/svelte/icons/check'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import X from '@lucide/svelte/icons/x'
  import { onMount, tick, type Snippet } from 'svelte'
  import { dndzone, TRIGGERS, type DndEvent } from 'svelte-dnd-action'
  import * as Sheet from '$lib/components/ui/sheet'
  import * as Dialog from '$lib/components/ui/dialog'
  import Composer from './Composer.svelte'
  import EntityCreateMenu from './EntityCreateMenu.svelte'
  import EntityMenu from './EntityMenu.svelte'
  import { favorites, resolveFavorites, type FavoriteEntry } from '$lib/favorites'
  import Palette from './Palette.svelte'
  import { runAction, setFocusedTask, ui } from '$lib/actions'
  import { refreshAppBadge } from '$lib/appBadge'
  import { clearCursor, isTyping, matcher } from '$lib/keys'
  import Shortcuts from './Shortcuts.svelte'
  import DropTarget from './DropTarget.svelte'
  import ProfileAvatar from './ProfileAvatar.svelte'
  import { deviceId } from '$lib/auth'
  import { Button } from '$lib/components/ui/button'
  import { routes, type Route, type Screen } from '$lib/routes'
  import {
    createArea,
    createProject,
    createView,
    moveItem,
    orderBetween,
    goals,
    pinnedViews,
    reorderViews,
    updateView,
    views,
    VIEW_INBOX,
    VIEW_UPCOMING,
    type Area,
    type Project,
    type Goal,
    type View,
    getProfile,
    getPage,
    getRoadmap,
  } from '$lib/sync'
  import { inbox_id } from '../../wasm/orbit_wasm'

  type AreaItem = { id: string; area: Area; projects: Project[] }
  type ViewPin = { id: string; name: string; builtin: string | null }

  let {
    screen,
    title,
    areas,
    inboxCount,
    parentId,
    onchange,
    children,
  }: {
    screen: Screen
    title: string
    areas: { area: Area; projects: Project[] }[]
    inboxCount: number
    parentId: string
    onchange: () => void
    children: Snippet<[(prefill?: string, scoped?: boolean) => void]>
  } = $props()
  let drawer = $state(false)
  let composing = $state(false)
  let palette = $state(false)
  let help = $state(false)
  let prefill = $state('')
  let forceParent = $state(false)
  let wide = $state(false)
  let composerOpener: HTMLElement | null = null

  type Crumb = { label: string; href?: string }
  let crumbs = $state<Crumb[]>([])
  /** Area › Project › Page trail above `parentId`, outermost first. Pages nest, so walk up (bounded). */
  async function trail(parentId: string): Promise<Crumb[]> {
    const out: Crumb[] = []
    let id = parentId
    for (let depth = 0; depth < 8; depth++) {
      const project = areas.flatMap((a) => a.projects).find((p) => p.id === id)
      if (project) {
        out.unshift({ label: project.name, href: `#/project/${project.id}` })
        id = project.parent_id
        continue
      }
      const area = areas.find((a) => a.area.id === id)
      if (area) {
        out.unshift({ label: area.area.name, href: `#/area/${area.area.id}` })
        break
      }
      const page = await getPage(id)
      if (!page) break
      out.unshift({ label: page.title || 'Untitled', href: `#/page/${page.id}` })
      id = page.parent_id
    }
    return out
  }
  $effect(() => {
    const s = screen
    void areas
    let live = true
    const build = async (): Promise<Crumb[]> => {
      if (s.name === 'area' || s.name === 'project') {
        const chain = await trail(s.id)
        return chain.map((c, i) => (i === chain.length - 1 ? { label: c.label } : c))
      }
      if (s.name === 'page') {
        const page = await getPage(s.id)
        return page ? [...(await trail(page.parent_id)), { label: page.title || 'Untitled' }] : [{ label: title }]
      }
      if (s.name === 'roadmap') {
        const r = await getRoadmap(s.id)
        return r ? [{ label: 'Roadmaps', href: '#/roadmaps' }, { label: r.name }] : [{ label: title }]
      }
      return [{ label: title }]
    }
    void build().then((c) => live && (crumbs = c))
    return () => (live = false)
  })

  let areaOrder: AreaItem[] = $derived(areas.map((t) => ({ id: t.area.id, ...t })))
  let newArea = $state('')
  let addingArea = $state(false)
  let areaOpener: HTMLElement | null = null
  let areaSection: HTMLElement | null = null
  let newProject = $state<Record<string, string>>({})
  let areaExpansion = $state<Record<string, boolean>>({})
  let pinned = $state<View[]>([])
  let hiddenViews = $state<View[]>([])
  let allViewRows = $state<View[]>([])
  let activeGoals = $state<Goal[]>([])
  /** Starred Areas, Projects, saved Views and Goals, in the order they were starred. */
  const favoriteList: FavoriteEntry[] = $derived.by(() => {
    const lookup = new Map<string, FavoriteEntry>()
    for (const a of areas) {
      lookup.set(a.area.id, { id: a.area.id, kind: 'area', name: a.area.name, href: `#/area/${a.area.id}` })
      for (const p of a.projects) {
        lookup.set(p.id, { id: p.id, kind: 'project', name: p.name, href: `#/project/${p.id}` })
      }
    }
    for (const v of allViewRows) {
      if (!v.builtin) lookup.set(v.id, { id: v.id, kind: 'view', name: v.name, href: `#/view/${v.id}` })
    }
    for (const g of activeGoals) lookup.set(g.id, { id: g.id, kind: 'goal', name: g.name, href: '#/goals' })
    return resolveFavorites($favorites, lookup)
  })
  let profile = $state({ display_name: '', avatar: '' })
  let device = $state('')
  let viewOrder: ViewPin[] = $derived(pinned.map((v) => ({ id: v.id, name: v.name, builtin: v.builtin })))

  // Reloaded whenever the shell refreshes (`areas` is a fresh array after every sync).
  $effect(() => {
    void areas
    loadViews()
    getProfile().then((p) => (profile = p))
    deviceId().then((id) => (device = id ?? ''))
  })
  onMount(() => {
    const refreshProfile = () => getProfile().then((p) => (profile = p))
    window.addEventListener('orbit-profile-change', refreshProfile)
    refreshProfile()
    try {
      const saved: unknown = JSON.parse(localStorage.getItem('orbit.sidebar.area-expansion') ?? '{}')
      if (saved && typeof saved === 'object' && !Array.isArray(saved)) {
        areaExpansion = Object.fromEntries(
          Object.entries(saved).filter((entry): entry is [string, boolean] => typeof entry[1] === 'boolean'),
        )
      }
    } catch {
      // Keep project-count defaults when storage is unavailable or invalid.
    }
    return () => window.removeEventListener('orbit-profile-change', refreshProfile)
  })
  // The app icon badge recounts on start and whenever the tab comes back; the Glance widget refreshes it too.
  onMount(() => {
    const refreshBadge = () => {
      if (document.visibilityState === 'visible') void refreshAppBadge()
    }
    document.addEventListener('visibilitychange', refreshBadge)
    void refreshAppBadge()
    return () => document.removeEventListener('visibilitychange', refreshBadge)
  })
  $effect(() => {
    if (composing || !composerOpener) return
    const opener = composerOpener
    composerOpener = null
    void tick().then(() => {
      if (opener.isConnected) opener.focus()
    })
  })
  async function loadViews() {
    // Inbox and Upcoming have fixed nav rows instead of pins.
    pinned = (await pinnedViews()).filter((v) => v.id !== VIEW_INBOX && v.id !== VIEW_UPCOMING)
    allViewRows = await views()
    hiddenViews = allViewRows.filter((v) => v.hidden)
    activeGoals = await goals()
  }
  async function newView() {
    const id = await createView('New view', 'task')
    await updateView(id, { pinned: true })
    await loadViews()
    drawer = false
    location.hash = `#/view/${id}`
  }
  async function unhide(id: string) {
    await updateView(id, { hidden: false })
    await loadViews()
  }

  // Desktop gets a centred Dialog, phones a bottom Sheet.
  function open(text = '', scoped = false) {
    composerOpener = document.activeElement instanceof HTMLElement ? document.activeElement : null
    drawer = false
    prefill = text
    forceParent = scoped
    wide = matchMedia('(min-width: 768px)').matches
    composing = true
  }
  function newTask() {
    open()
  }
  ui.newTask = newTask
  ui.capture = (text) => open(text ? `${text} ` : '')
  ui.openPalette = openPalette
  ui.openHelp = () => (help = true)
  function openPalette() {
    drawer = false
    palette = true
  }
  const keys = matcher()
  function onkeydown(e: KeyboardEvent) {
    const id = keys.press(
      { key: e.key, ctrlKey: e.ctrlKey, metaKey: e.metaKey, altKey: e.altKey, typing: isTyping(e.target) },
      e.timeStamp,
    )
    if (!id) return
    const t = e.target as HTMLElement
    // A focused button or link handles Enter itself; list keys leave an open dialog alone.
    if (id === 'cursor.open' && t.closest('button, a, [role="button"]')) return
    if (id.startsWith('cursor.') && document.querySelector('[role="dialog"]')) return
    e.preventDefault()
    runAction(id)
  }
  const isHidden = (id: string) => hiddenViews.some((v) => v.id === id)
  // Two tabs each side of the add button. Search and the full menu live in the header.
  let tabs: (Route | 'add')[] = $derived(
    isHidden(VIEW_UPCOMING)
      ? ['home', 'roadmaps', 'add', 'goals', 'routines']
      : ['home', 'upcoming', 'add', 'goals', 'routines'],
  )

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

  function considerViews(e: CustomEvent<DndEvent<ViewPin>>) {
    viewOrder = e.detail.items
  }
  async function finalizeViews(e: CustomEvent<DndEvent<ViewPin>>) {
    viewOrder = e.detail.items
    const { id, trigger } = e.detail.info
    if (trigger !== TRIGGERS.DROPPED_INTO_ZONE) return
    const i = viewOrder.findIndex((v) => v.id === id)
    await reorderViews(id, viewOrder[i + 1]?.id ?? null)
    await loadViews()
  }

  async function addArea(e: SubmitEvent) {
    e.preventDefault()
    if (!newArea.trim()) return
    await createArea(newArea)
    newArea = ''
    addingArea = false
    restoreAreaFocus()
  }
  function beginArea(e: MouseEvent) {
    areaOpener = e.currentTarget as HTMLElement
    areaSection = areaOpener.closest('section')
    newArea = ''
    addingArea = true
    void tick().then(() => areaSection?.querySelector<HTMLInputElement>('[aria-label="New area name"]')?.focus())
  }
  function cancelArea() {
    addingArea = false
    newArea = ''
    restoreAreaFocus()
  }
  function restoreAreaFocus() {
    const opener = areaOpener
    const section = areaSection
    areaOpener = null
    areaSection = null
    void tick().then(() => {
      const target = opener?.isConnected ? opener : section?.querySelector<HTMLElement>('[aria-label="New area"]')
      target?.focus()
    })
  }
  function areaKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault()
      e.stopPropagation()
      cancelArea()
    }
  }
  async function addProject(e: SubmitEvent, areaId: string) {
    e.preventDefault()
    const name = newProject[areaId] ?? ''
    if (!name.trim()) return
    await createProject(areaId, name)
    newProject[areaId] = ''
  }
  // Only the chevron changes this; the saved choice always wins, so collapsing the current area sticks.
  const areaIsExpanded = (a: AreaItem) => areaExpansion[a.id] ?? a.projects.length > 0
  function toggleArea(a: AreaItem) {
    areaExpansion = { ...areaExpansion, [a.id]: !areaIsExpanded(a) }
    try {
      localStorage.setItem('orbit.sidebar.area-expansion', JSON.stringify(areaExpansion))
    } catch {
      // The choice still applies until the shell reloads.
    }
  }
</script>

<svelte:window {onkeydown} onhashchange={() => clearCursor(setFocusedTask)} />

{#snippet row(r: Route, count?: number)}
  {@const Icon = routes[r].icon}
  <a
    href="#/{r}"
    onclick={() => (drawer = false)}
    aria-current={screen.name === r || (r === 'roadmaps' && screen.name === 'roadmap') ? 'page' : undefined}
    class="group flex h-8 items-center gap-[9px] rounded-[7px] px-2.5 hover:bg-hover aria-[current=page]:bg-press aria-[current=page]:font-semibold"
  >
    {#if r === 'profile'}
      <ProfileAvatar name={profile.display_name} avatar={profile.avatar} {device} size="size-5 text-[0.625rem]" />
    {:else}
      <Icon size={16} class="text-icon group-aria-[current=page]:text-accent" />
    {/if}
    <span class="min-w-0 flex-1 truncate"
      >{r === 'profile' && profile.display_name ? `Signed in as ${profile.display_name}` : routes[r].label}</span
    >
    {#if count}<span class="text-[0.7812rem] text-muted">{count}</span>{/if}
  </a>
{/snippet}

{#snippet areaTree(instance: string)}
  <section class="flex flex-col gap-1">
    <div class="flex items-center gap-1 px-2.5 pb-1">
      {#if addingArea}
        <form onsubmit={addArea} class="flex min-w-0 flex-1 items-center gap-1 text-muted focus-within:text-text">
          <input
            bind:value={newArea}
            placeholder="Area name"
            aria-label="New area name"
            onkeydown={areaKeydown}
            class="h-7 min-w-0 flex-1 bg-transparent text-[0.8438rem] outline-none placeholder:text-muted"
          />
          <Button type="submit" variant="ghost" size="icon-sm" aria-label="Create area" disabled={!newArea.trim()}>
            <Check size={14} />
          </Button>
          <Button variant="ghost" size="icon-sm" aria-label="Cancel adding area" onclick={cancelArea}>
            <X size={14} />
          </Button>
        </form>
      {:else}
        <h2 class="flex-1 text-[0.7812rem] font-semibold text-muted">Areas</h2>
        <Button variant="ghost" size="icon-sm" aria-label="New area" onclick={beginArea}>
          <Plus size={14} />
        </Button>
      {/if}
    </div>
    <ul
      use:dndzone={{ items: areaOrder, flipDurationMs: 150, delayTouchStart: 200 }}
      onconsider={considerAreas}
      onfinalize={finalizeAreas}
      class="flex flex-col gap-1"
    >
      {#each areaOrder as a (a.id)}
        <li class="flex flex-col gap-0.5">
          <DropTarget parentId={a.id}>
            <div
              class="group flex h-8 items-center rounded-[7px] hover:bg-hover"
              class:bg-press={screen.name === 'area' && screen.id === a.id}
            >
              <a
                href="#/area/{a.id}"
                onclick={() => (drawer = false)}
                aria-current={screen.name === 'area' && screen.id === a.id ? 'page' : undefined}
                class="flex min-w-0 flex-1 items-center rounded-[7px] px-2.5 font-medium"
              >
                {#if a.area.icon}<span aria-hidden="true" class="mr-1.5">{a.area.icon}</span>{/if}{a.area.name}
              </a>
              <button
                type="button"
                aria-label="{areaIsExpanded(a) ? 'Collapse' : 'Expand'} {a.area.name}"
                aria-expanded={areaIsExpanded(a)}
                aria-controls="area-projects-{instance}-{a.id}"
                onclick={() => toggleArea(a)}
                class="grid size-8 shrink-0 place-items-center rounded-[5px] text-muted hover:bg-press"
              >
                {#if areaIsExpanded(a)}<ChevronDown size={14} aria-hidden="true" />{:else}<ChevronRight
                    size={14}
                    aria-hidden="true"
                  />{/if}
              </button>
            </div>
          </DropTarget>
          <div id="area-projects-{instance}-{a.id}" class="flex flex-col gap-0.5" hidden={!areaIsExpanded(a)}>
            {#each a.projects.slice(0, 4) as p (p.id)}
              <DropTarget parentId={p.id}>
                <div class="group flex h-8 items-center rounded-[7px] hover:bg-hover">
                  <a
                    href="#/project/{p.id}"
                    onclick={() => (drawer = false)}
                    aria-current={screen.name === 'project' && screen.id === p.id ? 'page' : undefined}
                    class="flex min-w-0 flex-1 items-center rounded-[7px] pr-1 pl-6 text-[0.8438rem] hover:bg-hover aria-[current=page]:bg-press aria-[current=page]:font-semibold"
                  >
                    {#if p.icon}<span aria-hidden="true" class="mr-1.5">{p.icon}</span>{/if}{p.name}
                  </a>
                  <EntityMenu id={p.id} name={p.name} kind="project" fallbackHref="#/home" {onchange} />
                </div>
              </DropTarget>
            {/each}
            {#if a.projects.length > 4}
              <span class="pl-6 text-[0.7812rem] text-muted">+{a.projects.length - 4} more</span>
            {/if}
            <form
              onsubmit={(e) => addProject(e, a.id)}
              class="flex items-center gap-1.5 px-2 pl-6 text-muted focus-within:text-text"
            >
              <Plus size={13} aria-hidden="true" />
              <input
                bind:value={newProject[a.id]}
                placeholder="Add project"
                aria-label="Add project in {a.area.name}"
                class="h-7 w-full bg-transparent text-[0.8125rem] outline-none placeholder:text-muted"
              />
            </form>
          </div>
        </li>
      {/each}
    </ul>
    {#if areas.length === 0}
      <p class="px-2.5 text-[0.8125rem] text-muted">
        No areas yet. Name your first one below, for example Work or Home.
      </p>
      {#if !addingArea}
        <Button variant="ghost" size="sm" class="mx-2 self-start" onclick={beginArea}>
          <Plus size={14} /> Add area
        </Button>
      {/if}
    {/if}
  </section>
{/snippet}

{#snippet nav(instance: string)}
  <div class="flex h-full flex-col gap-4 overflow-y-auto p-3">
    <div class="flex items-center gap-2 px-1.5">
      <img src="{import.meta.env.BASE_URL}icon.svg" alt="" class="size-6" />
      <span class="flex-1 text-[0.9375rem] font-semibold">Orbit</span>
      <Button variant="primary" size="sm" onclick={newTask}>
        <Plus size={14} class="text-accent-ink" /> Add task
      </Button>
    </div>
    {#if favoriteList.length}
      <section class="flex flex-col gap-0.5" aria-label="Favorites">
        <h2 class="px-2.5 pb-1 text-[0.7812rem] font-semibold text-muted">Favorites</h2>
        {#each favoriteList as f (f.id)}
          <a
            href={f.href}
            onclick={() => (drawer = false)}
            class="flex h-8 items-center rounded-[7px] px-2.5 text-[0.8438rem] hover:bg-hover"
          >
            <span class="min-w-0 flex-1 truncate">{f.name}</span>
          </a>
        {/each}
      </section>
    {/if}
    <nav class="flex flex-col gap-0.5" aria-label="Main">
      {@render row('home')}
      {#if !isHidden(VIEW_INBOX)}
        <DropTarget parentId={inbox_id()}>{@render row('inbox', inboxCount)}</DropTarget>
      {/if}
      {#if !isHidden(VIEW_UPCOMING)}
        {@render row('upcoming')}
      {/if}
      {@render row('completed')}
    </nav>
    <nav class="flex flex-col gap-0.5" aria-label="Plans">
      {@render row('goals')}
      {@render row('roadmaps')}
      {@render row('routines')}
    </nav>
    {@render areaTree(instance)}
    <section>
      <div class="flex items-center gap-1 px-2.5 pb-1">
        <h2 class="flex-1 text-[0.7812rem] font-semibold text-muted">Views</h2>
        <Button variant="ghost" size="icon-sm" aria-label="New view" onclick={newView}>
          <Plus size={14} />
        </Button>
      </div>
      <ul
        use:dndzone={{ items: viewOrder, flipDurationMs: 150, delayTouchStart: 200 }}
        onconsider={considerViews}
        onfinalize={finalizeViews}
        class="flex flex-col gap-0.5"
      >
        {#each viewOrder as v (v.id)}
          <li class="group flex h-8 items-center rounded-[7px] hover:bg-hover">
            <a
              href="#/view/{v.id}"
              onclick={() => (drawer = false)}
              aria-current={screen.name === 'view' && screen.id === v.id ? 'page' : undefined}
              class="flex min-w-0 flex-1 items-center rounded-[7px] px-2.5 text-[0.8438rem] hover:bg-hover aria-[current=page]:bg-press aria-[current=page]:font-semibold"
            >
              {v.name}
            </a>
            {#if !v.builtin}
              <EntityMenu id={v.id} name={v.name} kind="view" fallbackHref="#/home" {onchange} />
            {/if}
          </li>
        {/each}
      </ul>
      {#if !viewOrder.length}
        <p class="px-2.5 text-[0.7812rem] text-muted">Pin a view to show it here.</p>
      {/if}
      {#if hiddenViews.length}
        <details class="mt-1 px-2.5">
          <summary class="cursor-pointer text-[0.7812rem] text-muted">Hidden views</summary>
          <ul class="mt-1 flex flex-col gap-0.5">
            {#each hiddenViews as v (v.id)}
              <li class="flex items-center gap-1">
                <a
                  href="#/view/{v.id}"
                  onclick={() => (drawer = false)}
                  class="min-w-0 flex-1 truncate text-[0.8125rem] text-muted hover:text-text"
                >
                  {v.name}
                </a>
                <Button variant="ghost" size="sm" aria-label="Unhide {v.name}" onclick={() => unhide(v.id)}>Show</Button
                >
                {#if !v.builtin}
                  <EntityMenu id={v.id} name={v.name} kind="view" fallbackHref="#/home" {onchange} />
                {/if}
              </li>
            {/each}
          </ul>
        </details>
      {/if}
    </section>
    <nav class="mt-auto flex flex-col gap-0.5" aria-label="Account">
      {@render row('profile')}
      {@render row('trash')}
      {@render row('settings')}
    </nav>
  </div>
{/snippet}

<div class="flex h-full">
  <aside class="hidden w-64 shrink-0 border-r border-line bg-side desk:block">{@render nav('desktop')}</aside>

  <div class="flex min-w-0 flex-1 flex-col">
    <header class="flex h-12 shrink-0 items-center gap-2 px-2 desk:px-4">
      <Button variant="ghost" size="icon" class="desk:hidden" aria-label="Open menu" onclick={() => (drawer = true)}>
        <Menu />
      </Button>
      <div class="flex flex-1 items-center justify-center gap-1 desk:justify-start">
        <nav aria-label="Breadcrumb" class="min-w-0">
          <ol class="flex min-w-0 items-center gap-1 font-medium text-muted">
            {#each crumbs as c, i (i)}
              {#if i}<li aria-hidden="true" class="shrink-0 text-faint">›</li>{/if}
              <li class="min-w-0 truncate">
                {#if c.href}<a href={c.href} class="rounded-sm px-1 hover:bg-hover hover:text-text">{c.label}</a>
                {:else}<span aria-current="page" class="px-1">{c.label}</span>{/if}
              </li>
            {/each}
          </ol>
        </nav>
        {#if screen.name === 'project'}
          <EntityCreateMenu
            ownerId={screen.id}
            ownerName={title}
            ownerKind="project"
            {areas}
            {onchange}
            onnewtask={(text) => open(text, true)}
          />
          <EntityMenu
            id={screen.id}
            name={title}
            kind="project"
            fallbackHref={areas.find((a) => a.projects.some((p) => p.id === screen.id))?.area.id
              ? `#/area/${areas.find((a) => a.projects.some((p) => p.id === screen.id))?.area.id}`
              : '#/home'}
            archiveable
            {onchange}
          />
        {/if}
      </div>
      <Button variant="ghost" class="hidden desk:inline-flex" onclick={newTask}>
        <Plus /> New task <kbd class="rounded-sm bg-sunk px-1.5 text-[0.7188rem] text-muted">Q</kbd>
      </Button>
      <Button variant="ghost" size="icon" aria-label="Search" onclick={openPalette}>
        <Search />
      </Button>
    </header>
    <main class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-[860px] px-4 pt-2 pb-28 desk:px-10 desk:pb-16">{@render children(open)}</div>
    </main>
  </div>
</div>

{#snippet tab(t: Route)}
  {@const Icon = routes[t].icon}
  <a
    href="#/{t}"
    aria-current={screen.name === t ? 'page' : undefined}
    class="group flex flex-1 flex-col items-center justify-center gap-1 text-[0.7188rem] font-medium text-muted aria-[current=page]:text-accent-text"
  >
    <Icon size={20} class="text-icon group-aria-[current=page]:text-accent" />
    {routes[t].label}
  </a>
{/snippet}

<!-- Two tabs each side keep the New task button on the bar's centre line. -->
<nav
  aria-label="Tabs"
  class="fixed inset-x-0 bottom-0 grid h-[calc(4.5rem+env(safe-area-inset-bottom))] grid-cols-[1fr_auto_1fr] border-t border-line bg-surface/92 pb-[env(safe-area-inset-bottom)] backdrop-blur desk:hidden"
>
  <div class="flex">
    {#each tabs.slice(0, tabs.indexOf('add')) as t (t)}{@render tab(t as Route)}{/each}
  </div>
  <div class="grid place-items-center px-3">
    <button
      type="button"
      aria-label="New task"
      onclick={newTask}
      class="grid size-12 place-items-center rounded-[15px] bg-accent text-accent-ink shadow-[0_4px_12px_rgba(0,0,0,.18)]"
    >
      <Plus size={22} />
    </button>
  </div>
  <div class="flex">
    {#each tabs.slice(tabs.indexOf('add') + 1) as t (t)}{@render tab(t as Route)}{/each}
  </div>
</nav>

<Sheet.Root bind:open={drawer}>
  <Sheet.Content side="left" class="w-64 max-w-[85vw] bg-side p-0" showCloseButton={false}>
    <Sheet.Title class="sr-only">Menu</Sheet.Title>
    {@render nav('mobile')}
  </Sheet.Content>
</Sheet.Root>

{#snippet composerHeader()}
  <div class="flex items-center justify-between gap-3">
    <h2 id="new-task-title" class="text-base font-semibold">New task</h2>
    <Button
      variant="ghost"
      size="icon"
      class="!size-[44px]"
      aria-label="Close new task"
      onclick={() => (composing = false)}
    >
      <X size={16} />
    </Button>
  </div>
{/snippet}

<Palette bind:open={palette} {parentId} />
<Shortcuts bind:open={help} />

{#if wide}
  <Dialog.Root bind:open={composing}>
    <Dialog.Content
      class="w-[min(768px,calc(100vw-2rem))] min-w-0"
      aria-labelledby="new-task-title"
      showCloseButton={false}
    >
      {@render composerHeader()}
      <Composer {prefill} {parentId} {forceParent} onsaved={() => (composing = false)} />
    </Dialog.Content>
  </Dialog.Root>
{:else}
  <Sheet.Root bind:open={composing}>
    <Sheet.Content
      side="bottom"
      class="p-4 pb-[calc(1rem+env(safe-area-inset-bottom))]"
      aria-labelledby="new-task-title"
      showCloseButton={false}
    >
      {@render composerHeader()}
      <Composer {prefill} {parentId} {forceParent} onsaved={() => (composing = false)} />
    </Sheet.Content>
  </Sheet.Root>
{/if}
