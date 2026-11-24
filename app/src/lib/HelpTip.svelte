<script lang="ts">
  let {
    label,
    text,
    side = 'down',
    align = 'start',
  }: { label: string; text: string; side?: 'down' | 'up'; align?: 'start' | 'end' } = $props()
  const uid = $props.id()
  let pinned = $state(false)
  let hover = $state(false)
  let focus = $state(false)
  const open = $derived(pinned || hover || focus)
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="relative inline-flex" data-ui onpointerenter={() => (hover = true)} onpointerleave={() => (hover = false)}>
  <button
    type="button"
    aria-label="Help: {label}"
    aria-describedby={open ? uid : undefined}
    aria-expanded={open}
    class="grid size-5 place-items-center rounded-full border border-line text-[0.6875rem] font-semibold text-muted hover:bg-hover hover:text-text"
    onclick={() => (pinned = !pinned)}
    onfocus={() => (focus = true)}
    onblur={() => {
      focus = false
      pinned = false
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') pinned = hover = false
    }}>?</button
  >
  {#if open}
    <span
      id={uid}
      role="tooltip"
      class="absolute z-40 w-72 max-w-[calc(100vw-2rem)] whitespace-pre-line rounded-lg border border-line bg-surface p-3 text-xs font-normal leading-relaxed text-text shadow-2 {side ===
      'up'
        ? 'bottom-full mb-1'
        : 'top-full mt-1'} {align === 'end' ? 'right-0' : 'left-0'}">{text}</span
    >
  {/if}
</span>
