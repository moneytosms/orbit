<script lang="ts">
  import Hammer from '@lucide/svelte/icons/hammer'
  let { title }: { title: string } = $props()
</script>

<h1 class="page-title">{title}</h1>
<div class="flex flex-col items-center gap-2 px-3 py-6 text-center text-muted">
  <Hammer size={20} class="text-icon" />
  <p>{title} isn't built yet. Capture tasks in the Inbox for now.</p>
</div>
