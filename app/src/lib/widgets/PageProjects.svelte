<script lang="ts">
  import FolderKanban from '@lucide/svelte/icons/folder-kanban'
  import Empty from '$lib/Empty.svelte'
  import { openCounts } from '$lib/area'
  import { visibleSnapshot, type Project } from '$lib/sync'

  /** The Projects of an Area with their open Task counts. */
  let {
    projects,
    version = 0,
    onmeta,
  }: { projects: Project[]; version?: number; onmeta?: (value: string) => void } = $props()
  let counts = $state(new Map<string, number>())
  $effect(() => {
    void version
    const ids = projects.map((p) => p.id)
    visibleSnapshot().then((rows) => (counts = openCounts(rows, ids)))
    onmeta?.(`${ids.length} project${ids.length === 1 ? '' : 's'}`)
  })
</script>

{#if projects.length}
  <ul class="grid gap-2 sm:grid-cols-2">
    {#each projects as p (p.id)}
      <li>
        <a
          href="#/project/{p.id}"
          class="flex items-center gap-2 rounded-xl border border-line bg-surface p-3 hover:bg-hover"
        >
          <span class="min-w-0 flex-1 truncate text-sm font-medium">{p.name}</span>
          <span class="text-xs text-muted">{counts.get(p.id) ?? 0} open</span>
        </a>
      </li>
    {/each}
  </ul>
{:else}
  <Empty
    icon={FolderKanban}
    text="Projects group the tasks for one outcome in this Area. Create one from the + menu or sidebar."
  />
{/if}
