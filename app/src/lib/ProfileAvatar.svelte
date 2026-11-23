<script lang="ts">
  import { onDestroy } from 'svelte'
  import { blobUrl } from '$lib/blobs'

  let {
    name = '',
    avatar = '',
    device = '',
    size = 'size-10',
  }: { name?: string; avatar?: string; device?: string; size?: string } = $props()
  let url = $state<string>()
  let previous = ''
  const seed = $derived([...device].reduce((n, c) => (n * 31 + c.charCodeAt(0)) >>> 0, 7))
  const emoji = $derived(['🌙', '🌿', '✨', '☀️', '🌊', '🪐', '🍀', '🔥'][seed % 8])
  const initial = $derived(Array.from(name.trim())[0]?.toLocaleUpperCase() ?? '')
  onDestroy(() => {
    if (previous) URL.revokeObjectURL(previous)
  })

  $effect(() => {
    const hash = avatar
    let active = true
    if (!hash) {
      if (url) URL.revokeObjectURL(url)
      url = undefined
      previous = ''
      return
    }
    void blobUrl(hash).then((next) => {
      if (!active) {
        if (next) URL.revokeObjectURL(next)
        return
      }
      if (previous) URL.revokeObjectURL(previous)
      previous = next ?? ''
      url = next
    })
    return () => (active = false)
  })
</script>

<span
  class="{size} inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-accent-soft font-semibold text-accent-text"
  style:background-color={initial || url ? undefined : `hsl(${seed % 360} 38% 42%)`}
  style:color={initial || url ? undefined : 'white'}
  aria-hidden="true"
>
  {#if url}
    <img src={url} alt="" class="size-full object-cover" />
  {:else}{initial || emoji}{/if}
</span>
