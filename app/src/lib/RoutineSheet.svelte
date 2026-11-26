<script lang="ts">
  import { Select } from '$lib/components/ui/select'
  import * as Sheet from '$lib/components/ui/sheet'
  import { Button } from '$lib/components/ui/button'
  import IconPicker from '$lib/IconPicker.svelte'
  import {
    createRoutine,
    goals as listGoals,
    updateRoutine,
    type Goal,
    type Routine,
    type RoutineInput,
    type Schedule,
    type tree,
  } from '$lib/sync'

  // One sheet sets up a Routine: schedule, length and time of day. `routine` set means editing.
  let {
    open = $bindable(false),
    areas,
    routine,
    ondone,
  }: {
    open?: boolean
    areas: Awaited<ReturnType<typeof tree>>
    routine?: Routine
    ondone: () => void
  } = $props()

  const inputClass =
    'h-[2.125rem] w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line'

  let name = $state('')
  let icon = $state('')
  let kind = $state<Schedule['kind']>('daily')
  let n = $state(2)
  let ongoing = $state(true)
  let days = $state(30)
  let time = $state('')
  let reminder = $state('')
  let goalId = $state('')
  let parentId = $state('')
  let targets = $state<Goal[]>([])

  $effect(() => {
    if (!open) return
    name = routine?.name ?? ''
    icon = routine?.icon ?? ''
    const s = routine?.schedule
    kind = s?.kind ?? 'daily'
    n = s && 'n' in s ? s.n : 2
    ongoing = routine ? routine.length_days == null : true
    days = routine?.length_days ?? 30
    time = routine?.time ?? ''
    reminder = routine?.reminder ?? ''
    goalId = routine?.goal_id ?? ''
    parentId = routine?.parent_id ?? areas[0]?.area.id ?? ''
    listGoals().then((g) => (targets = g.filter((x) => x.progress.kind === 'target')))
  })

  function schedule(): Schedule {
    const k = Math.max(1, Math.round(Number(n)) || 1)
    if (kind === 'every') return { kind, n: k }
    if (kind === 'per_week') return { kind, n: Math.min(7, k) }
    return { kind }
  }

  async function save(e: SubmitEvent) {
    e.preventDefault()
    const title = name.trim()
    if (!title || !parentId) return
    const input: RoutineInput = {
      name: title,
      icon,
      schedule: schedule(),
      length_days: ongoing ? null : Math.max(1, Math.round(Number(days)) || 1),
      time: time || null,
      goal_id: goalId || null,
      reminder: reminder || null,
    }
    if (routine) await updateRoutine(routine.id, input)
    else await createRoutine(parentId, input)
    open = false
    ondone()
  }
</script>

<Sheet.Root bind:open>
  <Sheet.Content side="right" class="overflow-y-auto sm:max-w-[420px]">
    <Sheet.Title class="sr-only">{routine ? 'Edit Routine' : 'New Routine'}</Sheet.Title>
    <form class="flex flex-col gap-4 pt-1 pr-10" onsubmit={save}>
      <div class="grid gap-1.5">
        <p class="text-xs font-medium text-muted">Icon</p>
        <IconPicker {icon} onselect={(value) => (icon = value)} />
      </div>
      <div class="grid gap-1.5">
        <label for="routine-name" class="text-xs font-medium text-muted">Name</label>
        <input id="routine-name" bind:value={name} class={inputClass} />
      </div>
      {#if !routine}
        <div class="grid gap-1.5">
          <label for="routine-area" class="text-xs font-medium text-muted">Area</label>
          <Select
            id="routine-area"
            bind:value={parentId}
            segmented={false}
            class="w-full"
            options={areas.map((a) => ({ value: a.area.id, label: a.area.name }))}
          />
        </div>
      {/if}
      <div class="grid gap-1.5">
        <label for="routine-schedule" class="text-xs font-medium text-muted">Schedule</label>
        <Select
          id="routine-schedule"
          bind:value={kind}
          segmented={false}
          class="w-full"
          options={[
            { value: 'daily', label: 'Daily' },
            { value: 'weekdays', label: 'Weekdays' },
            { value: 'every', label: 'Every N days' },
            { value: 'per_week', label: 'N per week' },
          ]}
        />
        {#if kind === 'every' || kind === 'per_week'}
          <input
            type="number"
            min="1"
            max={kind === 'per_week' ? 7 : undefined}
            bind:value={n}
            aria-label={kind === 'every' ? 'Days between' : 'Times per week'}
            class={inputClass}
          />
        {/if}
      </div>
      <div class="grid gap-1.5">
        <label for="routine-length" class="text-xs font-medium text-muted">Length</label>
        <Select
          id="routine-length"
          value={ongoing ? 'ongoing' : 'days'}
          onchange={(v) => (ongoing = v === 'ongoing')}
          segmented={false}
          class="w-full"
          options={[
            { value: 'ongoing', label: 'Ongoing' },
            { value: 'days', label: 'For N days, then archive' },
          ]}
        />
        {#if !ongoing}
          <input type="number" min="1" bind:value={days} aria-label="Length in days" class={inputClass} />
        {/if}
      </div>
      <div class="grid gap-1.5">
        <label for="routine-time" class="text-xs font-medium text-muted">Time of day</label>
        <input id="routine-time" type="time" bind:value={time} class={inputClass} />
        <p class="text-xs text-muted">Leave empty for anytime.</p>
      </div>
      <div class="grid gap-1.5">
        <label for="routine-reminder" class="text-xs font-medium text-muted">Daily reminder</label>
        <input id="routine-reminder" type="time" bind:value={reminder} class={inputClass} />
        <p class="text-xs text-muted">Leave empty for none. Skipped once you've checked in that day.</p>
      </div>
      <div class="grid gap-1.5">
        <label for="routine-goal" class="text-xs font-medium text-muted">Target goal</label>
        <Select
          id="routine-goal"
          bind:value={goalId}
          segmented={false}
          class="w-full"
          options={[{ value: '', label: 'None' }, ...targets.map((g) => ({ value: g.id, label: g.name }))]}
        />
        <p class="text-xs text-muted">Each done check-in adds 1.</p>
      </div>
      <Button type="submit" disabled={!name.trim() || !parentId}>{routine ? 'Save' : 'Add routine'}</Button>
    </form>
  </Sheet.Content>
</Sheet.Root>
