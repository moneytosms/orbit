<script lang="ts">
  import Editor from '$lib/Editor.svelte'
  import { Button } from '$lib/components/ui/button'
  import { deleteItem, editPageBody, getPage, renamePage, type Page } from '$lib/sync'

  let { id, onchange }: { id: string; onchange: () => void } = $props()

  let page = $state<Page | undefined>()
  let loaded = $state(false)
  // The rev the editor's text started from; it is the base_rev of the next edit.
  let baseRev = 0

  async function load() {
    page = await getPage(id)
    baseRev = page?.rev ?? 0
    loaded = true
  }
  $effect(() => {
    void id
    load()
  })

  async function saveBody(md: string) {
    if (!page) return
    await editPageBody(page.id, md, baseRev)
    // Re-read so the next edit is based on the rev this one produced.
    const next = await getPage(page.id)
    if (next) {
      baseRev = next.rev
      page = { ...page, rev: next.rev, title: next.title }
    }
    onchange()
  }

  async function remove() {
    if (!page) return
    await deleteItem(page.id)
    onchange()
    location.hash = '#/inbox'
  }

  async function saveTitle(e: FocusEvent) {
    const input = e.currentTarget as HTMLInputElement
    const value = input.value.trim()
    if (!page) return
    if (!value) input.value = page.title
    else if (value !== page.title) {
      await renamePage(page.id, value)
      page = { ...page, title: value }
      onchange()
    }
  }
</script>

{#if page}
  {#key page.id}
    <input
      aria-label="Page title"
      value={page.title}
      onblur={saveTitle}
      onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
      class="page-title mb-3 w-full bg-transparent outline-none"
    />
    <Editor value={page.body} label="Page body" onchange={saveBody} />
    <Button variant="ghost" aria-label="Delete page" class="mt-3" onclick={remove}>Delete page</Button>
  {/key}
{:else if loaded}
  <p class="text-sm text-muted">This page no longer exists.</p>
{/if}
