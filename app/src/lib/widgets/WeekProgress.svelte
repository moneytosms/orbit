<script lang="ts">
  import { localDay, visibleSnapshot } from '$lib/sync'
  import { share, weekProgress, type Progress } from '$lib/widgetData'
  import WidgetSkeleton from './WidgetSkeleton.svelte'

  /** Tasks finished in the last 7 days against those plus what is due by today, as a ring. */
  let {
    parentId,
    version = 0,
    onmeta,
  }: { parentId?: string; version?: number; onmeta?: (value: string) => void } = $props()
  let p = $state<Progress>({ done: 0, total: 0 })
  let loaded = $state(false)
  $effect(() => {
    void version
    const id = parentId
    Promise.all([visibleSnapshot(), localDay()]).then(([rows, today]) => {
      if (id !== parentId) return
      p = weekProgress(rows, today, Date.now(), id)
      loaded = true
      onmeta?.(p.total === 0 ? 'Nothing planned yet' : `${Math.round(share(p) * 100)}% of ${p.total} tasks`)
    })
  })
  const R = 42
  const C = 2 * Math.PI * R
</script>

{#if !loaded}
  <WidgetSkeleton rows={1} />
{:else}
  <div class="flex h-full items-center gap-4">
    <svg viewBox="0 0 100 100" class="size-24 shrink-0 -rotate-90" role="img" aria-label="{p.done} of {p.total} done">
      <circle cx="50" cy="50" r={R} fill="none" stroke-width="9" class="stroke-line" />
      <circle
        cx="50"
        cy="50"
        r={R}
        fill="none"
        stroke-width="9"
        stroke-linecap="round"
        class="stroke-accent"
        stroke-dasharray={C}
        stroke-dashoffset={C * (1 - share(p))}
      />
    </svg>
    <div class="min-w-0">
      <div class="text-xl font-semibold">{p.done} <span class="text-muted">/ {p.total}</span></div>
      <div class="text-xs text-muted">done in the last 7 days</div>
    </div>
  </div>
{/if}
