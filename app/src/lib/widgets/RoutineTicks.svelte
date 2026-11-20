<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import Repeat from '@lucide/svelte/icons/repeat'
  import { checkIn, clearCheckIn, localDay, routines, routineStats, type DayState, type Routine } from '$lib/sync'

  let { version = 0 }: { version?: number } = $props()
  type Entry = { routine: Routine; today: DayState }
  let entries = $state<Entry[]>([])
  let today = $state('')
  let loaded = $state(false)

  $effect(() => {
    void version
    load()
  })
  async function load() {
    today = await localDay()
    const next: Entry[] = []
    for (const routine of await routines()) {
      const day = (await routineStats(routine.id, today)).days.find(([d]) => d === today)?.[1]
      if (day && day !== 'off' && day !== 'paused') next.push({ routine, today: day })
    }
    entries = next
    loaded = true
  }
  async function tick(e: Entry) {
    if (e.today === 'done') await clearCheckIn(e.routine.id, today)
    else await checkIn(e.routine.id, today, 'done')
    await load()
  }
</script>

{#if loaded && entries.length === 0}
  <Empty
    icon={Repeat}
    text="No routines due today. Routines you add show up here to tick off."
    label="Routines"
    href="#/routines"
  />
{/if}
<ul class="grid gap-0.5">
  {#each entries as e (e.routine.id)}
    <li class="flex min-h-[36px] items-center gap-3 px-1">
      <button
        type="button"
        aria-label="Check in {e.routine.name}"
        aria-pressed={e.today === 'done'}
        onclick={() => tick(e)}
        class="grid size-[18px] shrink-0 place-items-center rounded-md border-[1.5px] text-xs {e.today === 'done'
          ? 'border-accent bg-accent text-white'
          : 'border-line-strong'}">{e.today === 'done' ? '✓' : e.today === 'skipped' ? '↷' : ''}</button
      >
      <span class="min-w-0 flex-1 truncate">{e.routine.name}</span>
    </li>
  {/each}
</ul>
