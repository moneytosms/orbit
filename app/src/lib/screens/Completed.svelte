<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import Empty from '$lib/Empty.svelte'
  import { Button } from '$lib/components/ui/button'
  import { dayKey, groupCompleted } from '$lib/completed'
  import { completedTasks, reopenTask, type Task } from '$lib/sync'

  // `version` changes on every refresh from another device or a local write; the list reloads with it.
  let { version = 0, onchange }: { version?: number; onchange: () => void } = $props()

  let tasks = $state<Task[]>([])
  let query = $state('')
  const groups = $derived(groupCompleted(tasks, query, dayKey(Date.now())))

  async function load() {
    tasks = await completedTasks()
  }
  $effect(() => {
    void version
    load()
  })

  const doneAt = (at: number | null) =>
    at == null ? '' : new Date(at).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' })

  async function restore(id: string) {
    await reopenTask(id)
    await load()
    onchange()
  }
</script>

<h1 class="page-title">Completed</h1>
<div class="mb-4">
  <input
    type="search"
    bind:value={query}
    placeholder="Search completed tasks"
    aria-label="Search completed tasks"
    class="h-9 w-full rounded-md border border-line bg-surface px-2.5 text-sm outline-none placeholder:text-muted focus-visible:border-accent-line"
  />
</div>
<div class="flex flex-col gap-4">
  {#each groups as group (group.day)}
    <section class="panel" aria-label="Completed {group.label}">
      <h2 class="panel-title">{group.label} <span class="font-normal text-muted">· {group.items.length}</span></h2>
      {#each group.items as task (task.id)}
        <div class="flex items-center gap-3 py-1.5">
          <span class="min-w-0 flex-1 truncate" title={task.title}>{task.title}</span>
          <span class="text-xs text-muted">{doneAt(task.completed_at)}</span>
          <Button variant="default" size="sm" aria-label="Restore {task.title}" onclick={() => restore(task.id)}>
            Restore
          </Button>
        </div>
      {/each}
    </section>
  {:else}
    <Empty
      icon={CircleCheck}
      text={query.trim()
        ? 'No completed tasks match that search.'
        : 'Tasks you finish appear here, grouped by the day you completed them.'}
    />
  {/each}
</div>
