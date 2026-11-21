<script lang="ts">
  import { Checkbox as CheckboxPrimitive } from 'bits-ui'
  import CheckIcon from '@lucide/svelte/icons/check'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    checked = $bindable(false),
    indeterminate = $bindable(false),
    class: className,
    ...restProps
  }: WithoutChildrenOrChild<CheckboxPrimitive.RootProps> = $props()
</script>

<CheckboxPrimitive.Root
  bind:ref
  data-slot="checkbox"
  class={cn(
    "peer relative flex size-4 shrink-0 after:absolute after:-inset-3 after:content-[''] desk:after:-inset-1 items-center justify-center rounded-xs border border-line-strong bg-surface outline-none transition-colors data-checked:border-accent data-checked:bg-accent data-checked:text-accent-ink disabled:cursor-not-allowed disabled:opacity-50",
    className,
  )}
  bind:checked
  bind:indeterminate
  {...restProps}
>
  {#snippet children({ checked, indeterminate })}
    <div data-slot="checkbox-indicator" class="[&>svg]:size-3.5 grid place-content-center text-current transition-none">
      {#if checked}
        <CheckIcon />
      {:else if indeterminate}
        <MinusIcon />
      {/if}
    </div>
  {/snippet}
</CheckboxPrimitive.Root>
