<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { addFile, blobUrl } from '$lib/blobs'

  type Patch = { icon?: string; cover?: string }
  // Rows cached before icons existed have neither field.
  let {
    icon = '',
    cover = '',
    onsave,
  }: { icon?: string; cover?: string; onsave: (patch: Patch) => void | Promise<void> } = $props()

  const EMOJI = (
    '📁 📂 🏠 🏡 💼 💻 📚 📖 📝 📌 📎 📅 ⏰ ✅ 🎯 🚀 💡 🔧 🛠️ 🧪 🎨 🎵 🎬 📷 ✈️ 🚗 🚲 🏃 🏋️ 🧘 ' +
    '🍎 🥗 🍳 ☕ 🛒 💰 💳 📈 🧾 🎓 🧠 ❤️ 🌱 🌳 🌊 ⭐ 🔥 🎁 🐶 🐱 👪 👤 🧹 🪴 🛏️ 🔑 📞 ✉️ 🌍 🏆'
  ).split(' ')
  const COLORS = ['#c2552f', '#d9822b', '#e0b04a', '#7a9a5b', '#4f8a7b', '#5b7fa6', '#8a6bb0', '#b5566f']
  const MAX_WIDTH = 1600

  let panel = $state<'icon' | 'cover' | null>(null)
  let typed = $state('')
  let error = $state('')
  let busy = $state(false)
  let imageUrl = $state('')

  $effect(() => {
    imageUrl = ''
    if (!cover.startsWith('blob:')) return
    let url = ''
    let stale = false
    blobUrl(cover.slice(5)).then((u) => {
      if (u && stale) URL.revokeObjectURL(u)
      else if (u) imageUrl = url = u
    })
    return () => {
      stale = true
      if (url) URL.revokeObjectURL(url)
    }
  })

  async function save(patch: Patch) {
    error = ''
    panel = null
    await onsave(patch)
  }

  function setTyped(e: SubmitEvent) {
    e.preventDefault()
    const first = [...new Intl.Segmenter().segment(typed.trim())][0]?.segment
    typed = ''
    if (first) save({ icon: first })
  }

  /** Scales an image wider than MAX_WIDTH down to a JPEG; smaller ones go up untouched. */
  async function shrink(file: File): Promise<File> {
    const bmp = await createImageBitmap(file)
    if (bmp.width <= MAX_WIDTH) return file
    const canvas = document.createElement('canvas')
    canvas.width = MAX_WIDTH
    canvas.height = Math.round((bmp.height * MAX_WIDTH) / bmp.width)
    canvas.getContext('2d')!.drawImage(bmp, 0, 0, canvas.width, canvas.height)
    const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/jpeg', 0.85))
    if (!blob) throw new Error('Could not process that image.')
    return new File([blob], file.name.replace(/\.\w+$/, '') + '.jpg', { type: 'image/jpeg' })
  }

  async function upload(e: Event) {
    const input = e.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    busy = true
    error = ''
    try {
      if (!file.type.startsWith('image/')) throw new Error('Pick an image file.')
      const { hash } = await addFile(await shrink(file))
      await save({ cover: `blob:${hash}` })
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not use that image.'
    } finally {
      busy = false
    }
  }

  const bg = $derived(
    cover.startsWith('#') ? `background:${cover}` : imageUrl ? `background-image:url(${imageUrl})` : '',
  )
  const chip =
    'rounded-md px-2 py-1 text-[0.8125rem] text-muted opacity-70 hover:bg-hover hover:opacity-100 focus-visible:opacity-100'
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (panel = null)} />

<div class="mb-2">
  {#if cover}
    <div
      role="img"
      aria-label="Cover"
      class="h-24 rounded-xl bg-sunk bg-cover bg-center desk:h-[7.5rem]"
      style={bg}
    ></div>
  {/if}
  <div class="flex flex-wrap items-end gap-1 px-1 {cover ? '-mt-7' : 'pt-1'}">
    {#if icon}
      <button
        type="button"
        aria-label="Change icon"
        aria-expanded={panel === 'icon'}
        onclick={() => (panel = panel === 'icon' ? null : 'icon')}
        class="grid size-14 place-items-center rounded-xl text-4xl leading-none hover:bg-hover {cover
          ? 'border border-line bg-surface shadow-1'
          : ''}"
      >
        {icon}
      </button>
    {:else}
      <button
        type="button"
        class={chip}
        aria-expanded={panel === 'icon'}
        onclick={() => (panel = panel === 'icon' ? null : 'icon')}
      >
        Add icon
      </button>
    {/if}
    <button
      type="button"
      class={chip}
      aria-expanded={panel === 'cover'}
      onclick={() => (panel = panel === 'cover' ? null : 'cover')}
    >
      {cover ? 'Change cover' : 'Add cover'}
    </button>
  </div>

  {#if panel === 'icon'}
    <div role="group" aria-label="Icon picker" class="mt-2 rounded-xl border border-line bg-surface p-3 shadow-1">
      <ul class="grid grid-cols-10 gap-0.5 max-[420px]:grid-cols-6">
        {#each EMOJI as e (e)}
          <li>
            <button
              type="button"
              aria-label="Use {e}"
              onclick={() => save({ icon: e })}
              class="grid size-8 place-items-center rounded-md text-xl hover:bg-hover"
            >
              {e}
            </button>
          </li>
        {/each}
      </ul>
      <form class="mt-2 flex gap-2" onsubmit={setTyped}>
        <input
          bind:value={typed}
          aria-label="Any emoji"
          placeholder="Type or paste an emoji"
          class="h-9 min-w-0 flex-1 rounded-md border border-line bg-bg px-2 text-sm"
        />
        <Button type="submit" size="sm" variant="ghost">Set</Button>
        {#if icon}<Button type="button" size="sm" variant="ghost" onclick={() => save({ icon: '' })}>Remove</Button
          >{/if}
      </form>
    </div>
  {:else if panel === 'cover'}
    <div role="group" aria-label="Cover picker" class="mt-2 rounded-xl border border-line bg-surface p-3 shadow-1">
      <ul class="flex flex-wrap gap-2">
        {#each COLORS as c (c)}
          <li>
            <button
              type="button"
              aria-label="Cover colour {c}"
              onclick={() => save({ cover: c })}
              class="size-8 rounded-full border border-line"
              style="background:{c}"
            ></button>
          </li>
        {/each}
      </ul>
      <div class="mt-3 flex flex-wrap items-center gap-2">
        <label
          class="inline-flex h-8 cursor-pointer items-center rounded-md px-3 text-sm hover:bg-hover focus-within:ring-2"
        >
          {busy ? 'Uploading...' : 'Upload image'}
          <input
            type="file"
            accept="image/*"
            aria-label="Upload cover image"
            class="sr-only"
            disabled={busy}
            onchange={upload}
          />
        </label>
        {#if cover}<Button type="button" size="sm" variant="ghost" onclick={() => save({ cover: '' })}>Remove</Button
          >{/if}
      </div>
      {#if error}<p role="alert" class="mt-2 text-sm text-danger">{error}</p>{/if}
    </div>
  {/if}
</div>
