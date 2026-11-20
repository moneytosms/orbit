<script lang="ts">
  import { onMount } from 'svelte'
  import { Calendar } from 'fullcalendar'
  import dayGrid from 'fullcalendar/daygrid'
  import timeGrid from 'fullcalendar/timegrid'
  import interaction from 'fullcalendar/interaction'
  import classicTheme from 'fullcalendar/themes/classic'
  import 'fullcalendar/skeleton.css'
  import 'fullcalendar/themes/classic/theme.css'
  import 'fullcalendar/themes/classic/palette.css'
  import * as Dialog from '$lib/components/ui/dialog'
  import * as Sheet from '$lib/components/ui/sheet'
  import Composer from './Composer.svelte'
  import type { BoardRow } from '$lib/Board.svelte'
  import { minutesBetween, toEvents } from '$lib/calendar'
  import { calendarData, getWeekStart, moveTask, updateTask, type Task } from '$lib/sync'
  import { inbox_id } from '../../wasm/orbit_wasm'

  let {
    rows,
    onchange,
    onopen,
  }: {
    rows: BoardRow[]
    onchange: () => void
    onopen: (id: string) => void
  } = $props()

  const tasks = $derived(rows.filter((r): r is Task & BoardRow => r.kind === 'task') as Task[])
  let firstDay = $state<number | null>(null)
  let cal: Calendar | undefined
  let range = $state<{ from: string; to: string } | null>(null)
  let adding = $state<string | null>(null)
  const wide = () => matchMedia('(min-width: 701px)').matches

  onMount(async () => {
    firstDay = await getWeekStart()
  })

  const day = (s: string) => s.slice(0, 10)
  const minute = (s: string) => s.slice(0, 16)

  async function dropped(info: {
    event: { startStr: string; allDay: boolean; extendedProps: Record<string, unknown> }
    revert: () => void
  }) {
    const { event } = info
    await moveTask(
      event.extendedProps.id as string,
      day(event.startStr),
      event.allDay ? null : minute(event.startStr).slice(11),
    )
    onchange()
  }
  async function resized(info: {
    event: { startStr: string; endStr: string; extendedProps: Record<string, unknown> }
  }) {
    const { event } = info
    if (!event.endStr) return
    const duration_min = minutesBetween(minute(event.startStr), minute(event.endStr))
    if (duration_min > 0) await updateTask(event.extendedProps.id as string, { duration_min })
    onchange()
  }
  function clicked(kind: string, id: string) {
    if (kind === 'task' || kind === 'deadline') onopen(id)
    else location.hash = kind === 'routine' ? '#/routines' : '#/goals'
  }

  // Month first on desk, Day below the `desk:` breakpoint.
  function mount(el: HTMLElement) {
    const c = new Calendar(el, {
      plugins: [dayGrid, timeGrid, interaction, classicTheme],
      initialView: wide() ? 'dayGridMonth' : 'timeGridDay',
      headerToolbar: { left: 'prev,next today', center: 'title', right: 'dayGridMonth,timeGridWeek,timeGridDay' },
      firstDay: firstDay ?? 1,
      height: 'auto',
      nowIndicator: true,
      editable: true,
      eventDurationEditable: true,
      longPressDelay: 300,
      eventLongPressDelay: 300,
      dateClick: (info) =>
        (adding = info.allDay ? `${day(info.dateStr)} ` : `${day(info.dateStr)} ${minute(info.dateStr).slice(11)} `),
      eventDrop: dropped,
      eventResize: resized,
      eventClick: (info) => clicked(info.event.extendedProps.kind, info.event.extendedProps.id),
      datesSet: (info) => {
        const from = day(info.startStr)
        const to = day(info.endStr)
        if (range?.from !== from || range?.to !== to) range = { from, to }
      },
    })
    c.render()
    cal = c
    return () => {
      c.destroy()
      cal = undefined
    }
  }

  $effect(() => {
    const r = range
    const t = tasks
    if (!r || !cal) return
    let stale = false
    calendarData(r.from, r.to).then((d) => {
      if (stale) return
      cal?.setOption('events', toEvents(t, d.routines, d.goals, d.milestones))
    })
    return () => (stale = true)
  })
</script>

{#snippet composer()}
  <Dialog.Title class="sr-only">New task</Dialog.Title>
  <Composer prefill={adding ?? ''} parentId={inbox_id()} onsaved={() => ((adding = null), onchange())} />
{/snippet}

{#if firstDay !== null}
  <div class="orbit-cal" {@attach mount}></div>
{/if}

{#if adding !== null}
  {#if wide()}
    <Dialog.Root open onOpenChange={(o) => !o && (adding = null)}>
      <Dialog.Content>{@render composer()}</Dialog.Content>
    </Dialog.Root>
  {:else}
    <Sheet.Root open onOpenChange={(o) => !o && (adding = null)}>
      <Sheet.Content side="bottom" class="p-4 pb-[calc(1rem+env(safe-area-inset-bottom))]">
        {@render composer()}
      </Sheet.Content>
    </Sheet.Root>
  {/if}
{/if}
