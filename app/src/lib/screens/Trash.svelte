<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { archived, restoreItem, trashed, unarchiveItem, type Trashed } from '$lib/sync'

  let { onchange }: { onchange: () => void } = $props()

  const KEEP_DAYS = 30
  const DAY = 86_400_000
  let items = $state<Trashed[]>([])
  let shelved = $state<Awaited<ReturnType<typeof archived>>>([])

  async function load() {
    items = await trashed()
    shelved = await archived()
  }
  $effect(() => {
    load()
  })

  const label = (r: Trashed) => ('title' in r ? r.title : 'name' in r ? r.name : '')
  const dateText = (at: number) =>
    new Date(at).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })
  const daysLeft = (at: number) => Math.max(0, KEEP_DAYS - Math.floor((Date.now() - at) / DAY))

  async function restore(id: string) {
    await restoreItem(id)
    await load()
    onchange()
  }
  async function unarchive(id: string) {
    await unarchiveItem(id)
    await load()
    onchange()
  }
</script>

<h1 class="page-title">Trash</h1>
<div class="flex flex-col gap-4">
  <section class="panel" aria-label="Trash">
    <h2 class="panel-title">Trash</h2>
    {#each items as r (r.id)}
      <div class="flex items-center gap-3 py-1.5">
        <span class="flex-1">{label(r)}</span>
        <span class="text-xs text-muted">{r.kind}</span>
        <span class="text-xs text-muted">Deleted {dateText(r.trashed_at ?? 0)}</span>
        <span class="text-xs text-muted">{daysLeft(r.trashed_at ?? 0)} days left</span>
        <Button variant="default" size="sm" aria-label="Restore {label(r)}" onclick={() => restore(r.id)}
          >Restore</Button
        >
      </div>
    {:else}
      <p class="text-sm text-muted">Nothing in the Trash.</p>
    {/each}
  </section>
  <section class="panel" aria-label="Archived">
    <h2 class="panel-title">Archived</h2>
    {#each shelved as r (r.id)}
      <div class="flex items-center gap-3 py-1.5">
        <span class="flex-1">{r.name}</span>
        <span class="text-xs text-muted">{r.kind}</span>
        <Button variant="default" size="sm" aria-label="Unarchive {r.name}" onclick={() => unarchive(r.id)}>
          Unarchive
        </Button>
      </div>
    {:else}
      <p class="text-sm text-muted">Nothing archived.</p>
    {/each}
  </section>
</div>
