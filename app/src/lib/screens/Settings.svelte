<script lang="ts">
  import Laptop from '@lucide/svelte/icons/laptop'
  import Smartphone from '@lucide/svelte/icons/smartphone'
  import Globe from '@lucide/svelte/icons/globe'
  import Terminal from '@lucide/svelte/icons/terminal'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import * as auth from '$lib/auth'
  import { getTheme, setTheme, type Theme } from '$lib/theme'

  let theme = $state(getTheme())
  const themes: [Theme, string][] = [
    ['light', 'Orbit Light'],
    ['dark', 'Orbit Dark'],
    ['system', 'System'],
  ]

  let list: auth.Device[] = $state([])
  let listError = $state('')
  const load = () =>
    auth.devices().then(
      (d) => ((list = d), (listError = '')),
      () => (listError = "Can't load Devices while offline."),
    )
  load()
  const icon = (p: string) => ({ web: Globe, android: Smartphone, cli: Terminal, script: Terminal })[p] ?? Laptop
  const when = (ms: number) => new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })

  let tokenName = $state('')
  let newToken = $state('')
  async function createToken(e: SubmitEvent) {
    e.preventDefault()
    newToken = (await auth.createToken(tokenName)).token
    tokenName = ''
    load()
  }
  const revoke = (id: string) => auth.revoke(id).then(load)

  let current = $state('')
  let next = $state('')
  let others = $state(false)
  let pwMsg = $state('')
  async function changePassword(e: SubmitEvent) {
    e.preventDefault()
    try {
      await auth.changePassword(current, next, others)
      pwMsg = others ? 'Password changed. Other Devices are signed out.' : 'Password changed.'
      current = next = ''
      load()
    } catch (err) {
      const c = err instanceof Error ? err.message : ''
      pwMsg = { credentials: 'Current password is wrong.', weak_password: 'Use at least 8 characters.' }[c] ?? 'Could not change the password.'
    }
  }

  let enroll: { secret: string; url: string } | undefined = $state()
  let totpCode = $state('')
  let totpPassword = $state('')
  async function startTotp(e: SubmitEvent) {
    e.preventDefault()
    try {
      enroll = await auth.startTotp(totpPassword)
      totpMsg = ''
    } catch {
      totpMsg = 'Current password is wrong.'
    }
    totpPassword = ''
  }
  let totpMsg = $state('')
  async function confirmTotp(e: SubmitEvent) {
    e.preventDefault()
    try {
      await auth.confirmTotp(totpCode)
      enroll = undefined
      totpMsg = 'Two-step login is on. Logins now ask for a code.'
    } catch {
      totpMsg = 'That code is wrong or expired.'
    }
    totpCode = ''
  }
</script>

<h1 class="page-title">Settings</h1>
<div class="flex flex-col gap-4">
  <section class="panel">
    <h2 class="panel-title">Appearance</h2>
    <div class="inline-flex gap-0.5 rounded-[9px] bg-sunk p-[3px]" role="group" aria-label="Theme">
      {#each themes as [value, label] (value)}
        <button
          type="button"
          aria-pressed={theme === value}
          onclick={() => setTheme((theme = value))}
          class="h-7 rounded-md px-3 font-medium text-muted aria-pressed:bg-surface aria-pressed:text-text aria-pressed:shadow-1"
        >
          {label}
        </button>
      {/each}
    </div>
  </section>

  <section class="panel">
    <h2 class="panel-title">Devices <span class="font-normal text-muted">{list.length || ''}</span></h2>
    {#if listError}<p class="text-muted">{listError}</p>{/if}
    <ul>
      {#each list as d (d.id)}
        {@const Icon = icon(d.platform)}
        <li class="flex min-h-[42px] items-center gap-3 border-b border-line py-2 last:border-0">
          <Icon size={16} class="shrink-0 text-icon" />
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="truncate font-medium">{d.name}</span>
              {#if d.current}<span class="chip bg-accent-soft text-accent-text">This device</span>{/if}
            </div>
            <div class="text-[12.5px] text-muted">
              {d.platform} · added {when(d.created)} · last seen {when(d.last_seen)}
            </div>
          </div>
          {#if !d.current}
            <Button size="sm" variant="destructive" onclick={() => revoke(d.id)}>Revoke</Button>
          {/if}
        </li>
      {/each}
    </ul>
    <form onsubmit={createToken} class="mt-3 flex gap-2">
      <Input bind:value={tokenName} placeholder="Token name, e.g. Backup script" aria-label="Token name" required />
      <Button type="submit">Create token</Button>
    </form>
    {#if newToken}
      <div class="mt-3 rounded-lg bg-warn-soft p-3 text-warn-text">
        <p>Copy this token now; it won't be shown again. Send it as <code>Authorization: Bearer …</code></p>
        <code class="mt-2 block rounded-xs bg-surface p-2 break-all text-text select-all">{newToken}</code>
      </div>
    {/if}
  </section>

  <section class="panel">
    <h2 class="panel-title">Password</h2>
    <form onsubmit={changePassword} class="flex max-w-sm flex-col gap-3">
      <div class="flex flex-col gap-1.5">
        <Label for="current">Current password</Label>
        <Input id="current" type="password" bind:value={current} autocomplete="current-password" required />
      </div>
      <div class="flex flex-col gap-1.5">
        <Label for="new">New password</Label>
        <Input id="new" type="password" bind:value={next} autocomplete="new-password" required />
      </div>
      <Label class="font-normal"><Checkbox bind:checked={others} /> Sign out other devices</Label>
      {#if pwMsg}<p role="status" class="text-muted">{pwMsg}</p>{/if}
      <Button type="submit" class="self-start">Change password</Button>
    </form>
  </section>

  <section class="panel">
    <h2 class="panel-title">Two-step login</h2>
    {#if enroll}
      <form onsubmit={confirmTotp} class="flex max-w-sm flex-col gap-3">
        <p class="text-muted">
          Add this key to your authenticator app, or <a class="link" href={enroll.url}>open it there</a>, then enter
          the code it shows.
        </p>
        <code class="rounded-xs bg-sunk p-2 tracking-wider break-all select-all">{enroll.secret}</code>
        <Input bind:value={totpCode} inputmode="numeric" autocomplete="one-time-code" aria-label="Code" required />
        <Button type="submit" variant="primary" class="self-start">Turn on</Button>
      </form>
    {:else}
      <form onsubmit={startTotp} class="flex max-w-sm flex-col gap-3">
        <p class="text-muted">Ask for a code from an authenticator app after the password.</p>
        <Input type="password" bind:value={totpPassword} placeholder="Current password" aria-label="Current password" autocomplete="current-password" required />
        <Button type="submit" class="self-start">Set up authenticator</Button>
      </form>
    {/if}
    {#if totpMsg}<p role="status" class="mt-2 text-muted">{totpMsg}</p>{/if}
  </section>
</div>
