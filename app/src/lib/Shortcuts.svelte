<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog'
  import { groupedKeys, keyLabel } from '$lib/keys'

  let { open = $bindable(false) }: { open: boolean } = $props()
  const groups = groupedKeys()
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-h-[85vh] overflow-y-auto">
    <Dialog.Title>Keyboard shortcuts</Dialog.Title>
    <div class="grid gap-4 sm:grid-cols-2">
      {#each groups as g (g.group)}
        <section aria-label={g.group}>
          <h2 class="panel-title mb-1">{g.group}</h2>
          <dl class="grid gap-1">
            {#each g.keys as k (k.keys)}
              <div class="flex items-center justify-between gap-3">
                <dt class="text-muted">{k.label}</dt>
                <dd><kbd class="rounded border border-line px-1.5 py-px font-mono text-xs">{keyLabel(k.keys)}</kbd></dd>
              </div>
            {/each}
          </dl>
        </section>
      {/each}
    </div>
  </Dialog.Content>
</Dialog.Root>
