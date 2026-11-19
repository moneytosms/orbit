<script>
  import { onMount } from 'svelte';
  import { createEditor, EditorContent } from 'svelte-tiptap';
  import StarterKit from '@tiptap/starter-kit';
  import Mention from '@tiptap/extension-mention';
  import { PluginKey } from '@tiptap/pm/state';
  const pages = ['Thesis outline', 'Reading list', 'Bike repair notes', 'Week plan 47'];
  const items = ['Task: Email advisor', 'Goal: Run 5k', 'Project: Thesis', 'Area: College'];
  let menu = $state(null); // { char, items, sel, command, rect }
  let editor = $state();
  let json = $state('');
  const pick = (m, i) => m.command({ id: m.items[i], label: m.items[i] });
  function render(char) {
    return () => ({
      onStart: (p) => (menu = { char, items: p.items, sel: 0, command: p.command, rect: p.clientRect?.() }),
      onUpdate: (p) => (menu = { ...menu, items: p.items, sel: 0, command: p.command, rect: p.clientRect?.() }),
      onKeyDown: ({ event }) => {
        if (!menu) return false;
        if (event.key === 'ArrowDown') { menu.sel = (menu.sel + 1) % menu.items.length; return true; }
        if (event.key === 'ArrowUp') { menu.sel = (menu.sel + menu.items.length - 1) % menu.items.length; return true; }
        if (event.key === 'Enter' && menu.items.length) { pick(menu, menu.sel); return true; }
        if (event.key === 'Escape') { menu = null; return true; }
        return false;
      },
      onExit: () => (menu = null),
    });
  }
  const filter = (list) => ({ query }) => list.filter((x) => x.toLowerCase().includes(query.toLowerCase())).slice(0, 6);
  onMount(() => {
    editor = createEditor({
      extensions: [StarterKit, Mention.configure({
        HTMLAttributes: { class: 'mention' },
        renderText: ({ node }) => node.attrs.mentionSuggestionChar === '[[' ? `[[${node.attrs.label}]]` : `@${node.attrs.label}`,
        renderHTML: ({ options, node }) => ['span', { ...options.HTMLAttributes, 'data-kind': node.attrs.mentionSuggestionChar }, node.attrs.mentionSuggestionChar === '[[' ? `[[${node.attrs.label}]]` : `@${node.attrs.label}`],
        suggestions: [
          { char: '@', items: filter(items), render: render('@') },
          { char: '[[', pluginKey: new PluginKey('pageLink'), allowSpaces: true, items: filter(pages), render: render('[[') },
        ],
      })],
      content: '<p>Type <code>@</code> for items or <code>[[</code> for Pages. </p>',
      onUpdate: ({ editor }) => (json = JSON.stringify(editor.getJSON().content?.flatMap((b) => b.content ?? []).filter((n) => n.type === 'mention').map((n) => n.attrs))),
    });
  });
</script>
<div class="ed"><EditorContent editor={$editor} /></div>
{#if menu}
  <div class="menu" style="top:{(menu.rect?.bottom ?? 0) + scrollY + 4}px;left:{menu.rect?.left ?? 0}px">
    {#each menu.items as it, i}<button class:on={i === menu.sel} onmousedown={(e) => { e.preventDefault(); pick(menu, i); }}>{menu.char === '[[' ? '📄 ' : ''}{it}</button>{:else}<i>no match</i>{/each}
  </div>
{/if}
<div class="log">{json ? 'mention nodes: ' + json : 'state log: no mentions yet'}</div>
<style>
  .ed :global(.tiptap) { min-height:200px; background:var(--surface); border:1px solid var(--border); border-radius:8px; padding:12px; outline:none; }
  .ed :global(.mention) { color:var(--accent-text); background:var(--accent-soft); border-radius:4px; padding:0 3px; }
  .menu { position:absolute; display:flex; flex-direction:column; background:var(--surface); border:1px solid var(--border-strong); border-radius:8px; padding:4px; z-index:10; min-width:200px; }
  .menu button { border:0; text-align:left; }
</style>
