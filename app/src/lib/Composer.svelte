<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import Plus from '@lucide/svelte/icons/plus'
  import { Button } from '$lib/components/ui/button'
  import {
    createArea,
    goals as listGoals,
    labels,
    pages as listPages,
    quickAdd,
    quickTitle,
    tree,
    type Parsed,
  } from '$lib/sync'
  import { parse_quick_add } from '../../wasm/orbit_wasm'

  let {
    prefill = '',
    parentId,
    autofocus = true,
    onsaved,
  }: { prefill?: string; parentId: string; autofocus?: boolean; onsaved?: () => void } = $props()

  // Read once: the composer is mounted fresh each time it opens.
  let draft = $state(untrack(() => prefill))
  let caret = $state(untrack(() => prefill.length))
  let inputEl: HTMLInputElement | undefined = $state()
  let known = $state({ areas: [] as string[], goals: [] as string[], labels: [] as string[], pages: [] as string[] })

  async function loadKnown() {
    const [t, all, gs, ps] = await Promise.all([tree(), labels(), listGoals(), listPages()])
    known = {
      areas: t.flatMap((x) => [x.area.name, ...x.projects.map((p) => p.name)]),
      goals: gs.map((g) => g.name),
      labels: all.map((l) => l.name),
      pages: ps.map((x) => x.title),
    }
  }
  onMount(() => {
    loadKnown()
    if (autofocus) inputEl?.focus()
  })

  const pad = (n: number) => String(n).padStart(2, '0')
  const localNow = () => {
    const d = new Date()
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
  }
  // Page titles are known from the local Pages, so `%Title` resolves.
  const parsed: Parsed = $derived(
    JSON.parse(
      parse_quick_add(
        draft,
        localNow(),
        JSON.stringify({ areas: known.areas, goals: known.goals, pages: known.pages, labels: known.labels }),
      ),
    ),
  )

  // The sigil word under the caret, with the names it could complete to.
  const suggestion = $derived.by(() => {
    const m = /(^|\s)([@$%#])(\S*)$/.exec(draft.slice(0, caret))
    if (!m) return null
    const sigil = m[2]
    const partial = m[3].toLowerCase()
    const pool = sigil === '@' ? known.areas : sigil === '$' ? known.goals : sigil === '#' ? known.labels : []
    const options = pool.filter((n) => n.toLowerCase().includes(partial) && n.toLowerCase() !== partial).slice(0, 6)
    return { sigil, start: m.index + m[1].length, options }
  })

  function track(e: Event) {
    caret = (e.currentTarget as HTMLInputElement).selectionStart ?? 0
  }
  async function pick(name: string) {
    const s = suggestion
    if (!s) return
    const insert = `${s.sigil}${name} `
    draft = draft.slice(0, s.start) + insert + draft.slice(caret)
    caret = s.start + insert.length
    await tick()
    inputEl?.setSelectionRange(caret, caret)
  }
  async function createPlace(name: string) {
    await createArea(name)
    await loadKnown()
  }
  function note(t: Parsed['tokens'][number]) {
    if (t.known) return t.kind === 'date' ? `→ ${t.value}` : ''
    if (t.kind === 'label') return 'new label'
    if (t.kind === 'place') return 'no such area'
    return 'kept in title'
  }

  async function save(e: SubmitEvent) {
    e.preventDefault()
    if (!quickTitle(parsed).trim()) return
    await quickAdd(parsed, parentId)
    draft = ''
    caret = 0
    await loadKnown()
    onsaved?.()
  }
</script>

<form
  onsubmit={save}
  class="flex flex-col gap-2 rounded-2xl border border-line bg-surface py-1.5 pr-1.5 pl-3 shadow-1 focus-within:border-accent-line"
>
  <div class="flex items-center gap-2">
    <Plus size={16} class="text-accent" />
    <input
      id="composer"
      bind:this={inputEl}
      bind:value={draft}
      oninput={track}
      onkeyup={track}
      onclick={track}
      placeholder="Add a task"
      aria-label="Task title"
      class="h-8 min-w-0 flex-1 bg-transparent text-[15px] outline-none placeholder:text-muted focus-visible:shadow-none"
    />
    <Button type="submit" variant="primary" disabled={!quickTitle(parsed).trim()}>Add</Button>
  </div>

  {#if parsed.tokens.length}
    <ul class="flex flex-wrap gap-1.5 pb-1" aria-label="Parsed tokens">
      {#each parsed.tokens as t (t.start)}
        <li
          class="flex items-center gap-1.5 rounded-full border border-line px-2 py-px text-xs {t.known
            ? 'text-muted'
            : 'text-warn'}"
        >
          <span>{t.text}</span>
          {#if note(t)}<span class="opacity-80">{note(t)}</span>{/if}
          {#if !t.known && t.kind === 'place'}
            <button type="button" class="font-medium text-accent-text underline" onclick={() => createPlace(t.value)}>
              Create area
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if suggestion && suggestion.options.length}
    <ul class="flex flex-wrap gap-1 pb-1" aria-label="Suggestions">
      {#each suggestion.options as n (n)}
        <li>
          <button
            type="button"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => pick(n)}
            class="rounded-md bg-sunk px-2 py-0.5 text-[13px] hover:bg-hover"
          >
            {suggestion.sigil}{n}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</form>
