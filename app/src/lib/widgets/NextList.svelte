<script lang="ts">
  import Empty from '$lib/Empty.svelte'
  import CircleCheck from '@lucide/svelte/icons/circle-check'
  import Check from '@lucide/svelte/icons/check'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import { completeTask, setMilestoneDone, whatsNext, type NextItem } from '$lib/sync'

  let { version = 0, onchange }: { version?: number; onchange?: () => void } = $props()
  let items = $state<NextItem[]>([])
  let loaded = $state(false)
  let openId = $state<string | null>(null)

  async function load() {
    items = (await whatsNext()).slice(0, 5)
    loaded = true
  }
  $effect(() => {
    void version
    load()
  })
  async function done(i: NextItem) {
    if (i.kind === 'task') await completeTask(i.id)
    else await setMilestoneDone(i.id, true)
    await load()
    onchange?.()
  }
  function closeDrawer() {
    openId = null
    load()
    onchange?.()
  }
</script>

{#if loaded && items.length === 0}
  <Empty
    icon={CircleCheck}
    text="Nothing needs you right now. Overdue and soon-due tasks show up here."
    label="Add a task"
    href="#/inbox"
  />
{/if}
<ul class="grid grid-cols-[minmax(0,1fr)] gap-0.5">
  {#each items as i (i.id)}
    <li class="flex min-h-[2.25rem] items-center gap-3 rounded-[9px] px-1 hover:bg-hover">
      <button
        type="button"
        aria-label="Complete {i.title}"
        onclick={() => done(i)}
        class="grid size-[1.125rem] shrink-0 place-items-center rounded-full border-[1.5px] border-line-strong text-transparent hover:border-ok hover:text-ok"
        ><Check size={12} strokeWidth={3} /></button
      >
      <div class="min-w-0 flex-1">
        {#if i.kind === 'task'}
          <button
            type="button"
            class="task-title-1 block max-w-full text-left"
            title={i.title}
            onclick={() => (openId = i.id)}>{i.title}</button
          >
        {:else}
          <span class="task-title-1 block" title={i.title}>{i.title}</span>
        {/if}
      </div>
      <span class="shrink-0 rounded-full bg-accent-soft px-2 py-px text-xs text-accent-text">{i.reason}</span>
    </li>
  {/each}
</ul>

<TaskDrawer taskId={openId} onclose={closeDrawer} />
