<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { parse_quick_add } from '../../wasm/orbit_wasm'
  import CaptureBar from './CaptureBar.svelte'
  import type { Parsed } from './sync'
  import { DEFAULT_PREFS, knownJson, quickAddPrefs, type QuickAddPrefs } from './captureKnown'

  let draft = $state('')
  let input: HTMLInputElement
  let message = $state('')
  let saving = $state(false)
  const localNow = () => {
    const now = new Date()
    const pad = (value: number) => String(value).padStart(2, '0')
    return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}T${pad(now.getHours())}:${pad(now.getMinutes())}`
  }
  type Known = { areas: string[]; goals: string[]; pages: string[]; labels: string[] }
  let known: Known = $state({ areas: [], goals: [], pages: [], labels: [] })
  let prefs: QuickAddPrefs = $state(DEFAULT_PREFS)
  const parsed = $derived(JSON.parse(parse_quick_add(draft, localNow(), knownJson(known, prefs))) as Parsed)

  // The names the option bar offers come from this device's replica; the window has no other store.
  // Date prefs come from the synced Settings where this window can read them, else the defaults.
  async function loadKnown() {
    known = await invoke<Known>('capture_known').catch(() => known)
    prefs = await quickAddPrefs()
  }
  async function edited() {
    await tick()
    input.focus()
    input.setSelectionRange(draft.length, draft.length)
  }

  // The transparent page is for the overlay window only; as a bare global it blanked the whole web app's background.
  onMount(() => document.documentElement.classList.add('capture'))

  async function hide() {
    await invoke('hide_capture').catch(() => {})
  }

  async function save(keepOpen: boolean) {
    if (saving || !draft.trim()) return
    saving = true
    message = ''
    try {
      await invoke('capture_task', { text: draft })
      draft = ''
      if (keepOpen) {
        await tick()
        input.focus()
      } else {
        await hide()
      }
    } catch (error) {
      message = String(error)
    } finally {
      saving = false
    }
  }

  onMount(() => {
    const off = listen('orbit://show-capture', () => {
      draft = ''
      message = ''
      void loadKnown()
      requestAnimationFrame(() => input.focus())
    })
    void loadKnown()
    requestAnimationFrame(() => input.focus())
    return () => void off.then((unlisten) => unlisten())
  })

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault()
      void hide()
    } else if (event.key === 'Enter') {
      event.preventDefault()
      void save(event.ctrlKey || event.metaKey)
    }
  }
</script>

<svelte:window
  onblur={() => {
    setTimeout(() => {
      if (!document.hasFocus()) void hide()
    }, 120)
  }}
/>

<main class="capture-shell">
  <div class="capture-input">
    <span class="capture-mark" aria-hidden="true">＋</span>
    <input bind:this={input} bind:value={draft} onkeydown={keydown} placeholder="Add a task" aria-label="Task title" />
    <button onclick={() => void save(false)} disabled={!draft.trim() || saving}>Add</button>
  </div>
  <div class="capture-bar">
    <CaptureBar bind:draft {parsed} {known} onedit={edited} />
  </div>
  {#if parsed.tokens.some((t) => !t.known)}
    <ul class="capture-chips" aria-label="Unresolved quick-add tokens">
      {#each parsed.tokens.filter((t) => !t.known) as token (token.start)}
        <li class="unknown">{token.text} kept in title</li>
      {/each}
    </ul>
  {/if}
  {#if message}<p role="alert" class="capture-message">{message}</p>{/if}
  <p class="capture-hint">Enter to add · Ctrl+Enter to add another · Esc to close</p>
</main>

<style>
  :global(html.capture),
  :global(html.capture body),
  :global(html.capture #app) {
    margin: 0;
    min-height: 100%;
    background: transparent;
  }
  .capture-shell {
    box-sizing: border-box;
    margin: 16px;
    padding: 12px 16px 8px;
    border: 1px solid var(--line, #d7dce2);
    border-radius: 16px;
    background: var(--surface, #fff);
    color: var(--text, #222);
    box-shadow: 0 12px 40px #0003;
  }
  .capture-input {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .capture-mark {
    color: var(--accent, #4569c8);
    font-size: 22px;
  }
  input {
    flex: 1;
    min-width: 0;
    height: 38px;
    border: 0;
    outline: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 16px;
  }
  button {
    border: 0;
    border-radius: 8px;
    padding: 8px 14px;
    background: var(--accent, #4569c8);
    color: white;
    font: inherit;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .capture-bar {
    margin: 8px 0 0 32px;
  }
  .capture-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 8px 0 0 32px;
    padding: 0;
    list-style: none;
  }
  .capture-chips li {
    border: 1px solid var(--line, #d7dce2);
    border-radius: 999px;
    padding: 2px 9px;
    color: var(--muted, #68717d);
    font-size: 12px;
  }
  .capture-chips li.unknown {
    color: var(--warn, #a36517);
  }
  .capture-hint {
    margin: 8px 0 0 32px;
    color: var(--muted, #68717d);
    font-size: 12px;
  }
  .capture-message {
    margin: 6px 0 0 32px;
    color: var(--warn, #a12a2a);
    font-size: 12px;
  }
</style>
