<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { login, setup } from '$lib/auth'

  let {
    mode,
    pending = 0,
    done,
  }: { mode: 'setup' | 'login'; pending?: number; done: () => void } = $props()

  let code = $state('')
  let password = $state('')
  let confirm = $state('')
  let totp = $state('')
  let askTotp = $state(false)
  let error = $state('')
  let busy = $state(false)

  const messages: Record<string, string> = {
    credentials: 'Wrong password.',
    rate_limited: 'Too many attempts. Wait a minute and try again.',
    setup_code: "That setup code doesn't match the one in the server log.",
    weak_password: 'Use at least 8 characters.',
    setup_needed: 'This server has no user yet. Reload to run first-time setup.',
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault()
    error = ''
    if (mode === 'setup' && password !== confirm) return (error = "The passwords don't match.")
    busy = true
    try {
      await (mode === 'setup' ? setup(code, password) : login(password, totp))
      done()
    } catch (err) {
      const c = err instanceof Error ? err.message : ''
      if (c === 'totp') {
        error = askTotp ? 'That code is wrong or expired.' : ''
        askTotp = true
      } else error = messages[c] ?? "Can't reach the server. Check your connection."
    } finally {
      busy = false
    }
  }
</script>

<main class="grid min-h-full place-items-center p-4">
  <form
    onsubmit={submit}
    class="flex w-full max-w-[360px] flex-col gap-4 rounded-2xl border border-line bg-surface p-6 shadow-1"
  >
    <div class="flex items-center gap-2">
      <img src="{import.meta.env.BASE_URL}icon.svg" alt="" class="size-7" />
      <span class="text-base font-semibold">Orbit</span>
    </div>
    {#if pending}
      <p class="rounded-lg bg-warn-soft px-3 py-2 text-warn-text" role="status">
        Signed out: log in to send {pending} pending {pending === 1 ? 'change' : 'changes'}
      </p>
    {/if}
    <div>
      <h1 class="text-xl font-semibold">{mode === 'setup' ? 'Set up Orbit' : 'Log in'}</h1>
      {#if mode === 'setup'}
        <p class="mt-1 text-muted">Enter the setup code from the server log and choose your password.</p>
      {/if}
    </div>
    {#if mode === 'setup'}
      <div class="flex flex-col gap-1.5">
        <Label for="code">Setup code</Label>
        <Input id="code" bind:value={code} autocomplete="off" required />
      </div>
    {/if}
    <div class="flex flex-col gap-1.5">
      <Label for="password">Password</Label>
      <Input
        id="password"
        type="password"
        bind:value={password}
        autocomplete={mode === 'setup' ? 'new-password' : 'current-password'}
        required
      />
    </div>
    {#if mode === 'setup'}
      <div class="flex flex-col gap-1.5">
        <Label for="confirm">Confirm password</Label>
        <Input id="confirm" type="password" bind:value={confirm} autocomplete="new-password" required />
      </div>
    {/if}
    {#if askTotp}
      <div class="flex flex-col gap-1.5">
        <Label for="totp">Authenticator code</Label>
        <Input id="totp" bind:value={totp} inputmode="numeric" autocomplete="one-time-code" required />
      </div>
    {/if}
    {#if error}<p class="text-danger-text" role="alert">{error}</p>{/if}
    <Button type="submit" variant="primary" disabled={busy}>
      {mode === 'setup' ? 'Create my account' : 'Log in'}
    </Button>
  </form>
</main>
