<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import Inbox from '@lucide/svelte/icons/inbox'
  import { dndzone, TRIGGERS, type DndEvent } from 'svelte-dnd-action'
  import Flag from '@lucide/svelte/icons/flag'
  import { Button } from '$lib/components/ui/button'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import Composer from '$lib/Composer.svelte'
  import { completeTask, labels, moveItem, orderBetween, type Due, type Task } from '$lib/sync'
  import { inbox_id } from '../../../wasm/orbit_wasm'

  type Entry = { task: Task; depth: number }
  type Group = { id: string; task: Task; subs: Entry[] }

  let {
    parentId,
    title,
    rows,
    onchange,
    onadd,
  }: {
    parentId: string
    title: string
    rows: Entry[]
    onchange: () => void
    onadd?: (prefill: string) => void
  } = $props()
  let openId = $state<string | null>(null)
  let names = $state(new Map<string, string>())

  // Label names for the chips; reloaded whenever the rows change.
  $effect(() => {
    void rows
    labels().then((all) => (names = new Map(all.map((l) => [l.id, l.name]))))
  })

  const flagClass: Record<number, string> = { 1: 'text-danger', 2: 'text-warn', 3: 'text-accent' }
  const pad = (n: number) => String(n).padStart(2, '0')
  const today = () => {
    const d = new Date()
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
  }
  // A floating local date, shown like "Tue 25 Nov", plus its time.
  function dueText(due: Due) {
    const [y, m, d] = due.date.split('-').map(Number)
    const day = new Date(y, m - 1, d).toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' })
    return due.time ? `${day} ${due.time}` : day
  }
  function closeDrawer() {
    openId = null
    onchange()
  }

  // Top-level Tasks are the draggable units; each carries its open subtasks.
  const groups = $derived.by(() => {
    const out: Group[] = []
    for (const r of rows) {
      if (r.depth === 0) out.push({ id: r.task.id, task: r.task, subs: [] })
      else out.at(-1)?.subs.push(r)
    }
    return out
  })
  let items = $derived(groups)

  function consider(e: CustomEvent<DndEvent<Group>>) {
    items = e.detail.items
  }
  // A drop takes the order between its new neighbours.
  function finalize(e: CustomEvent<DndEvent<Group>>) {
    items = e.detail.items
    const { id, trigger } = e.detail.info
    if (trigger !== TRIGGERS.DROPPED_INTO_ZONE) return
    const i = items.findIndex((g) => g.id === id)
    moveItem(id, parentId, orderBetween(items[i - 1]?.task.order, items[i + 1]?.task.order))
  }
</script>

{#snippet line(task: Task, depth: number)}
  <div
    class="group relative flex min-h-[42px] items-center gap-3 rounded-[9px] pr-2 hover:bg-hover"
    style:padding-left="{8 + depth * 24}px"
  >
    <button
      type="button"
      aria-label="Complete {task.title}"
      onclick={() => completeTask(task.id)}
      class="peer grid size-[18px] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
    >
      <Check size={12} strokeWidth={3} />
    </button>
    <button
      type="button"
      onclick={() => (openId = task.id)}
      class="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-0.5 py-2 text-left"
    >
      <span>{task.title}</span>
      {#if task.priority <= 3}
        <Flag size={12} class={flagClass[task.priority]} aria-label="Priority {task.priority}" />
      {/if}
      {#if task.due}
        <span class="text-xs {task.due.date < today() ? 'text-danger' : 'text-muted'}">{dueText(task.due)}</span>
      {/if}
      {#each task.label_ids as id (id)}
        {#if names.has(id)}
          <span class="rounded-full border border-line px-2 py-px text-xs text-muted">{names.get(id)}</span>
        {/if}
      {/each}
    </button>
    <span class="absolute right-2 bottom-0 h-px bg-line group-hover:hidden" style:left="{36 + depth * 24}px"></span>
  </div>
{/snippet}

<div class="mb-2 flex items-center gap-2">
  <h1 class="page-title flex-1">{title}</h1>
  {#if parentId !== inbox_id()}
    <Button variant="ghost" onclick={() => onadd?.(`@${title} `)}>Add task</Button>
  {/if}
</div>
<div class="mb-4">
  <Composer {parentId} autofocus={false} />
</div>

{#if groups.length}
  <ul use:dndzone={{ items, type: 'task', flipDurationMs: 150, delayTouchStart: 200, useCursorForDetection: true }} onconsider={consider} onfinalize={finalize}>
    {#each items as g (g.id)}
      <li>
        {@render line(g.task, 0)}
        {#each g.subs as s (s.task.id)}
          {@render line(s.task, s.depth)}
        {/each}
      </li>
    {/each}
  </ul>
{:else}
  <div class="flex flex-col items-center gap-2 px-3 py-6 text-center text-muted">
    <Inbox size={20} class="text-icon" />
    <p>Nothing here. Type a task above and press Enter.</p>
  </div>
{/if}

<TaskDrawer taskId={openId} onclose={closeDrawer} />
