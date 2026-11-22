<script lang="ts">
  let { rows = 2 }: { rows?: number } = $props()
</script>

<div role="status" aria-label="Loading widget" class="grid gap-2 py-1">
  {#each Array.from({ length: rows }, (_, i) => i) as row (row)}
    <div class="flex items-center gap-2">
      <span class="size-4 animate-pulse rounded-full bg-sunk"></span>
      <span class="h-3 flex-1 animate-pulse rounded bg-sunk" style="max-width: {row % 2 ? '65%' : '88%'}"></span>
    </div>
  {/each}
</div>
