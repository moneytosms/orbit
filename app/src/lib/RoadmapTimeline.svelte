<script lang="ts">
  import { layoutTimeline } from '$lib/roadmapTimeline'
  import type { PhaseSummary } from '$lib/sync'

  let { phases, today }: { phases: PhaseSummary[]; today: string } = $props()

  const timeline = $derived(layoutTimeline(phases, today))
  const lane = 22
</script>

<div class="overflow-x-auto pb-2" aria-label="Timeline">
  <div class="relative min-w-[720px]">
    <div class="mb-1 flex justify-between text-[11.5px] text-muted">
      <span>{timeline.first}</span><span>{timeline.last}</span>
    </div>
    <div class="relative flex flex-col gap-2">
      {#if timeline.todayX !== null}
        <div
          class="pointer-events-none absolute inset-y-0 z-10 w-px bg-accent"
          style="left: {timeline.todayX}%"
          role="img"
          aria-label="Today"
        >
          <span class="absolute -top-4 left-1 text-[11px] text-accent-text">Today</span>
        </div>
      {/if}
      {#each timeline.bands as b (b.id)}
        <section aria-label={b.name} class="relative" style="height: {b.lanes * lane + 28}px">
          <div
            class="absolute top-0 bottom-0 rounded-lg border bg-sunk/60 {b.dashed
              ? 'border-dashed border-muted'
              : 'border-line'} {b.current ? 'border-accent' : ''} {b.locked ? 'opacity-70' : ''}"
            style="left: {b.x}%; width: {b.w}%"
          >
            <span class="block truncate px-2 pt-1 text-[12.5px] font-semibold">
              {b.name}{b.completed ? ' ✓' : ''}
            </span>
          </div>
          {#each b.items as i (i.id)}
            <div
              class="absolute flex items-center gap-1"
              style="left: {i.x}%; top: {24 + i.lane * lane}px; {i.shape === 'bar' ? `width: ${i.w}%` : ''}"
            >
              {#if i.shape === 'bar'}
                <span class="block h-3 w-full min-w-1.5 rounded-sm bg-accent/70" title={i.title}></span>
              {:else if i.shape === 'diamond'}
                <span class="block size-3 rotate-45 bg-warn" title={i.title}></span>
              {:else}
                <span class="block h-3 w-0.5 bg-text" title={i.title}></span>
              {/if}
              <span class="whitespace-nowrap text-[11.5px]">{i.title}</span>
              {#if i.outside}
                <span class="rounded-sm bg-sunk px-1 text-[10.5px] text-warn-text">outside</span>
              {/if}
            </div>
          {/each}
        </section>
      {/each}
    </div>
  </div>
</div>
