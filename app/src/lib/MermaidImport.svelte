<script lang="ts">
  import { onDestroy } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { roadmapPrompt } from '$lib/roadmapPrompt'
  import {
    exportRoadmapCode,
    importRoadmapCode,
    parseMermaid,
    parseRoadmapCode,
    placeable,
    previewRoadmapCode,
    roadmapCode,
    tree,
    type MermaidPlan,
    type RoadmapCode,
    type RoadmapCodeDiff,
  } from '$lib/sync'

  let {
    onimport,
    roadmapId,
    onimportcode,
  }: {
    onimport: (plan: MermaidPlan) => void | Promise<void>
    /** Set where a Roadmap exists; enables the Orbit code mode for it. */
    roadmapId?: string
    onimportcode?: () => void | Promise<void>
  } = $props()

  let mode = $state<'mermaid' | 'orbit'>('mermaid')
  let text = $state('')
  let plan = $state<MermaidPlan | null>(null)
  let error = $state<{ line: number; message: string } | null>(null)
  let busy = $state(false)
  let timer: ReturnType<typeof setTimeout> | undefined

  function update(value: string) {
    text = value
    clearTimeout(timer)
    timer = setTimeout(() => {
      if (!text.trim()) {
        plan = null
        error = null
        return
      }
      const result = parseMermaid(text)
      if ('ok' in result) {
        plan = result.ok
        error = null
      } else {
        plan = null
        error = result.error
      }
    }, 150)
  }

  const count = $derived(plan?.phases.reduce((n, p) => n + p.items.length, 0) ?? 0)
  async function run() {
    if (!plan || busy) return
    busy = true
    try {
      await onimport(plan)
    } finally {
      busy = false
    }
  }

  let code = $state('')
  let parsed = $state<RoadmapCode | null>(null)
  let codeError = $state<{ line: number; message: string } | null>(null)
  let preview = $state<{ diff: RoadmapCodeDiff; errors: { line: number; message: string }[] } | null>(null)
  let failure = $state<string | null>(null)
  let copied = $state<'ai' | 'code' | null>(null)
  let codeTimer: ReturnType<typeof setTimeout> | undefined
  let copiedTimer: ReturnType<typeof setTimeout> | undefined

  function updateCode(value: string) {
    code = value
    clearTimeout(codeTimer)
    codeTimer = setTimeout(() => void checkCode(value), 150)
  }
  /** Parses and diffs the text against the Roadmap; a stale check (text changed meanwhile) is dropped. */
  async function checkCode(value: string) {
    parsed = null
    codeError = null
    preview = null
    failure = null
    if (!value.trim() || !roadmapId) return
    const result = parseRoadmapCode(value)
    if (!('ok' in result)) {
      codeError = result.error
      return
    }
    try {
      const next = await previewRoadmapCode(roadmapId, result.ok)
      if (value !== code) return
      parsed = result.ok
      preview = next
    } catch (e) {
      if (value === code) failure = e instanceof Error ? e.message : String(e)
    }
  }
  async function runCode() {
    if (!parsed || !roadmapId || !preview || preview.errors.length || busy) return
    busy = true
    try {
      await importRoadmapCode(roadmapId, parsed)
      code = ''
      parsed = null
      preview = null
      await onimportcode?.()
    } catch (e) {
      failure = e instanceof Error ? e.message : String(e)
    } finally {
      busy = false
    }
  }

  async function copy(which: 'ai' | 'code', payload: string) {
    try {
      await navigator.clipboard.writeText(payload)
      copied = which
      clearTimeout(copiedTimer)
      copiedTimer = setTimeout(() => (copied = null), 2000)
    } catch (e) {
      failure = e instanceof Error ? e.message : String(e)
    }
  }
  async function copyCode() {
    if (!roadmapId) return
    await copy('code', exportRoadmapCode(await roadmapCode(roadmapId)))
  }
  /** Prompt for an AI: the format, the closed kind lists, and the existing ids it may reference. */
  async function copyPrompt() {
    const [areas, items] = await Promise.all([tree(), placeable()])
    const of = (kind: string) =>
      items.filter((item) => item.kind === kind).map((item) => ({ id: item.id, title: item.title }))
    await copy(
      'ai',
      roadmapPrompt(
        areas.map(({ area }) => ({ id: area.id, title: area.name })),
        of('project'),
        of('goal'),
        of('task'),
      ),
    )
  }
  onDestroy(() => {
    clearTimeout(timer)
    clearTimeout(codeTimer)
    clearTimeout(copiedTimer)
  })
</script>

{#if roadmapId}
  <div class="flex gap-2 text-[0.8125rem]" role="group" aria-label="Import format">
    <Button variant={mode === 'mermaid' ? 'primary' : 'ghost'} size="sm" onclick={() => (mode = 'mermaid')}
      >Mermaid</Button
    >
    <Button variant={mode === 'orbit' ? 'primary' : 'ghost'} size="sm" onclick={() => (mode = 'orbit')}
      >Orbit code</Button
    >
  </div>
{/if}

{#if mode === 'mermaid' || !roadmapId}
  <div class="flex flex-col gap-2">
    <textarea
      aria-label="Mermaid diagram"
      placeholder={'gantt\n  section Plan\n  Task :a, 2026-01-01, 3d'}
      value={text}
      oninput={(e) => update(e.currentTarget.value)}
      rows="10"
      spellcheck="false"
      class="w-full rounded-md border border-line bg-surface p-2 font-mono text-[0.8125rem]"
    ></textarea>
    {#if error}
      <p role="alert" class="rounded-md bg-danger-soft px-2 py-1 text-[0.8125rem] text-danger-text">
        Line {error.line}: {error.message}
      </p>
    {:else if plan}
      <ul class="flex flex-col gap-1 text-[0.8125rem]" aria-label="Preview">
        {#each plan.phases as p, n (n)}
          <li class="flex items-center gap-2 rounded-md border border-line px-2 py-1">
            <span class="min-w-0 flex-1 truncate font-medium">{p.name}</span>
            <span class="text-muted">{p.items.length} {p.items.length === 1 ? 'item' : 'items'}</span>
            {#if p.start || p.end}
              <span class="text-muted">{[p.start, p.end].filter(Boolean).join(' – ')}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    <div class="flex justify-end">
      <Button variant="primary" disabled={!plan || !!error || !count || busy} onclick={run}>Import</Button>
    </div>
  </div>
{:else}
  <div class="flex flex-col gap-2">
    <textarea
      aria-label="Orbit code"
      placeholder={'%%orbit:v1\nflowchart LR\n  a["Read the Rust book"]:::task'}
      value={code}
      oninput={(e) => updateCode(e.currentTarget.value)}
      rows="10"
      spellcheck="false"
      class="w-full rounded-md border border-line bg-surface p-2 font-mono text-[0.8125rem]"
    ></textarea>
    {#if codeError}
      <p role="alert" class="rounded-md bg-danger-soft px-2 py-1 text-[0.8125rem] text-danger-text">
        Line {codeError.line}: {codeError.message}
      </p>
    {:else if preview?.errors.length}
      <ul
        role="alert"
        class="flex flex-col gap-1 rounded-md bg-danger-soft px-2 py-1 text-[0.8125rem] text-danger-text"
      >
        {#each preview.errors as err, n (n)}
          <li>{err.message}</li>
        {/each}
      </ul>
    {:else if preview}
      <ul class="flex flex-col gap-1 text-[0.8125rem]" aria-label="Preview">
        <li class="rounded-md border border-line px-2 py-1">
          Nodes: {preview.diff.createNodes} new, {preview.diff.updateNodes} updated
        </li>
        <li class="rounded-md border border-line px-2 py-1">
          Edges: {preview.diff.createEdges} new, {preview.diff.updateEdges} updated
        </li>
        <li class="rounded-md border border-line px-2 py-1">
          Bands: {preview.diff.createBands} new, {preview.diff.updateBands} updated
        </li>
      </ul>
    {/if}
    {#if failure}
      <p role="alert" class="rounded-md bg-danger-soft px-2 py-1 text-[0.8125rem] text-danger-text">{failure}</p>
    {/if}
    <div class="flex flex-wrap items-center justify-end gap-2">
      <Button variant="ghost" onclick={copyPrompt}>{copied === 'ai' ? 'Copied' : 'Copy for AI'}</Button>
      <Button variant="ghost" onclick={copyCode}>{copied === 'code' ? 'Copied' : 'Copy as code'}</Button>
      <Button variant="primary" disabled={!parsed || !preview || preview.errors.length > 0 || busy} onclick={runCode}
        >Import</Button
      >
    </div>
  </div>
{/if}
