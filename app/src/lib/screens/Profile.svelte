<script lang="ts">
  import { badgeShelf, getGamification, stardust, type Shelf, type Stardust } from '$lib/sync'
  import { getProfile } from '$lib/sync'
  import { deviceId } from '$lib/auth'
  import ProfileAvatar from '$lib/ProfileAvatar.svelte'
  import BadgeIcon from '$lib/BadgeIcon.svelte'

  let { version = 0 }: { version?: number } = $props()
  const toSettings = () => sessionStorage.setItem('orbit.jump', 'profile')
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
  const unset = $derived(!profile.display_name && !profile.avatar)
  const pct = (done: number, goal: number) => Math.min(100, goal ? Math.round((done / goal) * 100) : 0)
  const shortDay = (d: string) =>
    new Date(d + 'T00:00').toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
</script>

<h1 class="page-title">Profile</h1>
<section class="panel mb-4 flex flex-col items-center gap-3 px-4 py-6 text-center" aria-label="Profile">
  <div class="relative size-28">
    {#if gamification && data}
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
    {/if}
    <span class="absolute inset-0 flex items-center justify-center">
      <ProfileAvatar
        name={profile.display_name}
        avatar={profile.avatar}
        {device}
        size={gamification && data ? 'size-20 text-3xl' : 'size-28 text-4xl'}
      />
    </span>
  </div>
  <div>
    <p class="text-xl font-semibold">{profile.display_name || 'Unnamed'}</p>
    {#if gamification && data}
      <p class="text-muted">
        {data.level.name} · Level {data.level.n} · {data.total.toLocaleString()} Stardust{data.level.next === null
          ? ''
          : `, ${(data.level.next - data.total).toLocaleString()} to next Level`}
      </p>
    {/if}
  </div>
  <a class="link text-sm" href="#/settings" onclick={toSettings}
    >{unset ? 'Add a profile picture and name in Settings' : 'Edit name and picture in Settings'}</a
  >
</section>

{#if data}
  <section class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-3" aria-label="Targets">
    {#each [{ label: 'Today', done: data.targets.today_done, goal: data.targets.daily }, { label: 'This week', done: data.targets.week_done, goal: data.targets.weekly }] as t (t.label)}
      <div class="panel !p-3">
        <p class="text-xs text-muted">{t.label}</p>
        <p class="text-lg font-semibold">{t.done} <span class="text-sm font-normal text-muted">/ {t.goal}</span></p>
        <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-hover">
          <div class="h-full rounded-full bg-accent" style:width="{pct(t.done, t.goal)}%"></div>
        </div>
      </div>
    {/each}
    <div class="panel !p-3">
      <p class="text-xs text-muted">Target streak</p>
      <p class="text-lg font-semibold">
        {data.targets.streak}
        <span class="text-sm font-normal text-muted">{data.targets.streak === 1 ? 'day' : 'days'}</span>
      </p>
      <p class="mt-2 text-xs text-muted">
        {data.targets.vacation ? 'Vacation is on. ' : ''}Targets, days off and vacation in
        <a class="link" href="#/settings" onclick={() => sessionStorage.setItem('orbit.jump', 'targets')}>Settings</a>.
      </p>
    </div>
  </section>

  {#if gamification && shelf}
    <section class="panel mb-4" aria-label="Badges">
      <h2 class="panel-title">Badges · {shelf.patches} Patch{shelf.patches === 1 ? '' : 'es'}</h2>
      <ul class="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {#each shelf.badges as b (b.id)}
          <li
            class="flex items-center gap-3 rounded-xl border p-2.5"
            class:border-dashed={b.count === 0}
            aria-label={b.count === 0 ? `${b.name}, locked` : b.name}
          >
            <span class:opacity-60={b.count === 0} class:saturate-50={b.count === 0}>
              <BadgeIcon id={b.id} tier={b.tier} size="size-12" />
            </span>
            <div class="min-w-0" class:opacity-60={b.count === 0}>
              <p class="truncate font-medium">{b.name}{b.repeatable && b.count > 0 ? ` x ${b.count}` : ''}</p>
              <p class="text-xs text-muted">{b.tier} · {b.points.toLocaleString()} Stardust</p>
              {#if b.count === 0}<p class="text-xs text-muted">
                  {b.condition} · {b.progress.toLocaleString()} / {b.need.toLocaleString()}
                </p>{/if}
            </div>
          </li>
        {/each}
      </ul>
    </section>

    <section class="panel" aria-label="Stardust history">
      <h2 class="panel-title">Last 30 days</h2>
      <div
        class="flex h-28 items-end gap-[3px] border-b border-line"
        role="img"
        aria-label="Stardust per day, last 30 days"
      >
        {#each data.history as h (h.day)}
          <div class="flex h-full min-w-0 flex-1 items-end" title="{h.day}: {h.points}">
            <div
              class="w-full rounded-t-sm {h.points ? 'bg-accent' : 'bg-hover'}"
              style:height={h.points ? `${Math.max(6, (h.points / max) * 100)}%` : '2px'}
            ></div>
          </div>
        {/each}
      </div>
      {#if data.history.length}
        <div class="mt-1 flex justify-between text-xs text-muted">
          <span>{shortDay(data.history[0].day)}</span>
          <span>{shortDay(data.history[data.history.length - 1].day)}</span>
        </div>
      {/if}
    </section>
  {/if}
{/if}
