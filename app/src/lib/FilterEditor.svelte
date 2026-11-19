<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import {
    labels,
    parseFilter,
    printFilter,
    updateView,
    type Cond,
    type FilterError,
    type Label,
    type Node,
  } from '$lib/sync'

  let {
    id,
    filter,
    onclose,
    onapplied,
  }: {
    id: string
    filter: Node
    onclose: () => void
    onapplied: () => void
  } = $props()

  type Group = Extract<Node, { type: 'group' }>

  // A deep copy so edits stay local until Apply; a bare condition is edited as a one-child group.
  // JSON data only: the filter can be a Svelte state proxy, which structuredClone refuses.
  const clone = (n: Node): Node => JSON.parse(JSON.stringify(n))
  // svelte-ignore state_referenced_locally
  let tree = $state<Node>(
    filter.type === 'group' ? clone(filter) : { type: 'group', all: true, children: [clone(filter)] },
  )

  let labelList = $state<Label[]>([])
  labels().then((l) => (labelList = l))

  const paths = [
    'title',
    'priority',
    'due',
    'deadline',
    'label',
    'completed',
    'created_at',
    'touched_at',
    'parent',
    'project.name',
    'area.name',
    'goal.name',
  ]
  const datePaths = new Set(['due', 'deadline', 'created_at', 'touched_at'])
  const ops: [string, string][] = [
    ['is', 'is'],
    ['is_not', 'is not'],
    ['contains', 'contains'],
    ['is_empty', 'is empty'],
    ['before', 'before'],
    ['after', 'after'],
    ['within', 'within'],
  ]

  const newCond = (): Cond => ({ type: 'cond', path: 'title', op: 'is', value: '' })
  const newGroup = (): Group => ({ type: 'group', all: true, children: [] })

  function setPath(c: Cond, path: string) {
    c.path = path
    if (c.op === 'within') c.value = { from: '', to: '' }
    else
      c.value =
        path === 'completed' ? false : path === 'priority' ? 1 : path === 'label' ? (labelList[0]?.id ?? '') : ''
  }
  function setOp(c: Cond, op: string) {
    c.op = op
    c.value = op === 'within' ? { from: '', to: '' } : c.path === 'completed' ? false : c.path === 'priority' ? 1 : ''
  }
  function setWithin(c: Cond, key: 'from' | 'to', value: string) {
    const cur = c.value && typeof c.value === 'object' && !Array.isArray(c.value) ? c.value : {}
    c.value = { ...(cur as Record<string, unknown>), [key]: value }
  }
  function bound(c: Cond, key: 'from' | 'to') {
    const v = c.value
    return v && typeof v === 'object' && !Array.isArray(v) ? String((v as Record<string, unknown>)[key] ?? '') : ''
  }

  const fieldClass =
    'h-7 min-w-0 rounded-md border border-line bg-surface px-1.5 text-[13px] text-text shadow-1 outline-none focus-visible:border-accent-line'

  let typed = $state(false)
  let text = $state('')
  let error = $state<FilterError | null>(null)

  async function show(toTyped: boolean) {
    if (toTyped === typed) return
    if (toTyped) {
      text = await printFilter(JSON.parse(JSON.stringify(tree)))
      error = null
      typed = true
    } else if (await parseText()) typed = false
  }
  // Parses the typed text into the tree; on a bad token it keeps the text and shows where.
  async function parseText() {
    try {
      const n = await parseFilter(text)
      tree = n.type === 'group' ? n : { type: 'group', all: true, children: [n] }
      error = null
      return true
    } catch (e) {
      error = e as FilterError
      return false
    }
  }
  const chars = $derived([...text])

  async function apply() {
    if (typed && !(await parseText())) return
    await updateView(id, { filter: JSON.parse(JSON.stringify(tree)) })
    onapplied()
    onclose()
  }
</script>

{#snippet groupEditor(g: Group, depth: number, onremove?: () => void)}
  <div class="mb-2 rounded-lg border border-line bg-sunk p-2">
    <div class="mb-2 flex flex-wrap items-center gap-1.5">
      <select
        aria-label="Match"
        value={g.all ? 'all' : 'any'}
        onchange={(e) => (g.all = e.currentTarget.value === 'all')}
        class={fieldClass}
      >
        <option value="all">Match all</option>
        <option value="any">Match any</option>
      </select>
      <span class="flex-1"></span>
      {#if onremove}
        <Button size="sm" variant="ghost" aria-label="Remove group" onclick={onremove}>Remove</Button>
      {/if}
    </div>

    {#each g.children as child, i (i)}
      {#if child.type === 'group'}
        {@render groupEditor(child, depth + 1, () => g.children.splice(i, 1))}
      {:else}
        <div class="mb-1.5 flex flex-wrap items-center gap-1.5">
          <select
            aria-label="Field"
            value={child.path}
            onchange={(e) => setPath(child, e.currentTarget.value)}
            class={fieldClass}
          >
            {#each paths as p (p)}
              <option value={p}>{p.replaceAll('_', ' ').replaceAll('.', ' ')}</option>
            {/each}
          </select>
          <select
            aria-label="Operator"
            value={child.op}
            onchange={(e) => setOp(child, e.currentTarget.value)}
            class={fieldClass}
          >
            {#each ops as [value, text] (value)}
              <option {value}>{text}</option>
            {/each}
          </select>

          {#if child.op !== 'is_empty'}
            {#if child.op === 'within'}
              <input
                aria-label="From"
                placeholder="today"
                value={bound(child, 'from')}
                oninput={(e) => setWithin(child, 'from', e.currentTarget.value)}
                class="{fieldClass} w-24"
              />
              <span class="text-xs text-muted">to</span>
              <input
                aria-label="To"
                placeholder="+7d"
                value={bound(child, 'to')}
                oninput={(e) => setWithin(child, 'to', e.currentTarget.value)}
                class="{fieldClass} w-24"
              />
            {:else if child.path === 'priority'}
              <input
                type="number"
                min="1"
                max="4"
                aria-label="Value"
                value={child.value == null ? '' : String(child.value)}
                oninput={(e) => (child.value = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))}
                class="{fieldClass} w-16"
              />
            {:else if child.path === 'completed'}
              <select
                aria-label="Value"
                value={child.value === true ? 'true' : 'false'}
                onchange={(e) => (child.value = e.currentTarget.value === 'true')}
                class={fieldClass}
              >
                <option value="false">open</option>
                <option value="true">done</option>
              </select>
            {:else if child.path === 'label'}
              <select
                aria-label="Value"
                value={String(child.value ?? '')}
                onchange={(e) => (child.value = e.currentTarget.value)}
                class={fieldClass}
              >
                <option value="">Choose label</option>
                {#each labelList as l (l.id)}
                  <option value={l.id}>{l.name}</option>
                {/each}
              </select>
            {:else}
              <input
                aria-label="Value"
                placeholder={datePaths.has(child.path) ? 'today, +7d, 2026-12-01' : ''}
                value={String(child.value ?? '')}
                oninput={(e) => (child.value = e.currentTarget.value)}
                class="{fieldClass} min-w-32 flex-1"
              />
            {/if}
          {/if}

          <Button size="sm" variant="ghost" aria-label="Remove condition" onclick={() => g.children.splice(i, 1)}>
            Remove
          </Button>
        </div>
      {/if}
    {/each}

    <div class="flex flex-wrap items-center gap-1.5">
      <Button size="sm" onclick={() => g.children.push(newCond())}>Add condition</Button>
      <Button size="sm" disabled={depth >= 3} onclick={() => g.children.push(newGroup())}>Add group</Button>
    </div>
  </div>
{/snippet}

<div
  role="dialog"
  aria-label="Filter"
  class="absolute top-full left-0 z-30 mt-1 max-h-[70vh] w-[min(640px,calc(100vw-2rem))] overflow-y-auto rounded-xl border border-line bg-surface p-3 shadow-2"
>
  <div class="mb-2 flex gap-1" role="group" aria-label="Editor">
    <Button size="sm" variant={typed ? 'ghost' : 'on'} aria-pressed={!typed} onclick={() => show(false)}>Visual</Button>
    <Button size="sm" variant={typed ? 'on' : 'ghost'} aria-pressed={typed} onclick={() => show(true)}>Typed</Button>
  </div>
  {#if typed}
    <textarea
      aria-label="Typed filter"
      rows="3"
      spellcheck="false"
      bind:value={text}
      oninput={() => (error = null)}
      placeholder="p1 & #Career & due before: next week"
      class="{fieldClass} h-auto w-full resize-y py-1.5 font-mono"
    ></textarea>
    {#if error}
      <p role="alert" class="mt-1 font-mono text-xs break-all text-danger-text">
        {chars.slice(0, error.start).join('')}<mark class="rounded bg-danger-soft text-danger-text"
          >{chars.slice(error.start, Math.max(error.end, error.start + 1)).join('')}</mark
        >{chars.slice(error.end).join('')}
      </p>
      <p class="text-xs text-danger-text">{error.message}</p>
    {/if}
  {:else if tree.type === 'group'}
    {@render groupEditor(tree, 1)}
  {/if}
  <div class="mt-1 flex items-center justify-end gap-2">
    <Button variant="ghost" onclick={onclose}>Cancel</Button>
    <Button variant="primary" onclick={apply}>Apply</Button>
  </div>
</div>
