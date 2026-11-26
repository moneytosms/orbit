<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import ListChecks from '@lucide/svelte/icons/list-checks'
  import Empty from '$lib/Empty.svelte'
  import { ui } from '$lib/actions'
  import { completeTask, tasksIn, type Task } from '$lib/sync'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** The open Tasks directly under an Area or Project. */
  let {
    parentId,
    name = '',
    version = 0,
    limit = 20,
    onmeta,
    onchange,
  }: {
    parentId: string
    name?: string
    version?: number
    limit?: number
    onmeta?: (value: string) => void
    onchange?: () => void
  } = $props()
  let tasks = $state<Task[]>([])
  let total = $state(0)
  let loaded = $state(false)
  let openId = $state<string | null>(null)

  async function load() {
    const id = parentId
    const all = (await tasksIn(id)).filter((r) => r.depth === 0).map((r) => r.task)
    if (id !== parentId) return
    total = all.length
    tasks = all.slice(0, limit)
    loaded = true
    onmeta?.(`${total} open task${total === 1 ? '' : 's'}`)
  }
  $effect(() => {
    void version
    void parentId
    load()
  })
  async function done(t: Task) {
    await completeTask(t.id)
    await load()
    onchange?.()
  }
  const due = (t: Task) => {
    if (!t.due) return ''
    const [y, m, d] = t.due.date.split('-').map(Number)
    return new Date(y, m - 1, d).toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })
  }
</script>

{#if !loaded}
  <WidgetSkeleton />
{:else if tasks.length === 0}
  <Empty icon={ListChecks} text="No open tasks here." label="Add a task" onclick={() => ui.capture(`@${name}`)} />
{:else}
  <ul class="grid grid-cols-[minmax(0,1fr)] gap-0.5">
    {#each tasks as t (t.id)}
      <li class="flex min-h-[2.25rem] items-center gap-3 rounded-[9px] px-1 hover:bg-hover">
        <button
          type="button"
          aria-label="Complete {t.title}"
          onclick={() => done(t)}
          class="grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
          ><Check size={12} strokeWidth={3} /></button
        >
        <button type="button" class="task-title-1 flex-1 text-left" title={t.title} onclick={() => (openId = t.id)}
          >{t.title}</button
        >
        {#if due(t)}<span class="shrink-0 text-xs text-muted">{due(t)}</span>{/if}
      </li>
    {/each}
  </ul>
  {#if total > tasks.length}<p class="mt-1 text-xs text-muted">and {total - tasks.length} more in List</p>{/if}
{/if}

<TaskDrawer
  taskId={openId}
  onclose={() => {
    openId = null
    load()
    onchange?.()
  }}
/>
