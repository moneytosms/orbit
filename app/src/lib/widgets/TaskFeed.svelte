<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import Empty from '$lib/Empty.svelte'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import { moveAll, localToday } from '$lib/postpone'
  import { completeTask, localDay, visibleSnapshot, type Task } from '$lib/sync'
  import { dueToday, overdueTasks, recentlyDone } from '$lib/widgetData'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** Tasks due today, overdue, or recently done; with `parentId` only those under that Area or Project. */
  let {
    mode,
    parentId,
    version = 0,
    onchange,
    onmeta,
  }: {
    mode: 'today' | 'overdue' | 'done'
    parentId?: string
    version?: number
    onchange?: () => void
    onmeta?: (value: string) => void
  } = $props()
  let tasks = $state<Task[]>([])
  let loaded = $state(false)
  let openId = $state<string | null>(null)

  const noun = (n: number) => `${n} task${n === 1 ? '' : 's'}`
  async function load() {
    const [rows, today] = await Promise.all([visibleSnapshot(), localDay()])
    const all =
      mode === 'today'
        ? dueToday(rows, today, parentId)
        : mode === 'overdue'
          ? overdueTasks(rows, today, parentId)
          : recentlyDone(rows, 8, parentId)
    tasks = mode === 'done' ? all : all.slice(0, 20)
    loaded = true
    onmeta?.(
      mode === 'done'
        ? `Last ${noun(all.length)} finished`
        : `${noun(all.length)} ${mode === 'today' ? 'due today' : 'overdue'}`,
    )
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
  async function toToday(list: Task[]) {
    await moveAll(list, localToday(), 'moved to today')
    await load()
    onchange?.()
  }
  const day = (ms: number | null) =>
    ms === null ? '' : new Date(ms).toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })
  const due = (t: Task) => {
    if (!t.due) return ''
    const [y, m, d] = t.due.date.split('-').map(Number)
    return new Date(y, m - 1, d).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
  }
  const empty = $derived(
    mode === 'today'
      ? 'Nothing due today.'
      : mode === 'overdue'
        ? 'Nothing overdue. Nice.'
        : 'Tasks you finish show up here.',
  )
</script>

{#if !loaded}
  <WidgetSkeleton />
{:else if tasks.length === 0}
  <Empty icon={CircleCheck} text={empty} />
{:else}
  <ul class="grid grid-cols-[minmax(0,1fr)] gap-0.5">
    {#each tasks as t (t.id)}
      <li class="flex min-h-[2.25rem] items-center gap-3 rounded-[9px] px-1 hover:bg-hover">
        {#if mode === 'done'}
          <span class="grid size-[1.125rem] shrink-0 place-items-center rounded-full bg-ok text-bg"
            ><Check size={12} strokeWidth={3} /></span
          >
        {:else}
          <button
            type="button"
            aria-label="Complete {t.title}"
            onclick={() => done(t)}
            class="grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
            ><Check size={12} strokeWidth={3} /></button
          >
        {/if}
        <button type="button" class="task-title-1 flex-1 text-left" title={t.title} onclick={() => (openId = t.id)}
          >{t.title}</button
        >
        {#if mode === 'done'}
          <span class="shrink-0 text-xs text-muted">{day(t.completed_at)}</span>
        {:else if mode === 'overdue'}
          <span class="shrink-0 text-xs text-muted">{due(t)}</span>
          <button
            type="button"
            class="shrink-0 rounded-full px-2 py-px text-xs text-accent-text hover:bg-accent-soft"
            aria-label="Move {t.title} to today"
            onclick={() => toToday([t])}>Today</button
          >
        {/if}
      </li>
    {/each}
  </ul>
  {#if mode === 'overdue' && tasks.length > 1}
    <button type="button" class="mt-1 text-xs text-muted hover:text-text" onclick={() => toToday(tasks)}
      >Move all to today</button
    >
  {/if}
{/if}

<TaskDrawer
  taskId={openId}
  onclose={() => {
    openId = null
    load()
    onchange?.()
  }}
/>
