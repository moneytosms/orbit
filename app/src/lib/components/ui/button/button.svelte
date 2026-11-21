<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants'
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements'

  export const buttonVariants = tv({
    // Orbit buttons (DESIGN.md): surface default, one accent primary per region, ghost, toggled "on".
    base: "inline-flex shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-md border border-transparent text-sm font-medium transition-colors select-none outline-none disabled:pointer-events-none disabled:bg-sunk disabled:text-muted disabled:border-transparent disabled:shadow-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
    variants: {
      variant: {
        default: 'bg-surface border-line text-text shadow-1 hover:border-line-strong',
        primary: 'bg-accent text-accent-ink shadow-1 hover:brightness-106',
        ghost: 'text-muted hover:bg-hover hover:text-text',
        on: 'bg-accent-soft border-accent-line text-accent-text',
        destructive: 'bg-danger-soft text-danger-text hover:border-danger',
      },
      size: {
        default: 'h-10 px-3 desk:h-8',
        sm: 'h-9 px-2.5 desk:h-7',
        icon: 'size-11 text-icon desk:size-8',
        'icon-sm': 'size-[1.625rem] text-icon',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  })

  export type ButtonVariant = VariantProps<typeof buttonVariants>['variant']
  export type ButtonSize = VariantProps<typeof buttonVariants>['size']

  export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
    WithElementRef<HTMLAnchorAttributes> & {
      variant?: ButtonVariant
      size?: ButtonSize
    }
</script>

<script lang="ts">
  let {
    class: className,
    variant = 'default',
    size = 'default',
    ref = $bindable(null),
    href = undefined,
    type = 'button',
    disabled,
    children,
    ...restProps
  }: ButtonProps = $props()
</script>

{#if href}
  <a
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    href={disabled ? undefined : href}
    aria-disabled={disabled}
    role={disabled ? 'link' : undefined}
    tabindex={disabled ? -1 : undefined}
    {...restProps}
  >
    {@render children?.()}
  </a>
{:else}
  <button
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    {type}
    {disabled}
    {...restProps}
  >
    {@render children?.()}
  </button>
{/if}
