<script lang="ts">
  import { Switch as SwitchPrimitive } from 'bits-ui'
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    checked = $bindable(false),
    class: className,
    ...restProps
  }: WithoutChildrenOrChild<SwitchPrimitive.RootProps> = $props()
</script>

<!-- The one on/off control: a track and a thumb, tinted with the accent when on. -->
<SwitchPrimitive.Root
  bind:ref
  bind:checked
  data-slot="switch"
  class={cn(
    "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full bg-line-strong outline-none transition-colors after:absolute after:-inset-2 after:content-[''] focus-visible:ring-2 focus-visible:ring-accent-line focus-visible:ring-offset-2 focus-visible:ring-offset-bg data-checked:bg-accent disabled:cursor-not-allowed disabled:opacity-50",
    className,
  )}
  {...restProps}
>
  <SwitchPrimitive.Thumb
    class="pointer-events-none block size-4 translate-x-0.5 rounded-full bg-white shadow-sm transition-transform data-checked:translate-x-[1.125rem]"
  />
</SwitchPrimitive.Root>
