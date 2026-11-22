<script lang="ts">
  import ViewList from './ViewList.svelte'
  import { VIEW_UPCOMING } from '$lib/sync'

  let { version = 0, onmeta }: { version?: number; onmeta?: (value: string) => void } = $props()
</script>

<ViewList viewId={VIEW_UPCOMING} {version} limit={7} {onmeta} futureOnly />
