<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet'
  import Backlinks from '$lib/Backlinks.svelte'
  import { Button } from '$lib/components/ui/button'
  import { copyTaskLink, ui } from '$lib/actions'
  import {
    addReminder,
    createLabel,
    deleteItem,
    duplicateTask,
    getTask,
    goals as listGoals,
    inheritedGoal,
    labels as listLabels,
    pageOwner,
    removeReminder,
    updateTask,
    type Goal,
    type Label as TaskLabel,
    type Task,
    type TaskPatch,
  } from '$lib/sync'

  import { POSTPONE, type Postpone } from '$lib/overdue'
  import { postponeTask } from '$lib/postpone'
  import { SNOOZE_LABELS, SNOOZE_PRESETS, snoozeTaskPreset } from '$lib/notify'
  let { taskId, onclose }: { taskId: string | null; onclose: () => void } = $props()

  let open = $state(false)
  let side = $state<'right' | 'bottom'>('bottom')
  let task = $state<Task | undefined>()
  let all = $state<TaskLabel[]>([])
  let goalList = $state<Goal[]>([])
  let inherited = $state<string | null>(null)
  let ownerId = $state<string | null>(null)
  let linkVersion = $state(0)
  let newLabel = $state('')
  let remKind = $state<'due' | 'deadline' | 'at'>('due')
  let remMinutes = $state(30)
  let remAt = $state('')
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
    goalList = await listGoals()
    inherited = await inheritedGoal(id)
    ownerId = await pageOwner(id)
  }

  // Every write goes through the outbox, which predicts the row at once; re-read it to show the result.
  async function save(patch: TaskPatch) {
    if (!taskId) return
    await updateTask(taskId, patch)
    task = await getTask(taskId)
    inherited = await inheritedGoal(taskId)
    linkVersion++
  }

  async function addRem() {
    if (!taskId) return
    if (remKind === 'at') {
      if (!remAt) return
      await addReminder(taskId, { kind: 'at', at: remAt })
    } else {
      await addReminder(taskId, {
        kind: 'before',
        field: remKind,
        minutes: Math.max(0, Math.round(Number(remMinutes)) || 0),
      })
    }
    task = await getTask(taskId)
  }
  async function removeRem(id: string) {
    if (!taskId) return
    await removeReminder(taskId, id)
    task = await getTask(taskId)
  }
  const describe = (r: Task['reminders'][number]) =>
    r.kind === 'at'
      ? `At ${r.at.replace('T', ' ')}${r.spent ? ' (fired)' : ''}`
      : r.minutes === 0
        ? `At ${r.field === 'due' ? 'due time' : 'deadline'}`
        : `${r.minutes} min before ${r.field === 'due' ? 'due' : 'deadline'}`

  function saveTitle(e: FocusEvent) {
    const input = e.currentTarget as HTMLInputElement
    const value = input.value.trim()
    if (!task) return
    if (!value) input.value = task.title
    else if (value !== task.title) save({ title: value })
  }

  function saveDescription(value: string) {
    if (task && value !== task.description) save({ description: value })
  }

  // Quick postpone from the date fields; undo is offered by the toast.
  async function postpone(choice: Postpone) {
    if (!taskId || !task) return
    await postponeTask($state.snapshot(task) as Task, choice)
    task = await getTask(taskId)
    linkVersion++
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
  async function remove() {
    if (!taskId) return
    await deleteItem(taskId)
    open = false
    onclose()
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
            class="h-[2.125rem] w-full min-w-0 rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line focus-visible:shadow-[0_0_0_3px_var(--accent-soft)]"
          />
        </div>

        <div class="grid gap-1.5">
          <span class="text-xs font-medium text-muted">Description</span>
          {#key task.id}
            {#await import('$lib/Editor.svelte')}
              <p class="text-muted">Loading editor...</p>
            {:then { default: Editor }}
              <Editor value={task.description} label="Description" owner={ownerId} onchange={saveDescription} />
            {/await}
          {/key}
          <Backlinks id={task.id} version={linkVersion} />
        </div>

        {#if task.recurrence}
          <div class="flex items-center gap-2 text-sm text-muted">
            <span>
              Repeats every {task.recurrence.rule}{task.recurrence.from_completion ? ' from completion' : ''}
            </span>
            <button type="button" class="text-xs underline hover:text-text" onclick={() => save({ recurrence: null })}>
              Clear repeat
            </button>
          </div>
        {/if}

        <div role="group" aria-label="Postpone" class="flex flex-wrap items-center gap-1.5">
          <span class="text-xs font-medium text-muted">Postpone</span>
          {#each POSTPONE as [choice, text] (choice)}
            <Button size="sm" variant="ghost" onclick={() => postpone(choice)}>{text}</Button>
          {/each}
        </div>

        <div role="group" aria-label="Snooze reminder" class="flex flex-wrap items-center gap-1.5">
          <span class="text-xs font-medium text-muted">Snooze</span>
          {#each SNOOZE_PRESETS as preset (preset)}
            <Button
              size="sm"
              variant="ghost"
              onclick={async () => {
                if (!task) return
                await snoozeTaskPreset(task.id, preset)
                ui.notify(`Reminder snoozed until ${SNOOZE_LABELS[preset]}`)
              }}>{SNOOZE_LABELS[preset]}</Button
            >
          {/each}
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="grid gap-1.5">
            <label for="task-due-date" class="text-xs font-medium text-muted">Due date</label>
            <input
              id="task-due-date"
              type="date"
              value={task.due?.date ?? ''}
              onchange={saveDate}
              class="h-[2.125rem] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
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
              class="h-[2.125rem] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line disabled:bg-sunk disabled:text-muted"
            />
          </div>
          <div class="grid gap-1.5">
            <label for="task-deadline" class="text-xs font-medium text-muted">Deadline</label>
            <input
              id="task-deadline"
              type="date"
              value={task.deadline ?? ''}
              onchange={saveDeadline}
              class="h-[2.125rem] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
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
              class="h-[2.125rem] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
            />
          </div>
        </div>

        <div class="grid gap-1.5">
          <span class="text-xs font-medium text-muted">Reminders</span>
          <ul>
            {#each task.reminders as r (r.id)}
              <li class="flex items-center justify-between gap-2 py-1 text-sm">
                <span>{describe(r)}</span>
                <button
                  type="button"
                  class="text-xs underline hover:text-text"
                  aria-label="Remove reminder"
                  onclick={() => removeRem(r.id)}>Remove</button
                >
              </li>
            {/each}
          </ul>
          <div class="flex flex-wrap items-center gap-2">
            <select
              bind:value={remKind}
              aria-label="Reminder type"
              class="h-[2.125rem] rounded-md border border-line bg-surface px-2 text-sm text-text shadow-1"
            >
              <option value="due">Before due</option>
              <option value="deadline">Before deadline</option>
              <option value="at">At a time</option>
            </select>
            {#if remKind === 'at'}
              <input
                type="datetime-local"
                bind:value={remAt}
                aria-label="Reminder time"
                class="h-[2.125rem] rounded-md border border-line bg-surface px-2 text-sm text-text shadow-1"
              />
            {:else}
              <input
                type="number"
                min="0"
                bind:value={remMinutes}
                aria-label="Minutes before"
                class="h-[2.125rem] w-20 rounded-md border border-line bg-surface px-2 text-sm text-text shadow-1"
              />
              <span class="text-sm text-muted">min</span>
            {/if}
            <Button size="sm" onclick={addRem}>Add reminder</Button>
          </div>
        </div>

        <div class="grid gap-1.5">
          <label for="task-goal" class="text-xs font-medium text-muted">Goal</label>
          <select
            id="task-goal"
            value={task.goal_id ?? ''}
            onchange={(e) => save({ goal_id: e.currentTarget.value || null })}
            class="h-[2.125rem] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
          >
            <option value="">{inherited ? 'From project' : 'None'}</option>
            {#each goalList as g (g.id)}
              <option value={g.id}>{g.name}</option>
            {/each}
          </select>
          {#if !task.goal_id && inherited}
            <p class="text-xs text-muted">
              Counts towards {goalList.find((g) => g.id === inherited)?.name ?? 'a goal'} (from project)
            </p>
          {/if}
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
              placeholder="Add a label"
              aria-label="New label"
              class="h-[2.125rem] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none placeholder:text-muted focus-visible:border-accent-line"
            />
          </form>
        </div>
        <div class="flex flex-wrap gap-2">
          <Button variant="ghost" aria-label="Duplicate task" onclick={() => duplicateTask(task?.id ?? '')}>
            Duplicate task
          </Button>
          <Button variant="ghost" aria-label="Copy task link" onclick={() => copyTaskLink(task?.id ?? '')}>
            Copy link
          </Button>
        </div>
        <Button variant="ghost" aria-label="Delete task" onclick={remove}>Delete task</Button>
      </div>
    {/if}
  </Sheet.Content>
</Sheet.Root>
