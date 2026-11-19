<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet'
  import {
    createLabel,
    getTask,
    labels as listLabels,
    updateTask,
    type Label as TaskLabel,
    type Task,
    type TaskPatch,
  } from '$lib/sync'

  let { taskId, onclose }: { taskId: string | null; onclose: () => void } = $props()

  let open = $state(false)
  let side = $state<'right' | 'bottom'>('bottom')
  let task = $state<Task | undefined>()
  let all = $state<TaskLabel[]>([])
  let newLabel = $state('')
  const priorities = [1, 2, 3, 4] as const

  // A side drawer on desktop, a bottom sheet on phones; the viewport is read when a Task opens.
  $effect(() => {
    if (!taskId) {
      open = false
      return
    }
    side = typeof matchMedia === 'function' && matchMedia('(min-width: 768px)').matches ? 'right' : 'bottom'
    open = true
    task = undefined
    load(taskId)
  })

  async function load(id: string) {
    task = await getTask(id)
    all = await listLabels()
  }

  // Every write goes through the outbox, which predicts the row at once; re-read it to show the result.
  async function save(patch: TaskPatch) {
    if (!taskId) return
    await updateTask(taskId, patch)
    task = await getTask(taskId)
  }

  function saveTitle(e: FocusEvent) {
    const input = e.currentTarget as HTMLInputElement
    const value = input.value.trim()
    if (!task) return
    if (!value) input.value = task.title
    else if (value !== task.title) save({ title: value })
  }

  function saveDescription(e: FocusEvent) {
    const value = (e.currentTarget as HTMLTextAreaElement).value
    if (task && value !== task.description) save({ description: value })
  }

  // A time only applies to a date: clearing the date clears the time too.
  function saveDate(e: Event) {
    const date = (e.currentTarget as HTMLInputElement).value
    save({ due: date ? { date, time: task?.due?.time ?? null } : null })
  }

  function saveTime(e: Event) {
    const time = (e.currentTarget as HTMLInputElement).value
    if (task?.due) save({ due: { date: task.due.date, time: time || null } })
  }

  function saveDeadline(e: Event) {
    save({ deadline: (e.currentTarget as HTMLInputElement).value || null })
  }

  function saveDuration(e: Event) {
    const n = (e.currentTarget as HTMLInputElement).valueAsNumber
    save({ duration_min: Number.isFinite(n) && n > 0 ? Math.round(n) : null })
  }

  function toggleLabel(id: string) {
    if (!task) return
    const ids = task.label_ids.includes(id) ? task.label_ids.filter((x) => x !== id) : [...task.label_ids, id]
    save({ label_ids: ids })
  }

  async function addLabel(e: SubmitEvent) {
    e.preventDefault()
    const name = newLabel.trim()
    if (!name || !task) return
    const id = await createLabel(name)
    newLabel = ''
    all = await listLabels()
    await save({ label_ids: [...task.label_ids, id] })
  }
</script>

<Sheet.Root
  bind:open
  onOpenChange={(o) => {
    if (!o) onclose()
  }}
>
  <Sheet.Content
    {side}
    class={side === 'right'
      ? 'data-[side=right]:w-[420px] data-[side=right]:sm:max-w-[420px]'
      : 'max-h-[85vh] overflow-y-auto rounded-t-2xl'}
  >
    <Sheet.Title class="sr-only">Task details</Sheet.Title>
    {#if task}
      <div class="flex flex-col gap-4 pt-1 pr-10">
        <div class="grid gap-1.5">
          <label for="task-title" class="text-xs font-medium text-muted">Title</label>
          <input
            id="task-title"
            value={task.title}
            onblur={saveTitle}
            onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
            class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line focus-visible:shadow-[0_0_0_3px_var(--accent-soft)]"
          />
        </div>

        <div class="grid gap-1.5">
          <label for="task-description" class="text-xs font-medium text-muted">Description</label>
          <textarea
            id="task-description"
            value={task.description}
            onblur={saveDescription}
            rows="4"
            class="w-full rounded-md border border-line bg-surface px-2.5 py-2 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line focus-visible:shadow-[0_0_0_3px_var(--accent-soft)]"
          ></textarea>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="grid gap-1.5">
            <label for="task-due-date" class="text-xs font-medium text-muted">Due date</label>
            <input
              id="task-due-date"
              type="date"
              value={task.due?.date ?? ''}
              onchange={saveDate}
              class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
            />
          </div>
          <div class="grid gap-1.5">
            <label for="task-due-time" class="text-xs font-medium text-muted">Due time</label>
            <input
              id="task-due-time"
              type="time"
              value={task.due?.time ?? ''}
              disabled={!task.due}
              onchange={saveTime}
              class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line disabled:bg-sunk disabled:text-muted"
            />
          </div>
          <div class="grid gap-1.5">
            <label for="task-deadline" class="text-xs font-medium text-muted">Deadline</label>
            <input
              id="task-deadline"
              type="date"
              value={task.deadline ?? ''}
              onchange={saveDeadline}
              class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
            />
          </div>
          <div class="grid gap-1.5">
            <label for="task-duration" class="text-xs font-medium text-muted">Duration (minutes)</label>
            <input
              id="task-duration"
              type="number"
              min="1"
              value={task.duration_min ?? ''}
              onchange={saveDuration}
              class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
            />
          </div>
        </div>

        <div class="grid gap-1.5">
          <span class="text-xs font-medium text-muted">Priority</span>
          <div class="flex gap-1" role="group" aria-label="Priority">
            {#each priorities as p (p)}
              <button
                type="button"
                aria-pressed={task.priority === p}
                onclick={() => save({ priority: p })}
                class="h-8 flex-1 rounded-md border border-line text-sm aria-pressed:border-accent-line aria-pressed:bg-accent-soft aria-pressed:font-medium"
              >
                P{p}
              </button>
            {/each}
          </div>
        </div>

        <div class="grid gap-1.5">
          <span class="text-xs font-medium text-muted">Labels</span>
          <div class="flex flex-wrap gap-1.5" role="group" aria-label="Labels">
            {#each all as l (l.id)}
              <button
                type="button"
                aria-pressed={task.label_ids.includes(l.id)}
                onclick={() => toggleLabel(l.id)}
                class="rounded-full border border-line px-2.5 py-1 text-xs aria-pressed:border-accent-line aria-pressed:bg-accent-soft"
              >
                {l.name}
              </button>
            {:else}
              <p class="text-sm text-muted">No labels yet.</p>
            {/each}
          </div>
          <form onsubmit={addLabel}>
            <input
              id="task-new-label"
              bind:value={newLabel}
              placeholder="New label, then Enter"
              aria-label="New label"
              class="h-[34px] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none placeholder:text-muted focus-visible:border-accent-line"
            />
          </form>
        </div>
      </div>
    {/if}
  </Sheet.Content>
</Sheet.Root>
