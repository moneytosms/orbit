<script lang="ts">
  import { backlinksOf } from '$lib/sync'

  let { id, version = 0 }: { id: string; version?: number } = $props()

  let groups = $state<{ kind: string; items: { id: string; title: string; href: string | null }[] }[]>([])

  $effect(() => {
    void version
    backlinksOf(id).then((rows) => {
      const by = new Map<string, { id: string; title: string; href: string | null }[]>()
      for (const r of rows) by.set(r.kind, [...(by.get(r.kind) ?? []), r])
      groups = [...by].map(([kind, items]) => ({ kind, items }))
    })
  })
</script>

<section aria-label="Linked from" class="mt-4">
  <h2 class="text-xs font-medium text-muted">Linked from</h2>
  {#each groups as g (g.kind)}
    <h3 class="mt-2 text-xs capitalize text-muted">{g.kind}s</h3>
    <ul class="text-sm">
      {#each g.items as item (item.id)}
        <li>
          {#if item.href}<a href={item.href}>{item.title || 'Untitled'}</a>{:else}{item.title || 'Untitled'}{/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="text-sm text-muted">Nothing links here yet.</p>
  {/each}
</section>
