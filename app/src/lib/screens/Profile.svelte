<script lang="ts">
  import { setTargets, setVacation, stardust, type Stardust } from '$lib/sync'

  let { version = 0 }: { version?: number } = $props()
  let data = $state<Stardust>()

  $effect(() => {
    void version
    stardust().then((d) => (data = d))
  })

  const R = 44
  const C = 2 * Math.PI * R
  const frac = $derived.by(() => {
    if (!data || data.level.next === null) return 1
    const { floor, next } = data.level
    return Math.min(1, (data.total - floor) / (next - floor))
  })
  const max = $derived(Math.max(1, ...(data?.history.map((h) => h.points) ?? [])))
  const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
  const num = (e: Event) => Math.max(1, Math.floor(Number((e.target as HTMLInputElement).value) || 1))
  const toggleOff = (d: number) => {
    if (!data) return
    const off = data.targets.days_off
    setTargets({ days_off: off.includes(d) ? off.filter((x) => x !== d) : [...off, d].sort() })
  }
</script>

<h1 class="page-title">Profile</h1>
{#if data}
  <div class="flex flex-col items-center gap-1 px-3 py-4 text-center">
    <div class="relative size-28">
      <svg viewBox="0 0 100 100" class="size-28 -rotate-90" aria-hidden="true">
        <circle
          cx="50"
          cy="50"
          r={R}
          fill="none"
          stroke="currentColor"
          class="text-muted"
          opacity="0.25"
          stroke-width="6"
        />
        <circle
          cx="50"
          cy="50"
          r={R}
          fill="none"
          stroke="currentColor"
          class="text-accent"
          stroke-width="6"
          stroke-linecap="round"
          stroke-dasharray={C}
          stroke-dashoffset={C * (1 - frac)}
        />
      </svg>
      <span class="absolute inset-0 flex items-center justify-center text-3xl font-semibold">O</span>
    </div>
    <p class="text-lg font-semibold">{data.level.name} · Level {data.level.n}</p>
    <p class="text-muted">
      {data.total.toLocaleString()} Stardust{data.level.next === null
        ? ''
        : ` · ${(data.level.next - data.total).toLocaleString()} to next Level`}
    </p>
  </div>

  <section class="panel" aria-label="Targets">
    <h2 class="panel-title">Targets</h2>
    <p>Today: {data.targets.today_done} / {data.targets.daily}</p>
    <p>This week: {data.targets.week_done} / {data.targets.weekly}</p>
    <p>Target streak: {data.targets.streak} {data.targets.streak === 1 ? 'day' : 'days'}</p>
    <div class="mt-2 flex flex-wrap items-center gap-3">
      <label class="flex items-center gap-1.5">
        Daily
        <input
          type="number"
          min="1"
          class="w-16 rounded border px-1.5 py-0.5"
          value={data.targets.daily}
          onchange={(e) => setTargets({ target_daily: num(e) })}
        />
      </label>
      <label class="flex items-center gap-1.5">
        Weekly
        <input
          type="number"
          min="1"
          class="w-16 rounded border px-1.5 py-0.5"
          value={data.targets.weekly}
          onchange={(e) => setTargets({ target_weekly: num(e) })}
        />
      </label>
    </div>
    <div class="mt-2 flex flex-wrap items-center gap-3">
      <span>Days off</span>
      {#each DAYS as name, d (d)}
        <label class="flex items-center gap-1">
          <input type="checkbox" checked={data.targets.days_off.includes(d)} onchange={() => toggleOff(d)} />
          {name}
        </label>
      {/each}
    </div>
    <label class="mt-2 flex items-center gap-1.5">
      <input type="checkbox" checked={data.targets.vacation} onchange={(e) => setVacation(e.currentTarget.checked)} />
      Vacation
    </label>
  </section>

  <section class="panel" aria-label="Stardust history">
    <h2 class="panel-title">Last 30 days</h2>
    <svg viewBox="0 0 300 80" class="h-24 w-full text-accent" role="img" aria-label="Stardust per day, last 30 days">
      {#each data.history as h, i (h.day)}
        {@const bar = (h.points / max) * 76}
        <rect x={i * 10 + 1} y={80 - bar} width="8" height={bar} fill="currentColor" rx="1">
          <title>{h.day}: {h.points}</title>
        </rect>
      {/each}
    </svg>
  </section>
{/if}
