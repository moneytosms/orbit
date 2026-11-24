<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import X from '@lucide/svelte/icons/x'
  import { Button } from '$lib/components/ui/button'
  import { ui } from '$lib/actions'
  import { isDesktopTauri } from '$lib/platform'

  let { hasArea, hasTask, ondismiss }: { hasArea: boolean; hasTask: boolean; ondismiss: () => void } = $props()

  // Quick capture has no data to read, so it ticks once the user tries it.
  let tried = $state(false)
  const steps = $derived([
    {
      done: hasArea,
      label: 'Create an Area',
      hint: 'Areas group your projects, like Work or Home. Add one from the sidebar.',
    },
    { done: hasTask, label: 'Add a task', hint: 'Anything you need to do, big or small.' },
    {
      done: tried,
      label: 'Try quick capture',
      hint: isDesktopTauri()
        ? 'Jot a task from anywhere with the capture shortcut, or press Q here.'
        : 'Press Q to jot a task from any screen.',
    },
  ])
</script>

<section class="panel mb-4 grid gap-2" aria-label="Get started">
  <header class="flex items-start gap-2">
    <h2 class="flex-1 text-[0.9062rem] font-semibold">Get started</h2>
    <button
      type="button"
      aria-label="Dismiss Get started"
      class="grid size-7 shrink-0 place-items-center rounded-md text-icon hover:bg-hover hover:text-text focus-visible:outline-2"
      onclick={ondismiss}><X size={15} /></button
    >
  </header>
  <ul class="grid gap-2">
    {#each steps as s (s.label)}
      <li class="flex items-start gap-2">
        <span
          class="mt-0.5 grid size-5 shrink-0 place-items-center rounded-full border {s.done
            ? 'border-accent bg-accent text-white'
            : 'border-line-strong'}"
          aria-hidden="true"
          >{#if s.done}<Check size={12} />{/if}</span
        >
        <div class="min-w-0 flex-1">
          <p class="text-sm {s.done ? 'text-muted line-through' : 'font-medium'}">{s.label}</p>
          {#if !s.done}<p class="text-xs text-muted">{s.hint}</p>{/if}
        </div>
      </li>
    {/each}
  </ul>
  <Button
    variant="default"
    size="sm"
    class="justify-self-start"
    onclick={() => {
      tried = true
      ui.newTask()
    }}>Try quick capture</Button
  >
</section>
