<script lang="ts">
  import { doneShare, glance, type Glance } from '$lib/dashboard'
  import { badgePreference, dueBadgeCount, showBadge } from '$lib/appBadge'
  import { goals, localDay, routines, routineStats, stardust, visibleSnapshot } from '$lib/sync'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  let { version = 0 }: { version?: number } = $props()
  let g = $state<Glance & { done: number; routinesDue: number }>()

  $effect(() => {
    void version
    load()
  })
  async function load() {
    const today = await localDay()
    const base = glance(await visibleSnapshot(), today)
    if (badgePreference()) void showBadge(dueBadgeCount(base))
    let routinesDue = 0
    for (const r of await routines()) {
      const day = (await routineStats(r.id, today)).days.find(([d]) => d === today)?.[1]
      if (day && day !== 'off' && day !== 'paused') routinesDue++
    }
    const activeGoals = (await goals()).length
    g = { ...base, activeGoals, done: (await stardust()).targets.today_done, routinesDue }
  }

  const R = 20
  const C = 2 * Math.PI * R
  const tile = 'flex min-w-0 flex-col justify-between gap-2 rounded-xl border p-3 transition-colors'
</script>

{#if g}
  {@const share = doneShare(g.done, g.dueToday)}
  <div class="grid h-full grid-cols-2 gap-2 desk:grid-cols-4">
    <a href="#/upcoming" class="{tile} border-line bg-sunk hover:border-line-strong">
      <div class="flex items-center gap-3">
        <svg viewBox="0 0 48 48" class="size-12 shrink-0 -rotate-90" aria-hidden="true">
          <circle cx="24" cy="24" r={R} fill="none" stroke-width="5" class="stroke-line-strong" />
          <circle
            cx="24"
            cy="24"
            r={R}
            fill="none"
            stroke-width="5"
            stroke-linecap="round"
            class="stroke-accent transition-[stroke-dashoffset]"
            stroke-dasharray={C}
            stroke-dashoffset={C * (1 - share)}
          />
        </svg>
        <div class="min-w-0">
          <div class="text-[1.375rem] leading-none font-bold tracking-[-0.01em] tabular-nums">{g.dueToday}</div>
          <div class="mt-1 text-xs text-muted">due today</div>
        </div>
      </div>
      <div class="text-xs text-accent-text">{g.done} done today</div>
    </a>
    <a
      href="#/upcoming"
      class="{tile} {g.overdue
        ? 'border-transparent bg-danger-soft text-danger-text hover:border-danger'
        : 'border-line bg-sunk hover:border-line-strong'}"
    >
      <div class="text-[1.375rem] leading-none font-bold tracking-[-0.01em] tabular-nums">{g.overdue}</div>
      <div class="text-xs {g.overdue ? '' : 'text-muted'}">{g.overdue ? 'overdue, catch up' : 'nothing overdue'}</div>
    </a>
    <a href="#/routines" class="{tile} border-line bg-sunk hover:border-line-strong">
      <div class="text-[1.375rem] leading-none font-bold tracking-[-0.01em] tabular-nums">{g.routinesDue}</div>
      <div class="text-xs text-muted">routines today</div>
    </a>
    <a href="#/goals" class="{tile} border-line bg-sunk hover:border-line-strong">
      <div class="text-[1.375rem] leading-none font-bold tracking-[-0.01em] tabular-nums">{g.activeGoals}</div>
      <div class="text-xs text-muted">active goals</div>
    </a>
  </div>
{:else}
  <WidgetSkeleton rows={1} />
{/if}
