<script lang="ts">
  import type { Component } from 'svelte'
  import { Button } from '$lib/components/ui/button'

  // What this place is for, in one sentence, with at most one primary action.
  let {
    icon: Icon,
    text,
    label,
    href,
    onclick,
  }: {
    icon: Component<{ size?: number; class?: string }>
    text: string
    label?: string
    href?: string
    onclick?: () => void
  } = $props()
</script>

<div class="flex flex-col items-center gap-2 px-3 py-6 text-center text-muted">
  <Icon size={20} class="text-icon" />
  <p>{text}</p>
  {#if label}<Button size="sm" {href} {onclick}>{label}</Button>{/if}
</div>
