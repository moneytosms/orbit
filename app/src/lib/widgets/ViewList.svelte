<script lang="ts">
  import { runView, type Row } from '$lib/sync'

  let { viewId, version = 0, limit = 8 }: { viewId: string; version?: number; limit?: number } = $props()
  let rows = $state<Row[]>([])
  let loaded = $state(false)

  $effect(() => {
    void version
    const id = viewId
    runView(id).then((r) => {
      rows = r.groups.flatMap((g) => g.rows).slice(0, limit)
      loaded = true
    })
  })
  const label = (r: Row) => ('title' in r ? r.title : 'name' in r ? r.name : r.kind)
  const due = (r: Row) => (r.kind === 'task' ? (r.due?.date ?? '') : '')
</script>

{#if loaded && rows.length === 0}
  <p class="text-muted">Nothing here.</p>
{/if}
<ul class="grid gap-0.5">
  {#each rows as r (r.id)}
    <li class="flex min-h-[32px] items-center gap-3 px-1">
      <span class="min-w-0 flex-1 truncate">{label(r)}</span>
      {#if due(r)}<span class="shrink-0 text-xs text-muted">{due(r)}</span>{/if}
    </li>
  {/each}
</ul>
