<script lang="ts">
  import MoreHorizontal from '@lucide/svelte/icons/ellipsis'
  import Plus from '@lucide/svelte/icons/plus'
  import type { Snippet } from 'svelte'
  import { DropdownMenu } from 'bits-ui'
  import { ui } from '$lib/actions'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { favorites, toggleFavorite } from '$lib/favorites'
  import { archiveItem, deleteItem, updateArea, updateProject } from '$lib/sync'

  type Entity = { id: string; kind: 'area' | 'project'; name: string; fallbackHref: string; archiveable?: boolean }
  let {
    title,
    crumbs = [],
    newTaskText,
    entity,
    onchange,
    onedit,
    onreset,
    tabs,
    tab,
    ontab,
    actions,
  }: {
    title: string
    /** Muted trail above the title, such as the Area of a Project. */
    crumbs?: { label: string; href: string }[]
    /** Text typed into the capture window by the primary + Task button, e.g. `@Health`; no button without it. */
    newTaskText?: string
    /** The Area or Project the page is about: adds favourite, rename, archive and delete to the menu. */
    entity?: Entity
    onchange?: () => void
    /** Menu: Edit layout / Reset layout. */
    onedit?: () => void
    onreset?: () => void
    tabs?: { value: string; label: string }[]
    tab?: string
    ontab?: (value: string) => void
    /** Extra controls between + Task and the menu. */
    actions?: Snippet
  } = $props()

  let open = $state(false)
  let mode = $state<'rename' | 'delete'>('rename')
  let value = $state('')
  let trigger = $state<HTMLButtonElement | null>(null)

  function showDialog(next: 'rename' | 'delete') {
    mode = next
    value = entity?.name ?? ''
    open = true
  }
  async function rename() {
    const next = value.trim()
    if (!entity || !next || next === entity.name) {
      open = false
      return
    }
    if (entity.kind === 'area') await updateArea(entity.id, { name: next })
    else await updateProject(entity.id, { name: next })
    open = false
    onchange?.()
  }
  async function remove() {
    if (!entity) return
    await deleteItem(entity.id)
    open = false
    onchange?.()
    location.hash = entity.fallbackHref
  }
  async function archive() {
    if (!entity) return
    await archiveItem(entity.id)
    onchange?.()
    location.hash = entity.fallbackHref
  }

  const item = 'cursor-pointer rounded-md px-2.5 py-1.5 text-sm outline-none focus:bg-hover'
  const hasMenu = $derived(!!entity || !!onedit || !!onreset)

  function tabKey(e: KeyboardEvent, i: number) {
    if (!tabs || (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft')) return
    e.preventDefault()
    const next = tabs[(i + (e.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length]
    ontab?.(next.value)
    queueMicrotask(() => document.getElementById(`tab-${next.value}`)?.focus())
  }
</script>

<header class="mb-4">
  {#if crumbs.length}
    <nav aria-label="Breadcrumb" class="mb-1 flex items-center gap-1 text-xs text-muted">
      {#each crumbs as c, i (c.href)}
        {#if i > 0}<span aria-hidden="true">›</span>{/if}
        <a href={c.href} class="hover:text-text hover:underline">{c.label}</a>
      {/each}
      <span aria-hidden="true">›</span>
      <span aria-current="page">{title}</span>
    </nav>
  {/if}
  <div class="flex items-center gap-2">
    <h1 class="page-title mb-0 min-w-0 flex-1 truncate">{title}</h1>
    {#if newTaskText}
      <Button variant="primary" size="sm" onclick={() => ui.capture(newTaskText)}><Plus size={15} /> Task</Button>
    {/if}
    {@render actions?.()}
    {#if hasMenu}
      <DropdownMenu.Root>
        <DropdownMenu.Trigger
          bind:ref={trigger}
          type="button"
          data-tour="page-menu"
          aria-label="{title} actions"
          class="grid size-8 shrink-0 place-items-center rounded-md text-muted hover:bg-hover hover:text-text focus-visible:outline-2 focus-visible:outline-accent"
        >
          <MoreHorizontal size={18} aria-hidden="true" />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content align="end" class="z-50 min-w-40 rounded-lg border border-line bg-surface p-1 shadow-2">
            {#if entity}
              <DropdownMenu.Item class={item} onSelect={() => toggleFavorite(entity.id)}
                >{$favorites.includes(entity.id) ? 'Remove from favorites' : 'Add to favorites'}</DropdownMenu.Item
              >
              <DropdownMenu.Item class={item} onSelect={() => showDialog('rename')}>Rename</DropdownMenu.Item>
              {#if entity.archiveable}
                <DropdownMenu.Item class={item} onSelect={archive}>Archive</DropdownMenu.Item>
              {/if}
              <DropdownMenu.Item class="{item} text-danger focus:bg-danger-soft" onSelect={() => showDialog('delete')}
                >Delete</DropdownMenu.Item
              >
            {/if}
            {#if entity && (onedit || onreset)}
              <DropdownMenu.Separator class="my-1 h-px bg-line" />
            {/if}
            {#if onedit}
              <DropdownMenu.Item class={item} onSelect={onedit}>Edit layout</DropdownMenu.Item>
            {/if}
            {#if onreset}
              <DropdownMenu.Item class={item} onSelect={onreset}>Reset layout</DropdownMenu.Item>
            {/if}
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
    {/if}
  </div>

  {#if tabs}
    <div role="tablist" aria-label="{title} views" class="mt-3 flex gap-4 border-b border-line">
      {#each tabs as t, i (t.value)}
        <button
          type="button"
          role="tab"
          id="tab-{t.value}"
          aria-selected={tab === t.value}
          tabindex={tab === t.value ? 0 : -1}
          onclick={() => ontab?.(t.value)}
          onkeydown={(e) => tabKey(e, i)}
          class="-mb-px h-9 border-b-2 text-sm focus-visible:outline-2 focus-visible:outline-accent {tab === t.value
            ? 'border-accent font-medium text-text'
            : 'border-transparent text-muted hover:text-text'}">{t.label}</button
        >
      {/each}
    </div>
  {/if}
</header>

{#if entity}
  <Dialog.Root bind:open>
    <Dialog.Content
      class="!min-w-0 w-[min(384px,calc(100vw-2rem))]"
      onCloseAutoFocus={(event) => {
        event.preventDefault()
        if (trigger?.isConnected) trigger.focus()
      }}
    >
      <Dialog.Title>{mode === 'rename' ? `Rename ${entity.kind}` : `Delete ${entity.kind}?`}</Dialog.Title>
      {#if mode === 'rename'}
        <label class="mt-3 grid gap-1 text-sm">
          Name
          <input
            bind:value
            aria-label="New name"
            onkeydown={(e) => e.key === 'Enter' && rename()}
            class="h-9 rounded-md border border-line bg-surface px-2.5 outline-none focus-visible:border-accent-line"
          />
        </label>
        <div class="mt-4 flex justify-end gap-2">
          <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
          <Button variant="primary" disabled={!value.trim()} onclick={rename}>Save</Button>
        </div>
      {:else}
        <p class="mt-2 text-sm text-muted">“{entity.name}” will move to Trash. You can restore it from there.</p>
        <div class="mt-4 flex justify-end gap-2">
          <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
          <Button variant="destructive" onclick={remove}>Delete</Button>
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Root>
{/if}
