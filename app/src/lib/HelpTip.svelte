<script lang="ts">
  let { label, text }: { label: string; text: string } = $props()
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
      class="absolute left-0 top-full z-40 mt-1 w-64 rounded-lg border border-line bg-surface p-2 text-xs font-normal leading-snug text-text shadow-2"
      >{text}</span
    >
  {/if}
</span>
