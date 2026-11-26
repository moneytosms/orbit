<script lang="ts">
  import { onMount } from 'svelte'
  import { DEFAULT_PREFS, knownJson, quickAddPrefs } from '$lib/captureKnown'
  import { goals, labels, pages, quickAdd, quickTitle, tree, type Parsed } from '$lib/sync'
  import { inbox_id, parse_quick_add } from '../../../wasm/orbit_wasm'

  /** One-line capture with the Composer's parser; on a page the Task goes under `parentId`, else the Inbox. */
  let { parentId, onchange, onmeta }: { parentId?: string; onchange?: () => void; onmeta?: (value: string) => void } =
    $props()
  let draft = $state('')
  let added = $state('')
  let timer: ReturnType<typeof setTimeout> | undefined
  onMount(() => {
    onmeta?.(parentId ? 'Add to this page' : 'Add to your Inbox')
    return () => clearTimeout(timer)
  })

  const pad = (n: number) => String(n).padStart(2, '0')
  const localNow = () => {
    const d = new Date()
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
  }
  async function parseLine(text: string): Promise<Parsed> {
    const [t, ls, gs, ps, prefs] = await Promise.all([tree(), labels(), goals(), pages(), quickAddPrefs()])
    const known = {
      areas: t.flatMap((x) => [x.area.name, ...x.projects.map((p) => p.name)]),
      goals: gs.map((g) => g.name),
      labels: ls.map((l) => l.name),
      pages: ps.map((x) => x.title),
    }
    return JSON.parse(parse_quick_add(text, localNow(), knownJson(known, prefs ?? DEFAULT_PREFS)))
  }
  async function save(e: SubmitEvent) {
    e.preventDefault()
    const text = draft.trim()
    if (!text) return
    const p = await parseLine(text)
    const title = quickTitle(p).trim()
    if (!title) return
    // A page's own Tasks stay there (forceParent); on Home an `@Area` in the text still places it.
    await quickAdd(p, parentId ?? inbox_id(), parentId !== undefined)
    draft = ''
    added = `Added "${title}"`
    clearTimeout(timer)
    timer = setTimeout(() => (added = ''), 2500)
    onchange?.()
  }
</script>

<form onsubmit={save} class="flex h-full flex-col justify-center gap-2">
  <input
    bind:value={draft}
    aria-label="Quick add a task"
    placeholder="Add a task, e.g. Call Sam tomorrow p1"
    class="h-10 w-full rounded-[9px] border border-line bg-surface px-3 text-sm placeholder:text-muted focus:border-accent focus:outline-none"
  />
  <p class="min-h-4 text-xs text-muted" role="status">{added || 'Press Enter to add'}</p>
</form>
