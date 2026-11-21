<script lang="ts">
  import { onDestroy } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { parseMermaid, type MermaidPlan } from '$lib/sync'

  let { onimport }: { onimport: (plan: MermaidPlan) => void | Promise<void> } = $props()

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
  onDestroy(() => clearTimeout(timer))

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
</script>

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
