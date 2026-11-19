<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus'
  import Check from '@lucide/svelte/icons/check'
  import Inbox from '@lucide/svelte/icons/inbox'
  import { Button } from '$lib/components/ui/button'
  import { completeTask, createTask, type Task } from '$lib/sync'

  let { tasks }: { tasks: Task[] } = $props()
  let title = $state('')

  async function add(e: SubmitEvent) {
    e.preventDefault()
    if (!title.trim()) return
    await createTask(title)
    title = ''
  }
</script>

<h1 class="page-title">Inbox</h1>
<form
  onsubmit={add}
  class="mb-4 flex items-center gap-2 rounded-2xl border border-line bg-surface py-1.5 pr-1.5 pl-3 shadow-1 focus-within:border-accent-line"
>
  <Plus size={16} class="text-accent" />
  <input
    id="composer"
    bind:value={title}
    placeholder="Add a task"
    aria-label="Task title"
    class="h-8 min-w-0 flex-1 bg-transparent text-[15px] outline-none placeholder:text-muted focus-visible:shadow-none"
  />
  <Button type="submit" variant="primary" disabled={!title.trim()}>Add</Button>
</form>

{#if tasks.length}
  <ul>
    {#each tasks as task (task.id)}
      <li class="group relative flex min-h-[42px] items-center gap-3 rounded-[9px] px-2 hover:bg-hover">
        <button
          type="button"
          aria-label="Complete {task.title}"
          onclick={() => completeTask(task.id)}
          class="peer grid size-[18px] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
        >
          <Check size={12} strokeWidth={3} />
        </button>
        <span class="flex-1 py-2">{task.title}</span>
        <span class="absolute right-2 bottom-0 left-9 h-px bg-line group-hover:hidden"></span>
      </li>
    {/each}
  </ul>
{:else}
  <div class="flex flex-col items-center gap-2 px-3 py-6 text-center text-muted">
    <Inbox size={20} class="text-icon" />
    <p>Nothing here. Type a task above and press Enter.</p>
  </div>
{/if}
