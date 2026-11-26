<script lang="ts">
  // The spotlight tour. It runs once on first launch, and again whenever Settings, About sends `orbit-tour`.
  // Each step dims the app and outlines one thing; Back, Next and Skip all work, and so do the arrow keys and Escape.
  import { onMount, tick } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { cardPosition, STEPS, TOUR_KEY, tourOpen, visibleBox, type Box, type Step } from './tour'

  let open = $state(false)
  let index = $state(0)
  let box = $state<Box | null>(null)
  let pos = $state({ left: 0, top: 0 })
  let card: HTMLElement | undefined = $state()
  let steps = $state<Step[]>(STEPS)

  const step = $derived(steps[index])
  const last = $derived(index === steps.length - 1)

  function finish() {
    open = false
    tourOpen.set(false)
    try {
      localStorage.setItem(TOUR_KEY, '1')
    } catch {
      // The tour simply shows again next launch.
    }
  }

  async function place() {
    await tick()
    box = step?.target ? visibleBox(step.target) : null
    const w = card?.offsetWidth ?? 320
    const h = card?.offsetHeight ?? 170
    pos = cardPosition(box, { width: w, height: h }, { width: innerWidth, height: innerHeight })
  }

  async function go(next: number) {
    // Steps whose target is not on screen (the page menu on a screen without one) are skipped.
    const dir = next >= index ? 1 : -1
    let i = next
    const skipped: Step[] = []
    while (steps[i]?.optional && steps[i].target && !visibleBox(steps[i].target!)) {
      skipped.push(steps[i])
      i += dir
    }
    if (i < 0) i = 0
    if (i >= steps.length) return finish()
    // Dropping the skipped steps keeps the "n of N" counter honest.
    const target = steps[i]
    if (skipped.length) steps = steps.filter((s) => !skipped.includes(s))
    index = steps.indexOf(target)
    await place()
  }

  function start() {
    steps = STEPS
    index = 0
    open = true
    tourOpen.set(true)
    void go(0)
  }

  function onkey(e: KeyboardEvent) {
    if (!open) return
    if (e.key === 'Escape') finish()
    else if (e.key === 'ArrowRight') void go(index + 1)
    else if (e.key === 'ArrowLeft') void go(index - 1)
  }

  onMount(() => {
    const replay = () => start()
    window.addEventListener('orbit-tour', replay)
    let seen = false
    try {
      seen = localStorage.getItem(TOUR_KEY) === '1'
    } catch {
      seen = true
    }
    const timer = setTimeout(() => {
      if (!seen && !import.meta.env.VITE_DEMO) start()
    }, 1500)
    return () => {
      clearTimeout(timer)
      window.removeEventListener('orbit-tour', replay)
      tourOpen.set(false)
    }
  })
</script>

<svelte:window onkeydown={onkey} onresize={() => open && place()} />

{#if open && step}
  <div class="fixed inset-0 z-[70]" role="presentation">
    {#if box}
      <div
        class="pointer-events-none fixed rounded-xl border-2 border-accent transition-all duration-200 ease-out"
        style="left:{box.left - 6}px;top:{box.top - 6}px;width:{box.width + 12}px;height:{box.height +
          12}px;box-shadow:0 0 0 9999px rgb(0 0 0 / 0.5)"
      ></div>
    {:else}
      <div class="pointer-events-none fixed inset-0 bg-black/50"></div>
    {/if}
    <div
      bind:this={card}
      role="dialog"
      aria-modal="true"
      aria-label="Orbit tour"
      class="fixed w-[min(20rem,calc(100vw-1.5rem))] rounded-xl border border-line-strong bg-surface p-4 text-sm shadow-2 transition-[left,top] duration-200 ease-out"
      style="left:{pos.left}px;top:{pos.top}px"
    >
      <p class="text-[0.9375rem] font-semibold">{step.title}</p>
      <p class="mt-1 text-muted">{step.body}</p>
      <div class="mt-3 flex items-center justify-between gap-2">
        <span class="text-xs text-muted">{index + 1} of {steps.length}</span>
        <span class="flex items-center gap-1">
          <Button variant="ghost" size="sm" onclick={finish}>Skip</Button>
          {#if index > 0}<Button variant="default" size="sm" onclick={() => go(index - 1)}>Back</Button>{/if}
          <Button size="sm" onclick={() => (last ? finish() : go(index + 1))}>{last ? 'Done' : 'Next'}</Button>
        </span>
      </div>
    </div>
  </div>
{/if}
