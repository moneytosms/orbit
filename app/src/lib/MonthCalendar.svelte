<script lang="ts">
  // A small month picker (Monday first) shared by the Date and Deadline panels of the capture bar.
  // It owns only which month is on show; the caller owns what is picked.
  import ChevronLeft from '@lucide/svelte/icons/chevron-left'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import { monthGrid, shiftMonth } from './captureEdit'

  let {
    selected = '',
    pending = '',
    initial = '',
    onpick,
  }: {
    /** `YYYY-MM-DD` already set on the task; shown filled and `aria-pressed`. */
    selected?: string
    /** A picked day not yet applied; shown filled. */
    pending?: string
    /** `YYYY-MM-DD` whose month opens first; this month when empty. */
    initial?: string
    onpick: (date: string) => void
  } = $props()

  const now = new Date()
  const todayIso = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
  // The panel remounts each time it opens, so the starting month is read once.
  // svelte-ignore state_referenced_locally
  const start = initial.split('-').map(Number)
  let view = $state(
    start.length === 3 && start[0] ? { y: start[0], m: start[1] } : { y: now.getFullYear(), m: now.getMonth() + 1 },
  )
  const grid = $derived(monthGrid(view.y, view.m))
  const monthTitle = $derived(
    new Date(Date.UTC(view.y, view.m - 1, 1)).toLocaleDateString('en-GB', {
      month: 'long',
      year: 'numeric',
      timeZone: 'UTC',
    }),
  )
</script>

<div class="w-fit" role="group" aria-label="Calendar">
  <div class="mb-1 flex items-center justify-between gap-2 text-[0.8125rem] font-medium">
    <button
      type="button"
      class="grid size-7 place-items-center rounded-md hover:bg-hover"
      aria-label="Previous month"
      onclick={() => (view = shiftMonth(view.y, view.m, -1))}><ChevronLeft size={14} /></button
    >
    <span>{monthTitle}</span>
    <button
      type="button"
      class="grid size-7 place-items-center rounded-md hover:bg-hover"
      aria-label="Next month"
      onclick={() => (view = shiftMonth(view.y, view.m, 1))}><ChevronRight size={14} /></button
    >
  </div>
  <div class="grid grid-cols-7 gap-0.5 text-center text-xs">
    {#each ['M', 'T', 'W', 'T', 'F', 'S', 'S'] as d, i (i)}
      <span class="py-0.5 text-muted" aria-hidden="true">{d}</span>
    {/each}
    {#each grid as c (c.date)}
      <button
        type="button"
        class="grid size-7 place-items-center rounded-md hover:bg-hover {c.date === selected || c.date === pending
          ? 'bg-accent text-accent-ink'
          : c.date === todayIso
            ? 'font-semibold text-accent-text'
            : c.inMonth
              ? ''
              : 'text-muted opacity-60'}"
        aria-label={c.date}
        aria-pressed={c.date === selected}
        onclick={() => onpick(c.date)}>{Number(c.date.slice(8))}</button
      >
    {/each}
  </div>
</div>
