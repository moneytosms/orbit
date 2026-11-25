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
  import CaptureBar from './CaptureBar.svelte'
  import { pasteLines, type PasteLine } from './pasteLines'
  import { DEFAULT_PREFS, knownJson, quickAddPrefs, type QuickAddPrefs } from '$lib/captureKnown'

  let {
    prefill = '',
    parentId,
    forceParent = false,
    autofocus = true,
    onsaved,
  }: { prefill?: string; parentId: string; forceParent?: boolean; autofocus?: boolean; onsaved?: () => void } = $props()

  // Read once: the composer is mounted fresh each time it opens.
  let draft = $state(untrack(() => prefill))
  let caret = $state(untrack(() => prefill.length))
  let focused = $state(false)
  let inputEl: HTMLInputElement | undefined = $state()
  let known = $state({ areas: [] as string[], goals: [] as string[], labels: [] as string[], pages: [] as string[] })
  let prefs = $state<QuickAddPrefs>(DEFAULT_PREFS)

  async function loadKnown() {
    const [t, all, gs, ps] = await Promise.all([tree(), labels(), listGoals(), listPages()])
    known = {
      areas: t.flatMap((x) => [x.area.name, ...x.projects.map((p) => p.name)]),
      goals: gs.map((g) => g.name),
      labels: all.map((l) => l.name),
      pages: ps.map((x) => x.title),
    }
    prefs = await quickAddPrefs()
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
  const parsed: Parsed = $derived(JSON.parse(parse_quick_add(draft, localNow(), knownJson(known, prefs))))

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

  // A pick in the capture bar rewrites the line; put the caret back at its end so typing carries on.
  async function edited() {
    await tick()
    inputEl?.focus()
    caret = draft.length
    inputEl?.setSelectionRange(caret, caret)
  }
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
    if (t.known && t.kind === 'reminder')
      return t.value.includes('T') ? `at ${t.value.replace('T', ' ')}` : `${t.value} min before`
    if (t.known) return t.kind === 'date' ? `→ ${t.value}` : ''
    if (t.kind === 'label') return 'new label'
    if (t.kind === 'place') return 'no such area'
    return 'kept in title'
  }

  // Multi-line paste: offer one task per line instead of one long title.
  let pasted: PasteLine[] | null = $state(null)
  const parseLine = (text: string): Parsed => JSON.parse(parse_quick_add(text, localNow(), knownJson(known, prefs)))
  function onpaste(e: ClipboardEvent) {
    const lines = pasteLines(e.clipboardData?.getData('text') ?? '')
    if (lines.length < 2) return
    e.preventDefault()
    pasted = lines
  }
  async function addPasted(split: boolean) {
    const lines = pasted
    pasted = null
    if (!lines) return
    if (!split) {
      draft = lines.map((l) => l.text).join(' ')
      caret = draft.length
      await edited()
      return
    }
    const chain: string[] = []
    for (const l of lines) {
      const p = parseLine(l.text)
      if (!quickTitle(p).trim()) continue
      const parent = l.depth > 0 ? chain[Math.min(l.depth, chain.length) - 1] : undefined
      const id = await quickAdd(p, parent ?? parentId, forceParent || !!parent)
      chain.length = l.depth
      chain[l.depth] = id
    }
    await loadKnown()
    onsaved?.()
    await tick()
    inputEl?.focus()
  }

  async function save(e: SubmitEvent) {
    e.preventDefault()
    if (!quickTitle(parsed).trim()) return
    await quickAdd(parsed, parentId, forceParent)
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
      {onpaste}
      onkeydown={(e) => {
        if (e.key === 'Escape' && pasted) {
          e.preventDefault()
          e.stopPropagation()
          pasted = null
        }
      }}
      onkeyup={track}
      onclick={track}
      onfocus={() => (focused = true)}
      placeholder="Add a task"
      aria-label="Task title"
      class="h-8 min-w-0 flex-1 bg-transparent text-[0.9375rem] outline-none placeholder:text-muted focus-visible:shadow-none"
    />
    <Button type="submit" variant="primary" disabled={!quickTitle(parsed).trim()}>Add</Button>
  </div>

  {#if pasted}
    <div role="group" aria-label="Add pasted lines" class="flex flex-wrap items-center gap-2 pb-1 text-sm">
      <span>Add {pasted.length} tasks?</span>
      <Button type="button" variant="ghost" onclick={() => (pasted = null)}>Cancel</Button>
      <Button type="button" onclick={() => addPasted(false)}>Add 1 task</Button>
      <Button type="button" variant="primary" onclick={() => addPasted(true)}>Add {pasted.length} tasks</Button>
    </div>
  {/if}

  {#if focused || draft}
    <div class="pb-1">
      <CaptureBar bind:draft {parsed} {known} onedit={edited} />
    </div>
  {/if}

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
            class="rounded-md bg-sunk px-2 py-0.5 text-[0.8125rem] hover:bg-hover"
          >
            {suggestion.sigil}{n}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</form>
