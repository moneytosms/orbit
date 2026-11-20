<script lang="ts">
  import ViewList from './ViewList.svelte'
  import { VIEW_UPCOMING } from '$lib/sync'

  let { version = 0 }: { version?: number } = $props()
</script>

<ViewList viewId={VIEW_UPCOMING} {version} limit={7} />
