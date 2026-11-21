<script lang="ts">
  import Backlinks from '$lib/Backlinks.svelte'
  import EntityMenu from '$lib/EntityMenu.svelte'
  import Editor from '$lib/Editor.svelte'
  import { editPageBody, getPage, linkTarget, type Page } from '$lib/sync'

  let { id, onchange }: { id: string; onchange: () => void } = $props()

  let page = $state<Page | undefined>()
  let loaded = $state(false)
  // The rev the editor's text started from; it is the base_rev of the next edit.
  let baseRev = 0

  let readonly = $state(false)
  let linkVersion = $state(0)
  let fallbackHref = $state('#/home')

  async function load() {
    page = await getPage(id)
    readonly = (await linkTarget(id))?.state === 'archived'
    const parent = page ? await linkTarget(page.parent_id) : null
    fallbackHref = parent?.state === 'ok' ? (parent.href ?? '#/home') : '#/home'
    baseRev = page?.rev ?? 0
    loaded = true
  }
  async function refresh() {
    await load()
    onchange()
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
    linkVersion++
    onchange()
  }
</script>

{#if page}
  {#key page.id}
    <div class="mb-3 flex items-center gap-1">
      <h1 class="page-title min-w-0 flex-1">{page.title}</h1>
      <EntityMenu id={page.id} name={page.title} kind="page" {fallbackHref} onchange={refresh} />
    </div>
    <Editor value={page.body} label="Page body" owner={page.parent_id} editable={!readonly} onchange={saveBody} />
    <Backlinks id={page.id} version={linkVersion} />
  {/key}
{:else if loaded}
  <p class="text-sm text-muted">This page no longer exists.</p>
{/if}
