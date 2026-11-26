<script lang="ts">
  // Manage the shared Labels: add, rename and delete. A Label outlives the Tasks that use it, so a typo would
  // otherwise stay in the suggestions forever. Deleting also takes the Label off every Task that carries it.
  import Pencil from '@lucide/svelte/icons/pencil'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import Check from '@lucide/svelte/icons/check'
  import X from '@lucide/svelte/icons/x'
  import { onMount } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import * as Dialog from '$lib/components/ui/dialog'
  import { createLabel, deleteLabel, labelUsage, renameLabel } from '$lib/sync'

  type Entry = Awaited<ReturnType<typeof labelUsage>>[number]
  let items = $state<Entry[]>([])
  let adding = $state('')
  let editing = $state<string | null>(null)
  let draft = $state('')
  let error = $state('')
  let removing = $state<Entry | null>(null)

  async function load() {
    items = await labelUsage()
  }
  onMount(load)

  const clean = (name: string) => name.trim().replace(/^#/, '').trim()
  const taken = (name: string, except?: string) =>
    items.some((e) => e.label.id !== except && e.label.name.toLowerCase() === name.toLowerCase())

  async function add(e: SubmitEvent) {
    e.preventDefault()
    const name = clean(adding)
    if (!name) return
    if (taken(name)) return void (error = `#${name} already exists.`)
    error = ''
    await createLabel(name)
    adding = ''
    await load()
  }
  function begin(entry: Entry) {
    editing = entry.label.id
    draft = entry.label.name
    error = ''
  }
  async function save(entry: Entry) {
    const name = clean(draft)
    if (!name || name === entry.label.name) return void (editing = null)
    if (taken(name, entry.label.id)) return void (error = `#${name} already exists.`)
    error = ''
    await renameLabel(entry.label.id, name)
    editing = null
    await load()
  }
  async function confirmRemove() {
    const entry = removing
    removing = null
    if (!entry) return
    await deleteLabel(entry.label.id)
    await load()
  }
</script>

<form class="flex gap-2" onsubmit={add}>
  <Input bind:value={adding} placeholder="Add a label" aria-label="New label" class="min-w-0 flex-1" />
  <Button type="submit" disabled={!clean(adding)}>Add</Button>
</form>
{#if error}<p role="alert" class="mt-2 text-xs text-danger">{error}</p>{/if}

{#if items.length}
  <ul class="mt-3 divide-y divide-line rounded-lg border border-line">
    {#each items as entry (entry.label.id)}
      <li class="flex min-h-11 items-center gap-2 px-3 py-1.5">
        {#if editing === entry.label.id}
          <form
            class="flex min-w-0 flex-1 items-center gap-1.5"
            onsubmit={(e) => {
              e.preventDefault()
              void save(entry)
            }}
          >
            <span class="text-muted">#</span>
            <Input
              bind:value={draft}
              aria-label="Rename {entry.label.name}"
              class="h-8 min-w-0 flex-1"
              autofocus
              onkeydown={(e) => e.key === 'Escape' && (editing = null)}
            />
            <Button type="submit" size="icon-sm" variant="ghost" aria-label="Save name"><Check size={15} /></Button>
            <Button type="button" size="icon-sm" variant="ghost" aria-label="Cancel" onclick={() => (editing = null)}
              ><X size={15} /></Button
            >
          </form>
        {:else}
          <span class="min-w-0 flex-1 truncate">#{entry.label.name}</span>
          <span class="shrink-0 text-xs text-muted">{entry.tasks} {entry.tasks === 1 ? 'task' : 'tasks'}</span>
          <Button size="icon-sm" variant="ghost" aria-label="Rename #{entry.label.name}" onclick={() => begin(entry)}
            ><Pencil size={15} /></Button
          >
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label="Delete #{entry.label.name}"
            onclick={() => (removing = entry)}><Trash2 size={15} /></Button
          >
        {/if}
      </li>
    {/each}
  </ul>
{:else}
  <p class="mt-3 text-sm text-muted">No labels yet. Type #name in a task to make one, or add one above.</p>
{/if}

<Dialog.Root open={removing !== null} onOpenChange={(o) => !o && (removing = null)}>
  <Dialog.Content class="w-[min(26rem,calc(100vw-2rem))] min-w-0" showCloseButton={false}>
    {#if removing}
      <Dialog.Title class="text-base font-semibold">Delete #{removing.label.name}?</Dialog.Title>
      <p class="text-sm text-muted">
        {removing.tasks
          ? `It will be taken off ${removing.tasks} open ${removing.tasks === 1 ? 'task' : 'tasks'}.`
          : 'No open tasks use it.'} The tasks themselves stay.
      </p>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (removing = null)}>Cancel</Button>
        <Button variant="destructive" onclick={confirmRemove}>Delete</Button>
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
