<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Repeat from '@lucide/svelte/icons/repeat'
  import { underArea } from '$lib/area'
  import {
    checkIn,
    clearCheckIn,
    localDay,
    routines,
    routineIcon,
    routineStats,
    visibleSnapshot,
    type DayState,
    type Routine,
  } from '$lib/sync'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** With `parentId` only the Routines under that Area (or Project) show. */
  let {
    version = 0,
    parentId,
    onmeta,
    onchange,
  }: { version?: number; parentId?: string; onmeta?: (value: string) => void; onchange?: () => void } = $props()
  type Entry = { routine: Routine; today: DayState; current: number }
  let entries = $state<Entry[]>([])
  let today = $state('')
  let loaded = $state(false)
  const done = $derived(entries.filter((entry) => entry.today === 'done').length)
  const streak = $derived(Math.max(0, ...entries.map((entry) => entry.current)))

  $effect(() => {
    void version
    load()
  })
  async function load() {
    today = await localDay()
    const next: Entry[] = []
    const under = parentId ? underArea(await visibleSnapshot(), parentId) : null
    for (const routine of (await routines()).filter((r) => !under || under.has(r.parent_id))) {
      const stats = await routineStats(routine.id, today)
      const day = stats.days.find(([d]) => d === today)?.[1]
      if (day && day !== 'off' && day !== 'paused') next.push({ routine, today: day, current: stats.current_streak })
    }
    entries = next
    loaded = true
    onmeta?.(next.length ? `${done} of ${next.length} · 🔥 ${streak}-day streak` : '0 due today')
  }
  async function tick(e: Entry) {
    if (e.today === 'done') await clearCheckIn(e.routine.id, today)
    else await checkIn(e.routine.id, today, 'done')
    await load()
    onchange?.()
  }
  const late = (time: string | null) => {
    if (!time) return false
    const [hours, minutes] = time.split(':').map(Number)
    const now = new Date()
    return hours * 60 + minutes < now.getHours() * 60 + now.getMinutes()
  }
</script>

{#if loaded && entries.length === 0}
  <Empty icon={Repeat} text="No routines due today. Add one to start a streak." label="Routines" href="#/routines" />
{:else if !loaded}
  <WidgetSkeleton rows={3} />
{:else}
  <ul class="grid gap-1">
    {#each entries as e (e.routine.id)}
      <li
        class="flex min-h-11 items-center gap-2 rounded-md px-1 {e.today === 'pending' && late(e.routine.time)
          ? 'bg-danger/5'
          : ''}"
      >
        <button
          type="button"
          aria-label="Check in {e.routine.name}"
          aria-pressed={e.today === 'done'}
          onclick={() => tick(e)}
          class="tap-44 grid size-10 shrink-0 place-items-center rounded-full border text-sm {e.today === 'done'
            ? 'border-accent bg-accent text-white'
            : e.today === 'skipped'
              ? 'border-line-strong text-muted'
              : 'border-line-strong'}">{e.today === 'done' ? '✓' : e.today === 'skipped' ? '↷' : ''}</button
        >
        <span class="shrink-0 text-lg" aria-hidden="true">{routineIcon(e.routine)}</span>
        <span class="min-w-0 flex-1 truncate" title={e.routine.name}>{e.routine.name}</span>
        {#if e.routine.time}<time class="shrink-0 text-xs text-muted">{e.routine.time}</time>{/if}
        {#if e.today === 'skipped'}<span class="shrink-0 text-xs text-muted">Skipped</span>{/if}
      </li>
    {/each}
  </ul>
{/if}
