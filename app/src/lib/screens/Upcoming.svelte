<script lang="ts">
  // The Upcoming screen: a 7-day strip from today, the selected day's agenda, and Later for everything beyond.
  import { tick } from 'svelte'
  import CalendarDays from '@lucide/svelte/icons/calendar-days'
  import Check from '@lucide/svelte/icons/check'
  import ChevronLeft from '@lucide/svelte/icons/chevron-left'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import { Button } from '$lib/components/ui/button'
  import Empty from '$lib/Empty.svelte'
  import OverdueBanner from '$lib/OverdueBanner.svelte'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import { ui } from '$lib/actions'
  import {
    LATER_LIMIT,
    buildStrip,
    bucketByDay,
    hasPrevWeek,
    laterGroups,
    limitGroups,
    moveSelection,
    shiftWeek,
  } from '$lib/upcoming'
  import { completeTask, localDay, nameIndex, rowsSnapshot, type Due, type Task } from '$lib/sync'

  let { areas, onchange }: { areas: unknown; onchange: () => void } = $props()

  let today = $state('')
  let selected = $state('')
  let tasks = $state<Task[]>([])
  let names = $state(new Map<string, { kind: string; name: string; parent_id: string | null }>())
  let loaded = $state(false)
  let showAllLater = $state(false)
  let openId = $state<string | null>(null)
  let stripEl = $state<HTMLElement | undefined>()

  // Reloaded whenever the shell refreshes (`areas` is a fresh array after every sync).
  $effect(() => {
    void areas
    load()
  })
  async function load() {
    const [day, rows, index] = await Promise.all([localDay(), rowsSnapshot(), nameIndex()])
    today = day
    if (!selected || selected < day) selected = day
    tasks = rows.filter((r): r is Task => r.kind === 'task')
    names = index
    loaded = true
  }

  const strip = $derived(today ? buildStrip(today, selected, tasks) : [])
  const dayTasks = $derived(today ? (bucketByDay(tasks, [selected]).get(selected) ?? []) : [])
  const later = $derived(today ? laterGroups(tasks, today) : [])
  const laterCount = $derived(later.reduce((n, g) => n + g.tasks.length, 0))
  const laterShown = $derived(showAllLater ? later : limitGroups(later, LATER_LIMIT))

  const dateOf = (date: string) => {
    const [y, m, d] = date.split('-').map(Number)
    return new Date(y, m - 1, d)
  }
  const dayLabel = (date: string) =>
    dateOf(date).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'short' })
  const heading = (date: string) => (date === today ? `Today, ${dayLabel(date)}` : dayLabel(date))
  const shortDay = (date: string) =>
    dateOf(date).toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' })
  const rangeLabel = $derived(
    strip.length ? `${shortDay(strip[0].date)} to ${shortDay(strip[strip.length - 1].date)}` : '',
  )
  const timeOf = (due: Due | null) => due?.time ?? ''

  // Where a Task lives: its nearest Project or Area.
  function place(task: Task): string {
    let id: string | null = task.parent_id
    for (let i = 0; id && i < 6; i++) {
      const n = names.get(id)
      if (!n) return ''
      if (n.kind === 'project' || n.kind === 'area') return n.name
      id = n.parent_id
    }
    return ''
  }

  async function select(date: string) {
    selected = date
    await tick()
    stripEl?.querySelector<HTMLElement>('[aria-pressed="true"]')?.focus()
  }
  function onkeydown(e: KeyboardEvent) {
    const delta = e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : 0
    if (!delta || !today) return
    e.preventDefault()
    select(moveSelection(today, selected, delta))
  }
  function closeDrawer() {
    openId = null
    onchange()
    load()
  }
  async function complete(task: Task) {
    await completeTask(task.id)
    onchange()
  }

  const flag: Record<number, string> = { 1: 'text-danger', 2: 'text-warn', 3: 'text-accent' }
</script>

{#snippet line(task: Task)}
  <div
    data-task-row={task.id}
    class="group relative flex min-h-[2.625rem] min-w-0 items-center gap-3 rounded-[9px] px-2 hover:bg-hover data-[cursor=true]:bg-hover data-[cursor=true]:ring-2 data-[cursor=true]:ring-accent"
  >
    <button
      type="button"
      aria-label="Complete {task.title}"
      onclick={() => complete(task)}
      class="tap-44 peer grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
    >
      <Check size={12} strokeWidth={3} />
    </button>
    <button
      type="button"
      data-open
      onclick={() => (openId = task.id)}
      class="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-0.5 py-2 text-left"
    >
      <span class="task-title-3 flex-1" title={task.title}>{task.title}</span>
      {#if task.priority <= 3}
        <span class="text-xs font-semibold {flag[task.priority]}" aria-label="Priority {task.priority}"
          >P{task.priority}</span
        >
      {/if}
      {#if timeOf(task.due)}<span class="text-xs text-muted">{timeOf(task.due)}</span>{/if}
      {#if task.recurrence}
        <span class="text-xs text-muted" aria-label="Repeats {task.recurrence.rule}">↻</span>
      {/if}
      {#if place(task)}<span class="text-xs text-faint">{place(task)}</span>{/if}
    </button>
    <span class="absolute right-2 bottom-0 left-9 h-px bg-line group-hover:hidden"></span>
  </div>
{/snippet}

<div class="mb-2 flex items-center gap-2">
  <h1 class="page-title flex-1">Upcoming</h1>
</div>

<OverdueBanner {tasks} {onchange} />

{#if loaded}
  <div class="mb-3 flex items-center gap-2">
    <Button
      variant="ghost"
      size="icon"
      aria-label="Previous week"
      disabled={!hasPrevWeek(today, selected)}
      onclick={() => select(shiftWeek(today, selected, -1))}><ChevronLeft size={16} /></Button
    >
    <p class="flex-1 text-center text-sm text-muted" aria-live="polite">{rangeLabel}</p>
    <Button size="sm" aria-label="Go to today" disabled={selected === today} onclick={() => select(today)}>Today</Button
    >
    <Button variant="ghost" size="icon" aria-label="Next week" onclick={() => select(shiftWeek(today, selected, 1))}
      ><ChevronRight size={16} /></Button
    >
  </div>

  <!-- svelte-ignore a11y_interactive_supports_focus (focus lives on the day buttons; arrows bubble up here) -->
  <div bind:this={stripEl} role="toolbar" aria-label="Week" {onkeydown} class="mb-5 grid grid-cols-7 gap-1 sm:gap-2">
    {#each strip as d (d.date)}
      {@const on = d.date === selected}
      <button
        type="button"
        aria-pressed={on}
        aria-label="{dayLabel(d.date)}{d.today ? ', today' : ''}, {d.count} {d.count === 1 ? 'task' : 'tasks'}"
        onclick={() => (selected = d.date)}
        class="flex min-w-0 flex-col items-center gap-0.5 rounded-lg border px-1 py-2 outline-none focus-visible:ring-2 focus-visible:ring-accent {on
          ? 'border-accent bg-accent text-accent-ink'
          : d.today
            ? 'border-accent-line bg-accent-soft text-accent-text hover:border-accent'
            : 'border-line bg-surface text-text hover:bg-hover'}"
      >
        <span class="text-xs {on ? '' : d.today ? 'font-semibold' : 'text-muted'}">{d.weekday}</span>
        <span class="text-lg leading-6 {d.today ? 'font-bold' : 'font-medium'}">{d.day}</span>
        <span
          aria-hidden="true"
          class="size-1.5 rounded-full {d.count ? (on ? 'bg-accent-ink' : 'bg-accent') : 'bg-transparent'}"
        ></span>
      </button>
    {/each}
  </div>

  <section aria-label="Agenda for {dayLabel(selected)}">
    <h2 class="mb-1 px-2 py-1 text-sm font-semibold text-muted">
      {heading(selected)}{dayTasks.length ? ` · ${dayTasks.length}` : ''}
    </h2>
    {#if dayTasks.length}
      <ul>
        {#each dayTasks as t (t.id)}
          <li>{@render line(t)}</li>
        {/each}
      </ul>
    {:else}
      <Empty
        icon={CalendarDays}
        text="Nothing planned for this day."
        label="Add a task"
        onclick={() => ui.capture(selected)}
      />
    {/if}
  </section>

  {#if later.length}
    <section class="mt-6" aria-label="Later">
      <h2 class="mb-1 px-2 py-1 text-sm font-semibold text-muted">Later · {laterCount}</h2>
      {#each laterShown as g (g.date)}
        <h3 class="mt-2 px-2 text-xs font-medium text-faint">{shortDay(g.date)}</h3>
        <ul>
          {#each g.tasks as t (t.id)}
            <li>{@render line(t)}</li>
          {/each}
        </ul>
      {/each}
      {#if laterCount > LATER_LIMIT}
        <div class="mt-2 px-2">
          <Button variant="ghost" size="sm" aria-expanded={showAllLater} onclick={() => (showAllLater = !showAllLater)}
            >{showAllLater ? 'Show fewer' : `Show all ${laterCount}`}</Button
          >
        </div>
      {/if}
    </section>
  {/if}
{/if}

<TaskDrawer taskId={openId} onclose={closeDrawer} />
