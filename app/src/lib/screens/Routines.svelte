<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Repeat from '@lucide/svelte/icons/repeat'
  import { Button } from '$lib/components/ui/button'
  import RoutineSheet from '$lib/RoutineSheet.svelte'
  import { findRow, foldAway } from '$lib/foldAway'
  import { pushRestore } from '$lib/undo'
  import {
    archiveItem,
    archivedRoutines,
    checkIn,
    clearCheckIn,
    localDay,
    pauseRoutine,
    resumeRoutine,
    routines,
    routineStats,
    routineIcon,
    unarchiveItem,
    type DayState,
    type Routine,
    type RoutineStats,
    type tree,
  } from '$lib/sync'

  let { areas }: { areas: Awaited<ReturnType<typeof tree>> } = $props()

  type Entry = { routine: Routine; stats: RoutineStats; today: DayState | undefined }
  let entries = $state<Entry[]>([])
  let archived = $state<Routine[]>([])
  let today = $state('')
  let sheetOpen = $state(false)
  let editing = $state<Routine | undefined>()

  // Reloaded whenever the shell refreshes (`areas` is a fresh array after every sync).
  $effect(() => {
    void areas
    load()
  })

  async function load() {
    today = await localDay()
    const next: Entry[] = []
    for (const routine of await routines()) {
      const stats = await routineStats(routine.id, today)
      next.push({ routine, stats, today: stats.days.find(([d]) => d === today)?.[1] })
    }
    entries = next
    archived = await archivedRoutines()
  }

  const dayClass: Record<DayState, string> = {
    done: 'bg-accent',
    skipped: 'bg-accent-soft',
    missed: 'bg-danger/40',
    paused: 'bg-sunk',
    off: 'bg-transparent',
    pending: 'border border-line',
  }
  const pct = (r: number | null) => (r == null ? '–' : `${Math.round(r * 100)}%`)

  async function tick(e: Entry, state: 'done' | 'skipped') {
    if (e.today === state) await clearCheckIn(e.routine.id, today)
    else await checkIn(e.routine.id, today, state)
    await load()
  }

  // Tap ticks done; a long press (~500 ms), the context menu or Shift+Enter skips.
  let timer: ReturnType<typeof setTimeout> | undefined
  let longPressed = false
  function down(e: Entry) {
    longPressed = false
    timer = setTimeout(() => {
      longPressed = true
      tick(e, 'skipped')
    }, 500)
  }
  const up = () => clearTimeout(timer)
  function click(e: Entry) {
    if (longPressed) longPressed = false
    else tick(e, 'done')
  }
  function key(ev: KeyboardEvent, e: Entry) {
    if (ev.key !== 'Enter') return
    ev.preventDefault()
    tick(e, ev.shiftKey ? 'skipped' : 'done')
  }

  const canTick = (e: Entry) => e.today !== undefined && e.today !== 'off' && e.today !== 'paused'

  function edit(r?: Routine) {
    editing = r
    sheetOpen = true
  }
  async function archive(r: Routine) {
    await foldAway(findRow(r.id))
    await archiveItem(r.id)
    await load()
    pushRestore({
      text: `"${r.name}" archived`,
      restore: async () => {
        await unarchiveItem(r.id)
        await load()
      },
    })
  }
  async function unarchive(r: Routine) {
    await unarchiveItem(r.id)
    await load()
  }
  async function togglePause(e: Entry) {
    if (e.stats.paused) await resumeRoutine(e.routine.id)
    else await pauseRoutine(e.routine.id)
    await load()
  }
</script>

<div class="mb-4 flex items-center justify-between">
  <h1 class="page-title mb-0">Routines</h1>
  <Button onclick={() => edit()} disabled={!areas.length}>New routine</Button>
</div>

<div class="flex flex-col gap-3">
  {#each entries as e (e.routine.id)}
    <article class="panel" aria-label={e.routine.name} data-row-id={e.routine.id}>
      <div class="flex items-center gap-3">
        <button
          type="button"
          aria-label="Tick {e.routine.name}"
          aria-pressed={e.today === 'done'}
          disabled={!canTick(e)}
          onpointerdown={() => down(e)}
          onpointerup={up}
          onpointerleave={up}
          onpointercancel={up}
          onclick={() => click(e)}
          onkeydown={(ev) => key(ev, e)}
          oncontextmenu={(ev) => {
            ev.preventDefault()
            if (canTick(e)) tick(e, 'skipped')
          }}
          class="grid size-9 shrink-0 place-items-center rounded-full border border-line text-sm aria-pressed:border-accent-line aria-pressed:bg-accent-soft disabled:opacity-40"
        >
          {e.today === 'done' ? '✓' : e.today === 'skipped' ? '↷' : ''}
        </button>
        <div class="min-w-0 flex-1">
          <div class="flex min-w-0 items-center gap-2 font-medium">
            <span>{routineIcon(e.routine)}</span><span class="truncate">{e.routine.name}</span>
          </div>
          <div class="text-xs text-muted">
            {e.stats.paused ? 'Paused · ' : ''}{e.routine.time ?? 'Anytime'}
          </div>
        </div>
        <Button variant="ghost" size="sm" onclick={() => edit(e.routine)}>Edit</Button>
        <Button variant="ghost" size="sm" onclick={() => togglePause(e)}>{e.stats.paused ? 'Resume' : 'Pause'}</Button>
        <Button variant="ghost" size="sm" onclick={() => archive(e.routine)}>Archive</Button>
      </div>
      <div class="mt-3 flex flex-wrap gap-4 text-xs text-muted">
        <span>Streak {e.stats.current_streak}</span>
        <span>Best {e.stats.best_streak}</span>
        <span>Completion {pct(e.stats.completion_rate)}</span>
      </div>
      <div class="mt-2 flex flex-wrap gap-0.5" role="img" aria-label="Last 12 weeks">
        {#each e.stats.days.slice(-84) as [date, state] (date)}
          <span title="{date}: {state}" class="size-3 rounded-sm {dayClass[state]}"></span>
        {/each}
      </div>
    </article>
  {:else}
    <Empty
      icon={Repeat}
      text={areas.length
        ? 'Routines are habits you tick off each day and build streaks on.'
        : 'Create an Area first, then add routines to it.'}
      label={areas.length ? 'New routine' : undefined}
      onclick={() => edit()}
    />
  {/each}

  {#if archived.length}
    <section class="grid gap-2">
      <h2 class="panel-title">Archived</h2>
      {#each archived as r (r.id)}
        <div class="panel flex items-center justify-between">
          <span>{routineIcon(r)} {r.name}</span>
          <Button variant="ghost" size="sm" onclick={() => unarchive(r)}>Unarchive</Button>
        </div>
      {/each}
    </section>
  {/if}
</div>

<RoutineSheet bind:open={sheetOpen} {areas} routine={editing} ondone={load} />
