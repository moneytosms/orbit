<script lang="ts">
  // The overdue summary at the top of a task list: one bulk move for every overdue Task in it, a list to look over,
  // and a one-tap reschedule per row. Every move can be taken back from the toast.
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import ChevronUp from '@lucide/svelte/icons/chevron-up'
  import { Button } from '$lib/components/ui/button'
  import MonthCalendar from '$lib/MonthCalendar.svelte'
  import { addDays, overdueLabel, overdueTasks, planMoves } from '$lib/overdue'
  import { applyMoves, localToday, moveAll, projectOccurrences } from '$lib/postpone'
  import type { Due, Task } from '$lib/sync'

  let { tasks, onchange }: { tasks: Task[]; onchange: () => void } = $props()

  const uid = $props.id()
  const listId = `${uid}-list`
  const pickId = `${uid}-pick`
  let expanded = $state(false)
  let picking = $state(false)
  let busy = $state(false)
  let error = $state('')

  const overdue = $derived(overdueTasks(tasks, localToday()))

  // "Tue 24 Nov", from a floating local date.
  const dayLabel = (date: string) => {
    const [y, m, d] = date.split('-').map(Number)
    return new Date(y, m - 1, d).toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' })
  }
  const dueLabel = (due: Due | null) =>
    due ? (due.time ? `${dayLabel(due.date)} ${due.time}` : dayLabel(due.date)) : ''

  async function bulk(target: string, when: string) {
    busy = true
    error = ''
    try {
      await moveAll(overdue, target, when)
      picking = false
      onchange()
    } catch {
      error = 'Could not move those tasks. Try again.'
    } finally {
      busy = false
    }
  }

  function pick(date: string) {
    if (date < localToday()) {
      error = 'Pick today or a later day.'
      return
    }
    void bulk(date, `moved to ${dayLabel(date)}`)
  }

  async function one(task: Task) {
    busy = true
    error = ''
    try {
      await applyMoves(planMoves([task], localToday(), projectOccurrences), `"${task.title}" moved to today`)
      onchange()
    } catch {
      error = 'Could not move that task. Try again.'
    } finally {
      busy = false
    }
  }
</script>

{#if overdue.length}
  <section aria-label="Overdue tasks" class="mb-4 rounded-xl border border-danger/40 bg-danger-soft/60 px-3 py-2.5">
    <div class="flex flex-wrap items-center gap-2">
      <p class="mr-auto text-sm font-semibold text-danger-text" role="status">{overdueLabel(overdue.length)}</p>
      <Button size="sm" variant="default" disabled={busy} onclick={() => bulk(localToday(), 'moved to today')}
        >Reschedule to today</Button
      >
      <Button
        size="sm"
        variant="default"
        disabled={busy}
        onclick={() => bulk(addDays(localToday(), 1), 'moved to tomorrow')}>Tomorrow</Button
      >
      <Button
        size="sm"
        variant="default"
        disabled={busy}
        aria-expanded={picking}
        aria-controls={pickId}
        onclick={() => (picking = !picking)}>Pick date…</Button
      >
      <Button
        size="sm"
        variant="ghost"
        aria-expanded={expanded}
        aria-controls={listId}
        onclick={() => (expanded = !expanded)}
      >
        {expanded ? 'Hide list' : 'Show list'}
        {#if expanded}<ChevronUp size={14} aria-hidden="true" />{:else}<ChevronDown size={14} aria-hidden="true" />{/if}
      </Button>
    </div>

    {#if picking}
      <div id={pickId} class="mt-3 rounded-lg bg-surface p-3 shadow-1">
        <MonthCalendar initial={localToday()} onpick={pick} />
      </div>
    {/if}

    {#if expanded}
      <ul id={listId} class="mt-3 divide-y divide-line overflow-hidden rounded-lg bg-surface">
        {#each overdue as task (task.id)}
          <li class="flex items-center gap-3 px-3 py-2">
            <span class="min-w-0 flex-1 truncate text-sm text-text" title={task.title}>{task.title}</span>
            <span class="shrink-0 text-xs text-danger-text">{dueLabel(task.due)}</span>
            <Button
              size="sm"
              variant="ghost"
              disabled={busy}
              aria-label="Reschedule {task.title} to today"
              onclick={() => one(task)}>Reschedule</Button
            >
          </li>
        {/each}
      </ul>
    {/if}

    {#if error}
      <p role="alert" class="mt-2 text-sm text-danger-text">{error}</p>
    {/if}
  </section>
{/if}
