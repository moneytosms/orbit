<script lang="ts">
  // The one dropdown. A few short choices show flat as a segmented control, a longer list opens a popover,
  // and a long one adds a search box. The same props everywhere: `value` (bindable), `options`, `onchange`.
  import { Combobox, Select as S } from 'bits-ui'
  import Check from '@lucide/svelte/icons/check'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import { cn } from '$lib/utils.js'

  type Option = { value: string; label: string; disabled?: boolean }
  let {
    value = $bindable(''),
    options,
    onchange,
    id,
    class: className = '',
    disabled = false,
    placeholder = 'Select',
    segmented = true,
    'aria-label': ariaLabel,
  }: {
    value?: string
    options: Option[]
    onchange?: (value: string) => void
    id?: string
    class?: string
    disabled?: boolean
    placeholder?: string
    /** Allow the flat segmented look when there are few, short choices. */
    segmented?: boolean
    'aria-label'?: string
  } = $props()

  const SEARCH_FROM = 9
  const chars = $derived(options.reduce((n, o) => n + o.label.length, 0))
  const mode = $derived(
    segmented && options.length >= 2 && options.length <= 4 && chars <= 32
      ? 'seg'
      : options.length >= SEARCH_FROM
        ? 'search'
        : 'list',
  )
  const current = $derived(options.find((o) => o.value === value))
  // The primitives treat an empty string as "nothing chosen", so "None" style options travel under a stand-in.
  const NONE = '\u0000none'
  const enc = (v: string) => (v === '' ? NONE : v)
  const dec = (v: string) => (v === NONE ? '' : v)

  function set(next: string) {
    if (next === value) return
    value = next
    onchange?.(next)
  }

  let query = $state('')
  let open = $state(false)
  const shown = $derived(
    query.trim() ? options.filter((o) => o.label.toLowerCase().includes(query.trim().toLowerCase())) : options,
  )

  function arrow(e: KeyboardEvent, i: number) {
    const step =
      e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0
    if (!step) return
    e.preventDefault()
    const next = options[(i + step + options.length) % options.length]
    if (next && !next.disabled) {
      set(next.value)
      ;(e.currentTarget as HTMLElement).parentElement
        ?.querySelectorAll<HTMLElement>('[role="radio"]')
        [options.indexOf(next)]?.focus()
    }
  }

  const triggerClass = $derived(
    cn(
      'inline-flex h-8 min-w-0 items-center justify-between gap-2 rounded-lg border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none transition-colors hover:bg-hover focus-visible:border-accent-line focus-visible:shadow-[0_0_0_3px_var(--accent-soft)] disabled:cursor-not-allowed disabled:opacity-50',
      className,
    ),
  )
  const contentClass =
    'z-[60] max-h-72 min-w-[max(var(--bits-select-anchor-width,0px),var(--bits-combobox-anchor-width,0px),9rem)] overflow-y-auto rounded-xl border border-line-strong bg-surface p-1 text-sm text-text shadow-2 outline-none data-open:animate-in data-open:fade-in-0 data-closed:animate-out data-closed:fade-out-0'
  const itemClass =
    'flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 outline-none data-highlighted:bg-hover data-disabled:cursor-not-allowed data-disabled:opacity-40 data-selected:font-medium'
</script>

{#if mode === 'seg'}
  <div
    role="radiogroup"
    {id}
    aria-label={ariaLabel}
    class={cn('inline-flex max-w-full gap-0.5 rounded-lg border border-line bg-sunk p-0.5', className)}
  >
    {#each options as o, i (o.value)}
      <button
        type="button"
        role="radio"
        aria-checked={o.value === value}
        tabindex={o.value === value || (!current && i === 0) ? 0 : -1}
        disabled={disabled || o.disabled}
        onclick={() => set(o.value)}
        onkeydown={(e) => arrow(e, i)}
        class="min-w-0 truncate rounded-md px-2.5 py-1 text-[0.8125rem] text-muted outline-none transition-colors hover:text-text focus-visible:ring-2 focus-visible:ring-accent-line disabled:opacity-40 aria-checked:bg-surface aria-checked:font-semibold aria-checked:text-text aria-checked:shadow-1"
        >{o.label}</button
      >
    {/each}
  </div>
{:else if mode === 'list'}
  <S.Root
    type="single"
    {disabled}
    value={enc(value)}
    onValueChange={(v) => set(dec(v))}
    items={options.map((o) => ({ ...o, value: enc(o.value) }))}
  >
    <S.Trigger {id} aria-label={ariaLabel} class={triggerClass}>
      <span class={cn('truncate', !current && 'text-muted')}>{current?.label ?? placeholder}</span>
      <ChevronDown size={14} class="shrink-0 text-muted" aria-hidden="true" />
    </S.Trigger>
    <S.Portal>
      <S.Content sideOffset={4} class={contentClass}>
        <S.Viewport>
          {#each options as o (o.value)}
            <S.Item value={enc(o.value)} label={o.label} disabled={o.disabled} class={itemClass}>
              {#snippet children({ selected })}
                <span class="min-w-0 flex-1 truncate">{o.label}</span>
                {#if selected}<Check size={14} class="shrink-0 text-accent" aria-hidden="true" />{/if}
              {/snippet}
            </S.Item>
          {/each}
        </S.Viewport>
      </S.Content>
    </S.Portal>
  </S.Root>
{:else}
  <Combobox.Root
    type="single"
    {disabled}
    bind:open
    value={enc(value)}
    onValueChange={(v) => v && set(dec(v))}
    onOpenChange={(o) => {
      if (o) query = ''
    }}
    items={options.map((o) => ({ ...o, value: enc(o.value) }))}
  >
    <div class="relative">
      <Combobox.Input
        {id}
        aria-label={ariaLabel}
        placeholder={current?.label ?? placeholder}
        oninput={(e) => (query = e.currentTarget.value)}
        onfocus={(e) => e.currentTarget.select()}
        class={cn(triggerClass, 'w-full pr-7 placeholder:text-text')}
      />
      <Combobox.Trigger
        class="absolute inset-y-0 right-0 grid w-7 place-items-center text-muted"
        aria-label="Open list"
      >
        <ChevronDown size={14} aria-hidden="true" />
      </Combobox.Trigger>
    </div>
    <Combobox.Portal>
      <Combobox.Content sideOffset={4} class={contentClass}>
        <Combobox.Viewport>
          {#each shown as o (o.value)}
            <Combobox.Item value={enc(o.value)} label={o.label} disabled={o.disabled} class={itemClass}>
              {#snippet children({ selected })}
                <span class="min-w-0 flex-1 truncate">{o.label}</span>
                {#if selected}<Check size={14} class="shrink-0 text-accent" aria-hidden="true" />{/if}
              {/snippet}
            </Combobox.Item>
          {:else}
            <p class="px-2.5 py-2 text-muted">No match</p>
          {/each}
        </Combobox.Viewport>
      </Combobox.Content>
    </Combobox.Portal>
  </Combobox.Root>
{/if}
