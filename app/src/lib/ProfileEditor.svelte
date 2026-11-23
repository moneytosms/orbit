<script lang="ts">
  import { addFile } from '$lib/blobs'
  import ProfileAvatar from '$lib/ProfileAvatar.svelte'
  import { deviceId } from '$lib/auth'
  import { getProfile, setProfile } from '$lib/sync'

  let { version = 0 }: { version?: number } = $props()
  let profile = $state({ display_name: '', avatar: '' })
  let device = $state('')
  let busy = $state(false)
  let message = $state('')

  $effect(() => {
    void version
    void getProfile().then((next) => (profile = next))
    void deviceId().then((id) => (device = id ?? ''))
  })

  async function saveName(event: FocusEvent) {
    const name = (event.currentTarget as HTMLInputElement).value.trim()
    if (name === profile.display_name) return
    message = ''
    try {
      await setProfile({ display_name: name })
      profile = { ...profile, display_name: name }
    } catch {
      message = 'Could not save the display name.'
    }
  }

  function toBlob(canvas: HTMLCanvasElement, type: string, quality?: number): Promise<Blob | null> {
    return new Promise((resolve) => canvas.toBlob(resolve, type, quality))
  }

  async function resized(file: File): Promise<File> {
    if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type))
      throw new Error('Choose a PNG, JPEG or WebP image.')
    const bitmap = await createImageBitmap(file)
    try {
      const canvas = document.createElement('canvas')
      canvas.width = canvas.height = 512
      const ctx = canvas.getContext('2d')
      if (!ctx) throw new Error('Could not process that image.')
      const side = Math.min(bitmap.width, bitmap.height)
      ctx.drawImage(bitmap, (bitmap.width - side) / 2, (bitmap.height - side) / 2, side, side, 0, 0, 512, 512)
      const qualities = file.type === 'image/png' ? [undefined] : [0.9, 0.8, 0.7, 0.6]
      for (const type of [file.type, ...(file.type === 'image/png' ? ['image/webp'] : [])]) {
        for (const quality of type === 'image/png' ? qualities : [0.9, 0.8, 0.7, 0.6]) {
          const blob = await toBlob(canvas, type, quality)
          if (blob && blob.size <= 2 * 1024 * 1024)
            return new File([blob], 'avatar.' + (blob.type.split('/')[1] || 'png'), { type: blob.type })
        }
      }
      throw new Error('That image is still over 2 MB after resizing.')
    } finally {
      bitmap.close()
    }
  }

  async function upload(event: Event) {
    const input = event.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    busy = true
    message = ''
    try {
      const { hash } = await addFile(await resized(file))
      await setProfile({ avatar: hash })
      profile = { ...profile, avatar: hash }
    } catch (error) {
      message = error instanceof Error ? error.message : 'Could not save that picture.'
    } finally {
      busy = false
    }
  }

  async function remove() {
    busy = true
    message = ''
    try {
      await setProfile({ avatar: '' })
      profile = { ...profile, avatar: '' }
    } catch {
      message = 'Could not remove that picture.'
    } finally {
      busy = false
    }
  }
</script>

<div class="flex flex-wrap items-center gap-4">
  <ProfileAvatar name={profile.display_name} avatar={profile.avatar} {device} size="size-16 text-xl" />
  <div class="grid gap-2">
    <label class="grid gap-1 text-sm">
      <span class="font-medium">Display name</span>
      <input
        value={profile.display_name}
        onblur={saveName}
        aria-label="Display name"
        autocomplete="name"
        class="h-[2.125rem] w-64 max-w-full rounded-md border border-line bg-surface px-2.5 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line"
      />
    </label>
    <div class="flex flex-wrap items-center gap-2">
      <label class="cursor-pointer rounded-md border border-line px-3 py-1.5 text-sm hover:bg-hover">
        {busy ? 'Saving…' : profile.avatar ? 'Change picture' : 'Upload picture'}
        <input type="file" accept="image/png,image/jpeg,image/webp" class="sr-only" onchange={upload} disabled={busy} />
      </label>
      {#if profile.avatar}
        <button type="button" class="text-sm text-muted hover:text-text" onclick={remove} disabled={busy}>Remove</button
        >
      {/if}
      <span class="text-xs text-muted">PNG, JPEG or WebP · square crop · up to 2 MB</span>
    </div>
  </div>
</div>
{#if message}<p class="mt-2 text-sm text-danger-text" role="alert">{message}</p>{/if}
