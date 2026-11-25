<script lang="ts">
  // A cheat sheet for quick add with a live "try it" box. Click an example to load it; the result shows
  // exactly what the real parser reads, so the guide teaches by doing. Examples live in usageGuide.ts
  // and are tested against the parser.
  import { onMount } from 'svelte'
  import { dueLabel, durationLabel } from './captureEdit'
  import { goals as listGoals, labels, pages as listPages, tree, type Parsed } from './sync'
  import { guide } from './usageGuide'
  import { parse_quick_add } from '../../wasm/orbit_wasm'

  let draft = $state('call mom tomorrow 6pm !30m p2 #errand')
  let known = $state({ areas: [] as string[], goals: [] as string[], labels: [] as string[], pages: [] as string[] })

  onMount(async () => {
    const [t, all, gs, ps] = await Promise.all([tree(), labels(), listGoals(), listPages()])
    known = {
      areas: t.flatMap((x) => [x.area.name, ...x.projects.map((p) => p.name)]),
      goals: gs.map((g) => g.name),
      labels: all.map((l) => l.name),
      pages: ps.map((x) => x.title),
    }
  })

  const pad = (n: number) => String(n).padStart(2, '0')
  const localNow = () => {
    const d = new Date()
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
  }
  type Result = Parsed & { deadline?: string | null; duration_min?: number | null }
  const parsed: Result = $derived(JSON.parse(parse_quick_add(draft, localNow(), JSON.stringify(known))))

  // One row per thing the parser understood, in plain words.
  const rows = $derived.by(() => {
    const out: [string, string][] = []
    if (parsed.title) out.push(['Title', parsed.title])
    if (parsed.due) out.push(['Due', dueLabel(parsed.due)])
    if (parsed.deadline) out.push(['Deadline', parsed.deadline])
    if (parsed.duration_min) out.push(['Duration', durationLabel(parsed.duration_min)])
    if (parsed.priority) out.push(['Priority', `P${parsed.priority}`])
    if (parsed.place) out.push(['Area or Project', parsed.place])
    if (parsed.goal) out.push(['Goal', parsed.goal])
    if (parsed.page) out.push(['Page', parsed.page])
    if (parsed.labels.length) out.push(['Labels', parsed.labels.map((l) => `#${l}`).join(' ')])
    if (parsed.recurrence)
      out.push(['Repeats', `${parsed.recurrence.from_completion ? 'after completion: ' : ''}${parsed.recurrence.rule}`])
    if (parsed.reminders.length) out.push(['Reminders', String(parsed.reminders.length)])
    return out
  })
  // `code` spans in the guide text become <code>.
  const spans = (text: string) => text.split(/`([^`]+)`/).map((t, i) => ({ t, code: i % 2 === 1 }))
  const unresolved = $derived(parsed.tokens.filter((t) => !t.known).map((t) => t.text))
</script>

{#snippet rich(text: string)}{#each spans(text) as s, i (i)}{#if s.code}<code
        class="rounded bg-sunk px-1 font-mono text-[0.75rem]">{s.t}</code
      >{:else}{s.t}{/if}{/each}{/snippet}

<div class="flex flex-col gap-5">
  <div class="flex flex-col gap-2">
    <label for="guide-try" class="text-sm font-medium">Try it</label>
    <input
      id="guide-try"
      bind:value={draft}
      spellcheck="false"
      autocomplete="off"
      placeholder="Type a task the way you would say it"
      class="h-10 rounded-lg border border-line bg-surface px-3 text-[0.9375rem] focus-visible:border-accent-line"
    />
    <dl class="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-[0.8125rem]" aria-live="polite">
      {#each rows as [k, v] (k)}
        <dt class="text-muted">{k}</dt>
        <dd class="font-medium">{v}</dd>
      {:else}
        <dd class="col-span-2 text-muted">Nothing recognised yet. It would be added as a plain task.</dd>
      {/each}
    </dl>
    {#if unresolved.length}
      <p class="text-xs text-warn">Not found, kept in the title: {unresolved.join(', ')}</p>
    {/if}
    <p class="text-xs text-muted">Nothing is saved here. Your own Areas, Labels, Goals and Pages are used for names.</p>
  </div>

  {#each guide as g (g.id)}
    <section aria-labelledby="guide-{g.id}" class="flex flex-col gap-1.5">
      <h3 id="guide-{g.id}" class="text-sm font-semibold">{g.title}</h3>
      <p class="text-[0.8125rem] text-muted">
        {@render rich(g.intro)}
      </p>
      <ul class="flex flex-col divide-y divide-line rounded-lg border border-line">
        {#each g.examples as ex (ex.text)}
          <li class="flex flex-col gap-0.5 px-3 py-2 sm:flex-row sm:items-baseline sm:gap-3">
            <button
              type="button"
              class="w-fit rounded-md bg-sunk px-2 py-0.5 text-left font-mono text-[0.8125rem] hover:bg-hover"
              aria-label="Try: {ex.text}"
              onclick={() => (draft = ex.text)}>{ex.text}</button
            >
            <span class="text-[0.8125rem] text-muted">{@render rich(ex.means)}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/each}

  <section aria-labelledby="guide-more" class="flex flex-col gap-1.5">
    <h3 id="guide-more" class="text-sm font-semibold">Also good to know</h3>
    <ul class="list-disc pl-5 text-[0.8125rem] text-muted">
      <li>
        Click the buttons under the box in any capture window to set a date, priority, labels or reminder without
        typing.
      </li>
      <li>Paste several lines to add one task each. Indented lines become subtasks.</li>
      <li>
        In the app press <kbd class="rounded bg-sunk px-1">?</kbd> for keyboard shortcuts and
        <kbd class="rounded bg-sunk px-1">Ctrl K</kbd> for the command palette.
      </li>
      <li>From a terminal: <code class="rounded bg-sunk px-1">orbit add "call mom tomorrow 6pm"</code>.</li>
      <li>
        Words like “may” alone are not dates. Turn date recognition off under Quick add above if it gets in the way.
      </li>
    </ul>
  </section>
</div>
