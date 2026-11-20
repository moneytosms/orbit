<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import EyeOff from '@lucide/svelte/icons/eye-off'
  import Pin from '@lucide/svelte/icons/pin'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import { completeTask, notToday, pinItem, setMilestoneDone, whatsNext, type NextItem } from '$lib/sync'

  let { onchange }: { onchange: () => void } = $props()

  let items = $state<NextItem[]>([])
  let loaded = $state(false)
  let openId = $state<string | null>(null)

  async function load() {
    items = await whatsNext()
    loaded = true
  }
  $effect(() => {
    load()
  })

  async function done(i: NextItem) {
    if (i.kind === 'task') await completeTask(i.id)
    else await setMilestoneDone(i.id, true)
    await load()
    onchange()
  }
  async function pin(i: NextItem) {
    await pinItem(i.id, true)
    await load()
    onchange()
  }
  async function skip(i: NextItem) {
    await notToday(i.id, true)
    await load()
    onchange()
  }
  function closeDrawer() {
    openId = null
    load()
    onchange()
  }
  const action =
    'grid size-7 shrink-0 place-items-center rounded-md text-icon hover:bg-hover hover:text-text focus-visible:outline-2'
</script>

<h1 class="page-title">What's next</h1>
{#if loaded && items.length === 0}
  <p class="text-muted">Nothing needs you right now.</p>
{/if}
<ul class="grid gap-0.5">
  {#each items as i (i.id)}
    <li
      data-task-row={i.kind === 'task' ? i.id : undefined}
      class="flex min-h-[42px] items-center gap-3 rounded-[9px] px-2 hover:bg-hover data-[cursor=true]:bg-hover data-[cursor=true]:ring-2 data-[cursor=true]:ring-accent"
    >
      <button
        type="button"
        aria-label="Complete {i.title}"
        onclick={() => done(i)}
        class="grid size-[18px] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
        ><Check size={12} strokeWidth={3} /></button
      >
      <div class="min-w-0 flex-1">
        {#if i.kind === 'task'}
          <button type="button" data-open class="block max-w-full truncate text-left" onclick={() => (openId = i.id)}
            >{i.title}</button
          >
        {:else}
          <span class="block truncate">{i.title}</span>
        {/if}
        {#if i.breadcrumb.length}
          <div class="truncate text-xs text-muted">{i.breadcrumb.join(' / ')}</div>
        {/if}
      </div>
      <span class="shrink-0 rounded-full bg-accent-soft px-2 py-px text-xs text-accent-text">{i.reason}</span>
      <button type="button" aria-label="Pin {i.title}" class={action} onclick={() => pin(i)}><Pin size={15} /></button>
      <button type="button" aria-label="Not today {i.title}" class={action} onclick={() => skip(i)}
        ><EyeOff size={15} /></button
      >
    </li>
  {/each}
</ul>

<TaskDrawer taskId={openId} onclose={closeDrawer} />
