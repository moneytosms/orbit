<script lang="ts">
  // The visual side of quick capture: chips for destination, date, priority, labels and reminder that
  // show what the typed line means and let you set it by clicking. Every pick rewrites the typed text
  // (see captureEdit.ts), so the line stays the one source of truth in the Composer, the capture window
  // and anywhere else this is used.
  import Bell from '@lucide/svelte/icons/bell'
  import Calendar from '@lucide/svelte/icons/calendar'
  import CalendarClock from '@lucide/svelte/icons/calendar-clock'
  import Flag from '@lucide/svelte/icons/flag'
  import Hourglass from '@lucide/svelte/icons/hourglass'
  import Inbox from '@lucide/svelte/icons/inbox'
  import Tag from '@lucide/svelte/icons/tag'
  import Target from '@lucide/svelte/icons/target'
  import type { Component } from 'svelte'
  import MonthCalendar from './MonthCalendar.svelte'
  import {
    dateQuick,
    dateText,
    deadlineLabel,
    deadlineQuick,
    deadlineText,
    dueLabel,
    durationLabel,
    durationQuick,
    durationText,
    reminderQuick,
    setToken,
    toggleToken,
  } from './captureEdit'
  import type { Parsed } from './sync'

  type Known = { areas: string[]; goals: string[]; labels: string[]; pages: string[] }
  type Panel = 'place' | 'date' | 'deadline' | 'duration' | 'priority' | 'labels' | 'goal' | 'reminder'

  let {
    draft = $bindable(),
    parsed,
    known,
    onedit,
  }: { draft: string; parsed: Parsed; known: Known; onedit?: () => void } = $props()

  let open: Panel | null = $state(null)
  let pickDate = $state('')
  let pickTime = $state('')
  let customMin = $state('')

  const toggle = (p: Panel) => (open = open === p ? null : p)
  // A pick edits the line; the input keeps focus so typing carries on.
  function apply(next: string, close = true) {
    draft = next
    if (close) open = null
    onedit?.()
  }
  const set = (kind: Parsed['tokens'][number]['kind'], text: string, close = true) =>
    apply(setToken(draft, parsed.tokens, kind, text), close)

  const has = (kind: Parsed['tokens'][number]['kind']) => parsed.tokens.some((t) => t.kind === kind && t.known)
  const reminders = $derived(parsed.tokens.filter((t) => t.kind === 'reminder'))

  function applyPicked() {
    if (!pickDate && !pickTime) return
    // A time alone means the next time the clock shows it; a date alone has no time.
    const text = pickDate ? dateText(pickDate, pickTime || undefined) : pickTime
    set('date', text)
    pickDate = pickTime = ''
  }

  type Chip = { id: Panel; label: string; icon: Component; active: boolean; show: boolean }
  const chips: Chip[] = $derived([
    { id: 'place', label: parsed.place ?? 'Inbox', icon: Inbox, active: !!parsed.place, show: true },
    {
      id: 'date',
      label: parsed.due ? dueLabel(parsed.due) : 'Date',
      icon: Calendar,
      active: !!parsed.due,
      show: true,
    },
    {
      id: 'deadline',
      label: parsed.deadline ? `By ${deadlineLabel(parsed.deadline)}` : 'Deadline',
      icon: CalendarClock,
      active: !!parsed.deadline,
      show: true,
    },
    {
      id: 'duration',
      label: parsed.duration_min ? durationLabel(parsed.duration_min) : 'Duration',
      icon: Hourglass,
      active: !!parsed.duration_min,
      show: true,
    },
    {
      id: 'priority',
      label: parsed.priority ? `P${parsed.priority}` : 'Priority',
      icon: Flag,
      active: !!parsed.priority,
      show: true,
    },
    {
      id: 'labels',
      label: parsed.labels.length ? parsed.labels.map((l) => `#${l}`).join(' ') : 'Labels',
      icon: Tag,
      active: parsed.labels.length > 0,
      show: true,
    },
    { id: 'goal', label: parsed.goal ?? 'Goal', icon: Target, active: !!parsed.goal, show: known.goals.length > 0 },
    {
      id: 'reminder',
      label: reminders.length ? `${reminders.length} reminder${reminders.length > 1 ? 's' : ''}` : 'Reminder',
      icon: Bell,
      active: reminders.length > 0,
      show: true,
    },
  ])

  const chipClass = (active: boolean, on: boolean) =>
    `flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs transition-colors ${
      on
        ? 'border-accent-line bg-sunk text-accent-text'
        : active
          ? 'border-accent-line text-accent-text'
          : 'border-line text-muted'
    } hover:bg-hover`
  const optClass = 'rounded-md bg-sunk px-2 py-1 text-[0.8125rem] hover:bg-hover'
  const optOn = 'rounded-md bg-hover px-2 py-1 text-[0.8125rem] font-medium text-accent-text'
</script>

<div class="flex flex-col gap-1.5" role="group" aria-label="Task options">
  <ul class="flex flex-wrap gap-1.5">
    {#each chips.filter((c) => c.show) as c (c.id)}
      <li>
        <!-- mousedown is held back so the input keeps focus and the caret. -->
        <button
          type="button"
          class={chipClass(c.active, open === c.id)}
          aria-expanded={open === c.id}
          aria-label={c.id === 'place' ? `Destination: ${c.label}` : c.label}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => toggle(c.id)}
        >
          <c.icon size={13} aria-hidden="true" />
          <span class="max-w-40 truncate">{c.label}</span>
        </button>
      </li>
    {/each}
  </ul>

  {#if open === 'place'}
    <div class="flex max-h-32 flex-wrap gap-1 overflow-y-auto" aria-label="Destination">
      <button type="button" class={!parsed.place ? optOn : optClass} onclick={() => set('place', '')}>Inbox</button>
      {#each known.areas as n (n)}
        <button type="button" class={parsed.place === n ? optOn : optClass} onclick={() => set('place', `@${n}`)}
          >@{n}</button
        >
      {/each}
    </div>
  {:else if open === 'date'}
    <div class="flex flex-col gap-2" aria-label="Date">
      <div class="flex flex-wrap items-center gap-1">
        {#each dateQuick as q (q.text)}
          <button type="button" class={optClass} onclick={() => set('date', q.text)}>{q.label}</button>
        {/each}
        {#if parsed.due}
          <button type="button" class={optClass} onclick={() => set('date', '')}>No date</button>
        {/if}
      </div>
      <MonthCalendar
        selected={parsed.due?.date}
        pending={pickDate}
        initial={parsed.due?.date}
        onpick={(d) => {
          pickDate = d
          applyPicked()
        }}
      />
      <div class="flex flex-wrap items-center gap-2 text-[0.8125rem]">
        <input
          type="time"
          bind:value={pickTime}
          aria-label="Pick a time"
          class="h-8 rounded-md border border-line bg-surface px-2"
        />
        <button type="button" class={optClass} disabled={!pickDate && !pickTime} onclick={applyPicked}>Set</button>
      </div>
    </div>
  {:else if open === 'deadline'}
    <div class="flex flex-col gap-2" aria-label="Deadline">
      <div class="flex flex-wrap items-center gap-1">
        {#each deadlineQuick as q (q.text)}
          <button type="button" class={optClass} onclick={() => set('deadline', deadlineText(q.text))}>{q.label}</button
          >
        {/each}
        {#if parsed.deadline}
          <button type="button" class={optClass} onclick={() => set('deadline', '')}>No deadline</button>
        {/if}
      </div>
      <MonthCalendar
        selected={parsed.deadline ?? ''}
        initial={parsed.deadline ?? parsed.due?.date}
        onpick={(d) => set('deadline', deadlineText(d))}
      />
    </div>
  {:else if open === 'duration'}
    <div class="flex flex-col gap-2" aria-label="Duration">
      <div class="flex flex-wrap items-center gap-1">
        {#each durationQuick as q (q.minutes)}
          <button
            type="button"
            aria-pressed={parsed.duration_min === q.minutes}
            class={parsed.duration_min === q.minutes ? optOn : optClass}
            onclick={() => set('duration', durationText(q.minutes))}>{q.label}</button
          >
        {/each}
        {#if parsed.duration_min}
          <button type="button" class={optClass} onclick={() => set('duration', '')}>No duration</button>
        {/if}
      </div>
      <form
        class="flex flex-wrap items-center gap-2 text-[0.8125rem]"
        onsubmit={(e) => {
          e.preventDefault()
          const n = Math.round(Number(customMin))
          if (n >= 1 && n <= 1440) {
            set('duration', durationText(n))
            customMin = ''
          }
        }}
      >
        <input
          type="number"
          min="1"
          max="1440"
          inputmode="numeric"
          bind:value={customMin}
          aria-label="Custom duration in minutes"
          placeholder="Minutes"
          class="h-8 w-24 rounded-md border border-line bg-surface px-2"
        />
        <button type="submit" class={optClass} disabled={!(Number(customMin) >= 1 && Number(customMin) <= 1440)}
          >Set</button
        >
      </form>
    </div>
  {:else if open === 'priority'}
    <div class="flex flex-wrap gap-1" aria-label="Priority">
      {#each [1, 2, 3, 4] as n (n)}
        <button type="button" class={parsed.priority === n ? optOn : optClass} onclick={() => set('priority', `p${n}`)}
          >P{n}</button
        >
      {/each}
      {#if parsed.priority}
        <button type="button" class={optClass} onclick={() => set('priority', '')}>None</button>
      {/if}
    </div>
  {:else if open === 'labels'}
    <div class="flex max-h-32 flex-wrap gap-1 overflow-y-auto" aria-label="Labels">
      {#each known.labels as n (n)}
        <button
          type="button"
          aria-pressed={parsed.labels.includes(n)}
          class={parsed.labels.includes(n) ? optOn : optClass}
          onclick={() => apply(toggleToken(draft, parsed.tokens, 'label', n, `#${n}`), false)}>#{n}</button
        >
      {:else}
        <p class="text-xs text-muted">No labels yet. Type #name to add one.</p>
      {/each}
    </div>
  {:else if open === 'goal'}
    <div class="flex flex-wrap gap-1" aria-label="Goal">
      {#each known.goals as n (n)}
        <button type="button" class={parsed.goal === n ? optOn : optClass} onclick={() => set('goal', `$${n}`)}
          >${n}</button
        >
      {/each}
      {#if has('goal')}
        <button type="button" class={optClass} onclick={() => set('goal', '')}>None</button>
      {/if}
    </div>
  {:else if open === 'reminder'}
    <div class="flex flex-wrap gap-1" aria-label="Reminder">
      {#each reminderQuick as r (r.minutes)}
        <button
          type="button"
          aria-pressed={reminders.some((t) => t.value === r.minutes)}
          class={reminders.some((t) => t.value === r.minutes) ? optOn : optClass}
          onclick={() => apply(toggleToken(draft, parsed.tokens, 'reminder', r.minutes, r.text), false)}
          >{r.label}</button
        >
      {/each}
      {#if !parsed.due}
        <p class="w-full text-xs text-muted">Reminders before a due time need a date and time.</p>
      {/if}
    </div>
  {/if}
</div>
