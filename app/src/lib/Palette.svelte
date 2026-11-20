<script lang="ts">
  import { tick } from 'svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import { actions, filterActions, setFocusedTask, type Action } from '$lib/actions'
  import { groupHits, KIND_LABEL, search, searchReady, type Hit } from '$lib/search'
  import {
    goals as listGoals,
    labels,
    linkTarget,
    pages as listPages,
    parseFilter,
    quickAdd,
    quickTitle,
    runFilter,
    tree,
    type FilterError,
    type Parsed,
  } from '$lib/sync'
  import { parse_quick_add } from '../../wasm/orbit_wasm'

  let { open = $bindable(false), parentId }: { open: boolean; parentId: string } = $props()

  type Item = { key: string; label: string; hint: string; run: () => Promise<void> | void }
  type Section = { title: string; items: Item[] }

  let text = $state('')
  let active = $state(0)
  let error = $state('')
  let sections = $state<Section[]>([])
  let inputEl: HTMLInputElement | undefined = $state()
  let drawerTask = $state<string | null>(null)
  let stamp = 0

  const flat = $derived(sections.flatMap((s) => s.items))
  const mode = $derived(
    text.startsWith('>') ? 'actions' : text.startsWith('+') ? 'add' : text.startsWith('?') ? 'filter' : 'search',
  )
  const hints: Record<string, string> = {
    search: 'Search everything',
    actions: 'Run an action',
    add: 'Quick-add a Task',
    filter: 'Filter Tasks',
  }

  $effect(() => {
    if (!open) return
    text = ''
    active = 0
    tick().then(() => inputEl?.focus())
  })
  $effect(() => {
    if (open) void refresh(text)
  })

  const pad = (n: number) => String(n).padStart(2, '0')
  const localNow = () => {
    const d = new Date()
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
  }

  async function openHit(h: { id: string; kind: string }) {
    search().recordOpen(h.id)
    open = false
    if (h.kind === 'task') {
      drawerTask = h.id
      setFocusedTask(h.id)
      return
    }
    const href =
      h.kind === 'view' ? `#/view/${h.id}` : h.kind === 'routine' ? '#/routines' : (await linkTarget(h.id))?.href
    if (href) location.hash = href
  }
  const hitItem = (h: Hit): Item => ({ key: h.id, label: h.title || 'Untitled', hint: h.kind, run: () => openHit(h) })

  async function refresh(t: string) {
    const mine = ++stamp
    error = ''
    let out: Section[] = []
    if (t.startsWith('>')) {
      const list = filterActions(await actions(), t.slice(1))
      const run = (a: Action) => async () => {
        open = false
        await a.run()
      }
      out = [{ title: 'Actions', items: list.map((a) => ({ key: a.id, label: a.name, hint: a.group, run: run(a) })) }]
    } else if (t.startsWith('+')) {
      const [tr, all, gs, ps] = await Promise.all([tree(), labels(), listGoals(), listPages()])
      const known = {
        areas: tr.flatMap((x) => [x.area.name, ...x.projects.map((p) => p.name)]),
        goals: gs.map((g) => g.name),
        labels: all.map((l) => l.name),
        pages: ps.map((x) => x.title),
      }
      const parsed: Parsed = JSON.parse(parse_quick_add(t.slice(1), localNow(), JSON.stringify(known)))
      const title = quickTitle(parsed).trim()
      if (title) {
        out = [
          {
            title: 'Quick-add',
            items: [
              {
                key: 'add',
                label: `Add “${title}”`,
                hint: parsed.due ? `due ${parsed.due.date}` : 'Inbox',
                run: async () => {
                  open = false
                  await quickAdd(parsed, parentId)
                },
              },
            ],
          },
        ]
      }
    } else if (t.startsWith('?')) {
      const src = t.slice(1).trim()
      if (src) {
        try {
          const rows = await runFilter(await parseFilter(src))
          out = [
            {
              title: `${rows.length} Task${rows.length === 1 ? '' : 's'}`,
              items: rows
                .slice(0, 50)
                .map((r) =>
                  hitItem({
                    id: r.id,
                    kind: r.kind,
                    title: (r as { title?: string; name?: string }).title ?? (r as { name?: string }).name ?? '',
                    score: 0,
                  }),
                ),
            },
          ]
        } catch (e) {
          error = (e as FilterError).message ?? String(e)
        }
      }
    } else {
      await searchReady()
      const hits = search().query(t, 30)
      out = t.trim()
        ? groupHits(hits).map((g) => ({ title: KIND_LABEL[g.kind] ?? g.kind, items: g.hits.map(hitItem) }))
        : hits.length
          ? [{ title: 'Recent', items: hits.map(hitItem) }]
          : []
    }
    if (mine !== stamp) return
    sections = out
    active = 0
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      if (!flat.length) return
      active = (active + (e.key === 'ArrowDown' ? 1 : -1) + flat.length) % flat.length
      document.getElementById(`pal-${active}`)?.scrollIntoView?.({ block: 'nearest' })
    } else if (e.key === 'Enter') {
      e.preventDefault()
      flat[active]?.run()
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content
    showCloseButton={false}
    class="top-[12%] min-w-0 w-[min(640px,calc(100%-2rem))] -translate-y-0 gap-0 p-0"
  >
    <Dialog.Title class="sr-only">Search and commands</Dialog.Title>
    <input
      bind:this={inputEl}
      bind:value={text}
      {onkeydown}
      role="combobox"
      aria-expanded={flat.length > 0}
      aria-controls="pal-list"
      aria-activedescendant={flat.length ? `pal-${active}` : undefined}
      aria-label="Search and commands"
      autocomplete="off"
      placeholder="Search, or type > actions, + add, ? filter"
      class="h-12 w-full border-b border-line bg-transparent px-4 text-[15px] outline-none placeholder:text-muted"
    />
    <div class="max-h-[50vh] overflow-y-auto p-2">
      {#if error}
        <p role="alert" class="px-2.5 py-2 text-[13px] text-danger-text">{error}</p>
      {/if}
      <ul id="pal-list" role="listbox" aria-label={hints[mode]} class="flex flex-col">
        {#each sections as sec (sec.title)}
          <li role="presentation" class="px-2.5 pt-2 pb-1 text-[12.5px] font-semibold text-muted">{sec.title}</li>
          {#each sec.items as it (it.key)}
            {@const i = flat.indexOf(it)}
            <li
              id="pal-{i}"
              role="option"
              aria-selected={i === active}
              onclick={() => it.run()}
              onkeydown={() => {}}
              onmousemove={() => (active = i)}
              class="flex h-9 cursor-pointer items-center gap-2 rounded-[7px] px-2.5 aria-selected:bg-press"
            >
              <span class="min-w-0 flex-1 truncate">{it.label}</span>
              <span class="text-[12.5px] text-muted">{it.hint}</span>
            </li>
          {/each}
        {/each}
      </ul>
      {#if !sections.length && !error}
        <p class="px-2.5 py-3 text-[13px] text-muted">
          {text.trim() || mode !== 'search' ? 'Nothing found.' : 'Nothing opened yet.'}
        </p>
      {/if}
    </div>
  </Dialog.Content>
</Dialog.Root>

<TaskDrawer
  taskId={drawerTask}
  onclose={() => {
    drawerTask = null
    setFocusedTask(null)
  }}
/>
