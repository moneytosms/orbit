<script lang="ts">
  const EMOJI = (
    '📁 📂 🏠 🏡 💼 💻 📚 📖 📝 📌 📎 📅 ⏰ ✅ 🎯 🚀 💡 🔧 🛠️ 🧪 🎨 🎵 🎬 📷 ✈️ 🚗 🚲 🏃 🏋️ 🧘 ' +
    '🍎 🥗 🍳 ☕ 🛒 💰 💳 📈 🧾 🎓 🧠 ❤️ 🌱 🌳 🌊 ⭐ 🔥 🎁 🐶 🐱 👪 👤 🧹 🪴 🛏️ 🔑 📞 ✉️ 🌍 🏆'
  ).split(' ')
  let {
    icon = '',
    onselect,
    triggerClass = '',
  }: { icon?: string; onselect: (icon: string) => void; triggerClass?: string } = $props()
  let open = $state(false)
  let typed = $state('')

  function setTyped(e: SubmitEvent) {
    e.preventDefault()
    const first = [...new Intl.Segmenter().segment(typed.trim())][0]?.segment
    typed = ''
    if (first) {
      onselect(first)
      open = false
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (open = false)} />

<div>
  {#if icon}
    <button
      type="button"
      aria-label="Change icon"
      aria-expanded={open}
      onclick={() => (open = !open)}
      class={triggerClass || 'grid size-11 place-items-center rounded-lg text-3xl leading-none hover:bg-hover'}
      >{icon}</button
    >
  {:else}
    <button
      type="button"
      aria-expanded={open}
      onclick={() => (open = !open)}
      class={triggerClass || 'rounded-md px-2 py-1 text-sm text-muted hover:bg-hover'}>Add icon</button
    >
  {/if}
  {#if open}
    <div role="group" aria-label="Icon picker" class="mt-2 rounded-xl border border-line bg-surface p-3 shadow-1">
      <ul class="grid grid-cols-10 gap-0.5 max-[420px]:grid-cols-6">
        {#each EMOJI as e (e)}
          <li>
            <button
              type="button"
              aria-label="Use {e}"
              onclick={() => {
                onselect(e)
                open = false
              }}
              class="grid size-8 place-items-center rounded-md text-xl hover:bg-hover">{e}</button
            >
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
        <button type="submit" class="rounded-md px-2 text-sm hover:bg-hover">Set</button>
        {#if icon}<button
            type="button"
            class="rounded-md px-2 text-sm hover:bg-hover"
            onclick={() => {
              onselect('')
              open = false
            }}>Remove</button
          >{/if}
      </form>
    </div>
  {/if}
</div>
