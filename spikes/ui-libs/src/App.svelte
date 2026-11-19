<script>
  import Calendar from './Calendar.svelte';
  import Dnd from './Dnd.svelte';
  import Editor from './Editor.svelte';
  const tabs = { calendar: Calendar, dnd: Dnd, editor: Editor };
  let tab = $state(new URLSearchParams(location.search).get('tab') ?? 'calendar');
  let dark = $state(false);
  $effect(() => document.documentElement.setAttribute('data-color-scheme', dark ? 'dark' : 'light'));
  const Tab = $derived(tabs[tab]);
</script>
<header style="display:flex;gap:6px;padding:8px;border-bottom:1px solid var(--border);flex-wrap:wrap">
  <strong style="color:var(--accent-text)">PROTOTYPE</strong>
  {#each Object.keys(tabs) as t}<button class:on={tab === t} onclick={() => (tab = t)}>{t}</button>{/each}
  <button onclick={() => (dark = !dark)}>{dark ? 'light' : 'dark'}</button>
</header>
<main style="padding:8px"><Tab /></main>
