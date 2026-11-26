<script lang="ts">
  import { tick } from 'svelte'
  import * as Dialog from '$lib/components/ui/dialog'
  import TaskDrawer from '$lib/TaskDrawer.svelte'
  import { actions, filterActions, setFocusedTask, ui, type Action } from '$lib/actions'
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
  import { knownJson, quickAddPrefs } from '$lib/captureKnown'
  import { deadlineLabel, dueLabel, durationLabel } from '$lib/captureEdit'
  import { commandsFirst, matchCommands } from '$lib/palette'
  import SearchIcon from '@lucide/svelte/icons/search'
  import SquareCheck from '@lucide/svelte/icons/square-check'
  import FolderIcon from '@lucide/svelte/icons/folder'
  import TargetIcon from '@lucide/svelte/icons/target'
  import FileText from '@lucide/svelte/icons/file-text'
  import ZapIcon from '@lucide/svelte/icons/zap'
  import PlusIcon from '@lucide/svelte/icons/plus'

  let { open = $bindable(false), parentId }: { open: boolean; parentId: string } = $props()

  type IconC = typeof SearchIcon
  type Item = { key: string; label: string; hint: string; icon?: IconC; run: () => Promise<void> | void }
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
  const modeBadge: Record<string, string> = {
    search: 'Search',
    actions: 'Actions',
    add: 'Quick add',
    filter: 'Filter',
  }
  const KIND_ICON: Record<string, IconC> = {
    task: SquareCheck,
    project: FolderIcon,
    area: FolderIcon,
    goal: TargetIcon,
    page: FileText,
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
  // Palette-only command: the tour's replay is a window event (see AboutSection), not an action in the registry.
  const tourCommand: Action = {
    id: 'tour.replay',
    name: 'Replay tour',
    group: 'App',
    run: () => {
      window.dispatchEvent(new CustomEvent('orbit-tour'))
    },
  }
  const runCommand = (a: Action) => async () => {
    open = false
    await a.run()
  }
  /**
   * Commands an unprefixed query finds too, and whether they go above the task results (exact or prefix title).
   * A failed registry read leaves only the palette's own commands.
   */
  async function commandSection(t: string): Promise<{ section: Section; lead: boolean } | null> {
    const registry = await actions().catch(() => [] as Action[])
    const matched = matchCommands([...registry, tourCommand], t)
    if (!matched.length) return null
    return {
      section: {
        title: 'Commands',
        items: matched.map(({ item: a }) => ({
          key: a.id,
          label: a.name,
          hint: 'Command',
          icon: ZapIcon,
          run: runCommand(a),
        })),
      },
      lead: commandsFirst(matched),
    }
  }
  const hitItem = (h: Hit): Item => ({
    key: h.id,
    label: h.title || 'Untitled',
    hint: h.kind,
    icon: KIND_ICON[h.kind],
    run: () => openHit(h),
  })

  async function refresh(t: string) {
    const mine = ++stamp
    error = ''
    let out: Section[] = []
    if (t.startsWith('>')) {
      const list = filterActions(await actions(), t.slice(1))
      out = [
        {
          title: 'Actions',
          items: list.map((a) => ({ key: a.id, label: a.name, hint: a.group, icon: ZapIcon, run: runCommand(a) })),
        },
      ]
    } else if (t.startsWith('+')) {
      const [tr, all, gs, ps] = await Promise.all([tree(), labels(), listGoals(), listPages()])
      const known = {
        areas: tr.flatMap((x) => [x.area.name, ...x.projects.map((p) => p.name)]),
        goals: gs.map((g) => g.name),
        labels: all.map((l) => l.name),
        pages: ps.map((x) => x.title),
      }
      const parsed: Parsed = JSON.parse(
        parse_quick_add(t.slice(1), localNow(), knownJson(known, await quickAddPrefs())),
      )
      const extra = parsed as Parsed & { deadline?: string | null; duration_min?: number | null }
      const title = quickTitle(parsed).trim()
      if (title) {
        out = [
          {
            title: 'Quick-add',
            items: [
              {
                key: 'add',
                label: `Add “${title}”`,
                hint: [
                  parsed.place ?? 'Inbox',
                  parsed.due ? dueLabel(parsed.due) : '',
                  extra.deadline ? `by ${deadlineLabel(extra.deadline)}` : '',
                  extra.duration_min ? durationLabel(extra.duration_min) : '',
                  parsed.priority ? `P${parsed.priority}` : '',
                ]
                  .filter(Boolean)
                  .join(' · '),
                icon: PlusIcon,
                run: async () => {
                  open = false
                  await quickAdd(parsed, parentId)
                },
              },
              {
                // The same option bar as every other capture place, with this line already typed.
                key: 'add-review',
                label: 'Review in the capture window',
                hint: 'Dates, priority, labels',
                run: () => {
                  open = false
                  ui.capture(t.slice(1).trim())
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
              items: rows.slice(0, 50).map((r) =>
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
      const groups: Section[] = t.trim()
        ? groupHits(hits).map((g) => ({ title: KIND_LABEL[g.kind] ?? g.kind, items: g.hits.map(hitItem) }))
        : hits.length
          ? [{ title: 'Recent', items: hits.map(hitItem) }]
          : []
      const cmds = t.trim() ? await commandSection(t) : null
      out = cmds ? (cmds.lead ? [cmds.section, ...groups] : [...groups, cmds.section]) : groups
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
    class="pal-panel top-[16%] max-w-none min-w-0 w-[min(680px,calc(100%-2rem))] -translate-y-0 gap-0 overflow-hidden rounded-2xl border border-line-strong p-0 shadow-2 max-sm:top-3 max-sm:w-[calc(100%-1.5rem)]"
  >
    <Dialog.Title class="sr-only">Search and commands</Dialog.Title>

    <div class="flex h-12 items-center gap-3 border-b border-line px-4">
      <SearchIcon class="size-[18px] shrink-0 text-icon" aria-hidden="true" />
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
        class="h-full min-w-0 flex-1 border-0 bg-transparent text-[0.9375rem] text-text outline-none placeholder:text-muted"
      />
      <span
        class="shrink-0 rounded-md bg-sunk px-2 py-0.5 text-[0.6875rem] font-medium tracking-[0.02em] text-muted"
        aria-hidden="true">{modeBadge[mode]}</span
      >
    </div>

    <div class="max-h-[50vh] overflow-y-auto p-2">
      {#if error}
        <p role="alert" class="px-2.5 py-2 text-[0.8125rem] text-danger-text">{error}</p>
      {/if}
      <ul id="pal-list" role="listbox" aria-label={hints[mode]} class="flex flex-col">
        {#each sections as sec (sec.title)}
          <li
            role="presentation"
            class="px-2.5 pt-2.5 pb-1 text-[0.6875rem] font-semibold tracking-[0.08em] text-muted uppercase"
          >
            {sec.title}
          </li>
          {#each sec.items as it (it.key)}
            {@const i = flat.indexOf(it)}
            <li
              id="pal-{i}"
              role="option"
              aria-selected={i === active}
              onclick={() => it.run()}
              onkeydown={() => {}}
              onmousemove={() => (active = i)}
              class="relative flex h-9 cursor-pointer items-center gap-2.5 rounded-[7px] px-2.5 max-sm:h-11 aria-selected:bg-hover"
            >
              {#if i === active}
                <span class="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-accent" aria-hidden="true"></span>
              {/if}
              {#if it.icon}
                <it.icon class="size-[15px] shrink-0 text-icon" aria-hidden="true" />
              {/if}
              <span class="task-title-1 min-w-0 flex-1 truncate" title={it.label}>{it.label}</span>
              <span class="shrink-0 pl-2 text-[0.75rem] text-muted">{it.hint}</span>
            </li>
          {/each}
        {/each}
      </ul>
      {#if !sections.length && !error}
        <div class="flex flex-col items-center gap-3 px-4 py-8 text-center">
          <p class="text-[0.8125rem] text-muted">
            {text.trim() || mode !== 'search' ? 'Nothing found.' : 'Nothing opened yet. Start typing to find anything.'}
          </p>
          <p class="flex flex-wrap items-center justify-center gap-x-3 gap-y-1.5 text-[0.75rem] text-muted">
            <span><kbd class="pal-kbd">&gt;</kbd> actions</span>
            <span><kbd class="pal-kbd">+</kbd> add</span>
            <span><kbd class="pal-kbd">?</kbd> filter</span>
          </p>
        </div>
      {/if}
    </div>

    <div
      class="flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t border-line bg-bg/60 px-4 py-2 text-[0.6875rem] text-muted"
    >
      <span class="flex items-center gap-1.5"><kbd class="pal-kbd">↑↓</kbd> move</span>
      <span class="flex items-center gap-1.5"><kbd class="pal-kbd">↵</kbd> open</span>
      <span class="flex items-center gap-1.5"><kbd class="pal-kbd">esc</kbd> close</span>
      <span class="flex items-center gap-3 max-sm:ml-0 sm:ml-auto">
        <span class="flex items-center gap-1.5"><kbd class="pal-kbd">&gt;</kbd> actions</span>
        <span class="flex items-center gap-1.5"><kbd class="pal-kbd">+</kbd> add</span>
        <span class="flex items-center gap-1.5"><kbd class="pal-kbd">?</kbd> filter</span>
      </span>
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

<style>
  :global(.pal-kbd) {
    display: inline-flex;
    min-width: 1.25rem;
    align-items: center;
    justify-content: center;
    border-radius: 0.3125rem;
    border: 1px solid var(--line);
    background: var(--sunk);
    padding: 0 0.3125rem;
    font-family: inherit;
    font-size: 0.6875rem;
    line-height: 1.25rem;
    color: var(--muted);
  }
  /* Open motion: fade and scale in over 120ms; the app's [data-motion] rules and reduced motion disable it. */
  :global(.pal-panel[data-state='open']) {
    animation: pal-in 120ms ease-out both;
  }
  :global(body:has(.pal-panel) [data-slot='dialog-overlay']) {
    backdrop-filter: blur(3px);
    -webkit-backdrop-filter: blur(3px);
  }
  @keyframes pal-in {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.pal-panel[data-state='open']) {
      animation: none;
    }
  }
</style>
