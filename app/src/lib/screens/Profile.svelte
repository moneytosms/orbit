<script lang="ts">
  import { badgeShelf, getGamification, stardust, type Shelf, type Stardust } from '$lib/sync'
  import { getProfile } from '$lib/sync'
  import { deviceId } from '$lib/auth'
  import ProfileAvatar from '$lib/ProfileAvatar.svelte'
  import ProfileEditor from '$lib/ProfileEditor.svelte'

  let { version = 0 }: { version?: number } = $props()
  let data = $state<Stardust>()
  let shelf = $state<Shelf>()
  let gamification = $state(true)
  let profile = $state({ display_name: '', avatar: '' })
  let device = $state('')

  $effect(() => {
    void version
    getGamification().then((on) => (gamification = on))
    getProfile().then((p) => (profile = p))
    deviceId().then((id) => (device = id ?? ''))
    stardust().then((d) => (data = d))
    badgeShelf().then((s) => (shelf = s))
  })

  const R = 44
  const C = 2 * Math.PI * R
  const frac = $derived.by(() => {
    if (!data || data.level.next === null) return 1
    const { floor, next } = data.level
    return Math.min(1, (data.total - floor) / (next - floor))
  })
  const max = $derived(Math.max(1, ...(data?.history.map((h) => h.points) ?? [])))
</script>

<h1 class="page-title">Profile</h1>
<section class="panel mb-4" aria-label="Profile details">
  <ProfileEditor {version} />
</section>
{#if data}
  {#if gamification}
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
        <span class="absolute inset-0 flex items-center justify-center">
          <ProfileAvatar name={profile.display_name} avatar={profile.avatar} {device} size="size-20 text-3xl" />
        </span>
      </div>
      <p class="text-lg font-semibold">{data.level.name} · Level {data.level.n}</p>
      <p class="text-muted">
        {data.total.toLocaleString()} Stardust{data.level.next === null
          ? ''
          : ` · ${(data.level.next - data.total).toLocaleString()} to next Level`}
      </p>
    </div>
  {/if}

  <section class="panel" aria-label="Targets">
    <h2 class="panel-title">Targets</h2>
    <p>Today: {data.targets.today_done} / {data.targets.daily}</p>
    <p>This week: {data.targets.week_done} / {data.targets.weekly}</p>
    <p>Target streak: {data.targets.streak} {data.targets.streak === 1 ? 'day' : 'days'}</p>
    <p class="mt-2 text-muted">
      {data.targets.vacation ? 'Vacation is on. ' : ''}Change targets, days off and vacation in
      <a class="link" href="#/settings">Settings</a>.
    </p>
  </section>

  {#if gamification && shelf}
    <section class="panel" aria-label="Badges">
      <h2 class="panel-title">Badges · {shelf.patches} Patch{shelf.patches === 1 ? '' : 'es'}</h2>
      <ul class="grid grid-cols-2 gap-2 sm:grid-cols-3">
        {#each shelf.badges as b (b.id)}
          <li
            class="rounded border p-2"
            class:opacity-50={b.count === 0}
            class:border-dashed={b.count === 0}
            aria-label={b.count === 0 ? `${b.name}, locked` : b.name}
          >
            <p class="font-medium">{b.name}{b.repeatable && b.count > 0 ? ` x ${b.count}` : ''}</p>
            <p class="text-xs text-muted">{b.tier} · {b.points.toLocaleString()} Stardust</p>
            {#if b.count === 0}
              <p class="text-xs text-muted">{b.condition}</p>
            {/if}
          </li>
        {/each}
      </ul>
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
{/if}
