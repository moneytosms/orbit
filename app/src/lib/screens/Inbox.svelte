<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus'
  import Check from '@lucide/svelte/icons/check'
  import Inbox from '@lucide/svelte/icons/inbox'
  import { dndzone, TRIGGERS, type DndEvent } from 'svelte-dnd-action'
  import { Button } from '$lib/components/ui/button'
  import { completeTask, createTask, moveItem, orderBetween, type Task } from '$lib/sync'

  type Entry = { task: Task; depth: number }
  type Group = { id: string; task: Task; subs: Entry[] }

  let { parentId, title, rows }: { parentId: string; title: string; rows: Entry[] } = $props()
  let draft = $state('')

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

  async function add(e: SubmitEvent) {
    e.preventDefault()
    if (!draft.trim()) return
    await createTask(draft, parentId)
    draft = ''
  }

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
    <span class="flex-1 py-2">{task.title}</span>
    <span class="absolute right-2 bottom-0 h-px bg-line group-hover:hidden" style:left="{36 + depth * 24}px"></span>
  </div>
{/snippet}

<h1 class="page-title">{title}</h1>
<form
  onsubmit={add}
  class="mb-4 flex items-center gap-2 rounded-2xl border border-line bg-surface py-1.5 pr-1.5 pl-3 shadow-1 focus-within:border-accent-line"
>
  <Plus size={16} class="text-accent" />
  <input
    id="composer"
    bind:value={draft}
    placeholder="Add a task"
    aria-label="Task title"
    class="h-8 min-w-0 flex-1 bg-transparent text-[15px] outline-none placeholder:text-muted focus-visible:shadow-none"
  />
  <Button type="submit" variant="primary" disabled={!draft.trim()}>Add</Button>
</form>

{#if groups.length}
  <ul use:dndzone={{ items, flipDurationMs: 150, delayTouchStart: 200 }} onconsider={consider} onfinalize={finalize}>
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
