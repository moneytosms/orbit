<script lang="ts">
  import FileText from '@lucide/svelte/icons/file-text'
  import Empty from '$lib/Empty.svelte'
  import { createPage, pagesOf, type Page } from '$lib/sync'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** The Pages (notes) filed under an Area or Project. */
  let {
    parentId,
    version = 0,
    onmeta,
    onchange,
  }: { parentId: string; version?: number; onmeta?: (value: string) => void; onchange?: () => void } = $props()
  let list = $state<Page[]>([])
  let loaded = $state(false)

  $effect(() => {
    void version
    const id = parentId
    pagesOf(id).then((p) => {
      if (id !== parentId) return
      list = p
      loaded = true
      onmeta?.(`${p.length} page${p.length === 1 ? '' : 's'}`)
    })
  })
  async function newPage() {
    const id = await createPage(parentId, 'Untitled')
    onchange?.()
    location.hash = `#/page/${id}`
  }
</script>

{#if !loaded}
  <WidgetSkeleton />
{:else if list.length === 0}
  <Empty icon={FileText} text="Pages hold notes and docs." label="New page" onclick={newPage} />
{:else}
  <ul class="grid gap-0.5">
    {#each list as p (p.id)}
      <li>
        <a href="#/page/{p.id}" class="task-title-1 block min-h-[2rem] py-1 text-sm hover:underline">{p.title}</a>
      </li>
    {/each}
  </ul>
  <button type="button" class="mt-2 self-start text-xs text-muted hover:text-text" onclick={newPage}>New page</button>
{/if}
