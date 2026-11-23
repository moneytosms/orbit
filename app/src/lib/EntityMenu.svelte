<script lang="ts">
  import MoreHorizontal from '@lucide/svelte/icons/ellipsis'
  import { DropdownMenu } from 'bits-ui'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import {
    archiveItem,
    deleteItem,
    deleteSection,
    renamePage,
    renameRoadmap,
    rowsSnapshot,
    updateArea,
    updateGoal,
    updateProject,
    updateSection,
    updateView,
  } from '$lib/sync'

  type Kind = 'area' | 'project' | 'view' | 'goal' | 'roadmap' | 'page' | 'section'
  let {
    id,
    name,
    kind,
    fallbackHref,
    onchange,
    archiveable = false,
  }: {
    id: string
    name: string
    kind: Kind
    fallbackHref: string
    onchange: () => void
    archiveable?: boolean
  } = $props()

  let open = $state(false)
  let mode = $state<'rename' | 'delete'>('rename')
  let value = $state('')
  let trigger = $state<HTMLButtonElement | null>(null)

  $effect(() => {
    value = name
  })

  function showDialog(nextMode: 'rename' | 'delete') {
    mode = nextMode
    if (nextMode === 'rename') value = name
    open = true
  }

  async function routeIsAffected() {
    const match = location.hash.match(/^#\/(area|project|page|view|roadmap|task)\/([^/?#]+)/)
    if (!match) return false
    let currentId = decodeURIComponent(match[2])
    if (currentId === id) return true
    const rows = await rowsSnapshot()
    const byId = new Map(rows.map((row) => [row.id, row]))
    for (let depth = 0; depth < 32; depth++) {
      const row = byId.get(currentId)
      if (!row || !('parent_id' in row) || !row.parent_id) return false
      currentId = row.parent_id
      if (currentId === id) return true
    }
    return false
  }

  async function rename() {
    const next = value.trim()
    if (!next || next === name) {
      value = name
      open = false
      return
    }
    if (kind === 'area') await updateArea(id, { name: next })
    else if (kind === 'project') await updateProject(id, { name: next })
    else if (kind === 'goal') await updateGoal(id, { name: next })
    else if (kind === 'roadmap') await renameRoadmap(id, next)
    else if (kind === 'page') await renamePage(id, next)
    else if (kind === 'section') await updateSection(id, { name: next })
    else await updateView(id, { name: next })
    value = next
    open = false
    onchange()
  }

  async function remove() {
    const navigate = await routeIsAffected()
    if (kind === 'section') await deleteSection(id)
    else await deleteItem(id)
    open = false
    onchange()
    if (navigate) location.hash = fallbackHref
  }

  async function archive() {
    const navigate = await routeIsAffected()
    await archiveItem(id)
    onchange()
    if (navigate) location.hash = fallbackHref
  }
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger
    bind:ref={trigger}
    type="button"
    aria-label="{name} actions"
    class="grid size-8 shrink-0 place-items-center rounded-md text-muted hover:bg-hover hover:text-text focus-visible:outline-2 focus-visible:outline-accent"
  >
    <MoreHorizontal size={18} aria-hidden="true" />
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content align="end" class="z-50 min-w-36 rounded-lg border border-line bg-surface p-1 shadow-2">
      <DropdownMenu.Item
        class="cursor-pointer rounded-md px-2.5 py-1.5 text-sm outline-none focus:bg-hover"
        onSelect={() => showDialog('rename')}>Rename</DropdownMenu.Item
      >
      {#if archiveable}
        <DropdownMenu.Item
          class="cursor-pointer rounded-md px-2.5 py-1.5 text-sm outline-none focus:bg-hover"
          onSelect={archive}>Archive</DropdownMenu.Item
        >
      {/if}
      <DropdownMenu.Item
        class="cursor-pointer rounded-md px-2.5 py-1.5 text-sm text-danger outline-none focus:bg-danger-soft"
        onSelect={() => showDialog('delete')}>Delete</DropdownMenu.Item
      >
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>

<Dialog.Root bind:open>
  <Dialog.Content
    class="!min-w-0 w-[min(384px,calc(100vw-2rem))]"
    onCloseAutoFocus={(event) => {
      event.preventDefault()
      if (trigger?.isConnected) trigger.focus()
    }}
  >
    <Dialog.Title>{mode === 'rename' ? `Rename ${kind}` : `Delete ${kind}?`}</Dialog.Title>
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
      <p class="mt-2 text-sm text-muted">
        {#if kind === 'section'}Tasks in “{name}” will move to its parent. The Section will move to Trash.{:else}“{name}”
          will move to Trash. You can restore it from there.{/if}
      </p>
      <div class="mt-4 flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
        <Button variant="destructive" onclick={remove}>Delete</Button>
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
