<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import ListChecks from '@lucide/svelte/icons/list-checks'
  import { taskIsUpcoming } from '$lib/dashboard'
  import { localDay, runView, type Row } from '$lib/sync'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  let {
    viewId,
    version = 0,
    limit = 8,
    futureOnly = false,
    onmeta,
  }: {
    viewId: string
    version?: number
    limit?: number
    futureOnly?: boolean
    onmeta?: (value: string) => void
  } = $props()
  let rows = $state<Row[]>([])
  let loaded = $state(false)

  $effect(() => {
    void version
    const id = viewId
    Promise.all([runView(id), futureOnly ? localDay() : Promise.resolve('')]).then(([r, today]) => {
      rows = r.groups
        .flatMap((g) => g.rows)
        .filter((row) => !futureOnly || taskIsUpcoming(row, today))
        .slice(0, limit)
      loaded = true
      onmeta?.(`${rows.length} upcoming item${rows.length === 1 ? '' : 's'}`)
    })
  })
  const label = (r: Row) => ('title' in r ? r.title : 'name' in r ? r.name : r.kind)
  const due = (r: Row) => {
    const date = r.kind === 'task' ? r.due?.date : ''
    if (!date) return ''
    const [y, m, d] = date.split('-').map(Number)
    return new Date(y, m - 1, d).toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })
  }
</script>

{#if loaded && rows.length === 0}
  <Empty icon={ListChecks} text="This view has no items yet." label="Add a task" href="#/inbox" />
{:else if !loaded}
  <WidgetSkeleton />
{:else}
  <ul class="grid grid-cols-[minmax(0,1fr)] gap-0.5">
    {#each rows as r (r.id)}
      <li class="flex min-h-[2rem] items-center gap-3 px-1">
        <span class="task-title-1 flex-1" title={label(r)}>{label(r)}</span>
        {#if due(r)}<span class="shrink-0 text-xs text-muted">{due(r)}</span>{/if}
      </li>
    {/each}
  </ul>
{/if}
