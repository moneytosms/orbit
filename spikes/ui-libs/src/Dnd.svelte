<script>
  import { dndzone } from 'svelte-dnd-action';
  import { flip } from 'svelte/animate';
  import { draggable, droppable } from '@thisux/sveltednd';
  const mk = (p) => ['Write intro', 'Email advisor', 'Buy milk', 'Fix bike', 'Read ch. 4', 'Plan week'].map((t, i) => ({ id: `${p}${i}`, title: t }));
  // A: svelte-dnd-action, list + board, all zones share a type so cards move anywhere
  let listA = $state(mk('a'));
  let boardA = $state([{ id: 'todo', items: mk('t').slice(0, 3) }, { id: 'doing', items: mk('d').slice(3, 5) }, { id: 'done', items: mk('x').slice(5) }]);
  // B: @thisux/sveltednd, per-item droppables for ordering
  let listB = $state(mk('b'));
  let lib = $state('A');
  let log = $state([]);
  const say = (m) => (log = [m, ...log].slice(0, 30));
  const opts = (items) => ({ items, flipDurationMs: 150, delayTouchStart: 200, dropTargetStyle: { outline: '2px dashed var(--accent)' } });
  function dropB(s) {
    const from = listB.findIndex((x) => x.id === s.draggedItem.id), to = +s.targetContainer;
    const [it] = listB.splice(from, 1); listB.splice(to, 0, it);
    say(`B list: ${listB.map((x) => x.title).join(' | ')}`);
  }
</script>
<p>
  <button class:on={lib === 'A'} onclick={() => (lib = 'A')}>A svelte-dnd-action</button>
  <button class:on={lib === 'B'} onclick={() => (lib = 'B')}>B @thisux/sveltednd</button>
  Touch: press 200 ms then drag. Keyboard (A): Tab to item, Space, arrows, Space.
</p>
{#if lib === 'A'}
  <h4>List</h4>
  <ul class="list" use:dndzone={opts(listA)} onconsider={(e) => (listA = e.detail.items)}
      onfinalize={(e) => { listA = e.detail.items; say(`A list: ${listA.map((x) => x.title).join(' | ')}`); }}>
    {#each listA as it (it.id)}<li animate:flip={{ duration: 150 }}>{it.title}</li>{/each}
  </ul>
  <h4>Board</h4>
  <div class="board">
    {#each boardA as col (col.id)}
      <div class="col"><b>{col.id}</b>
        <ul class="list" use:dndzone={opts(col.items)} onconsider={(e) => (col.items = e.detail.items)}
            onfinalize={(e) => { col.items = e.detail.items; say(`A board: ${boardA.map((c) => c.id + '[' + c.items.map((x) => x.title).join(',') + ']').join(' ')}`); }}>
          {#each col.items as it (it.id)}<li animate:flip={{ duration: 150 }}>{it.title}</li>{/each}
        </ul>
      </div>
    {/each}
  </div>
{:else}
  <h4>List</h4>
  <ul class="list">
    {#each listB as it, i (it.id)}
      <li use:draggable={{ container: String(i), dragData: it }} use:droppable={{ container: String(i), callbacks: { onDrop: dropB } }}>{it.title}</li>
    {/each}
  </ul>
  <p style="color:var(--muted)">B board skipped unless B list is competitive (README board only changes column, no in-column order).</p>
{/if}
<div class="log">{log.join('\n') || 'state log: no drops yet'}</div>
<style>
  .list { list-style:none; margin:0; padding:4px; min-height:40px; display:flex; flex-direction:column; gap:4px; }
  li { background:var(--surface); border:1px solid var(--border); border-radius:6px; padding:10px; user-select:none; touch-action:none; }
  .board { display:flex; gap:8px; overflow-x:auto; }
  .col { background:var(--sunk); border-radius:8px; padding:6px; min-width:160px; flex:1; }
  :global(.svelte-dnd-dragging), :global([data-dragging=true]) { opacity:.5; }
</style>
