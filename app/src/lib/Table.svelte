<script lang="ts">
  import ArrowDown from '@lucide/svelte/icons/arrow-down'
  import ArrowUp from '@lucide/svelte/icons/arrow-up'
  import {
    cellText,
    defaultFields,
    fieldTitle,
    rowTitle,
    sortPath,
    type BoardRow,
    type CellCtx,
  } from '$lib/Board.svelte'
  import { labels as listLabels, nameIndex, type Sort } from '$lib/sync'

  let {
    rows,
    fields,
    sorts,
    entity = 'task',
    onsort,
    onopen,
  }: {
    rows: BoardRow[]
    fields: string[]
    sorts: Sort[]
    entity?: string
    onsort: (sorts: Sort[]) => void
    onopen: (row: BoardRow) => void
  } = $props()

  let cx = $state<CellCtx>({ labels: new Map(), names: new Map() })
  $effect(() => {
    void rows
    Promise.all([listLabels(), nameIndex()]).then(
      ([l, names]) => (cx = { labels: new Map(l.map((x) => [x.id, x])), names }),
    )
  })

  const columns = $derived(['title', ...(fields.length ? fields : defaultFields(entity))])
  const active = (f: string) => sorts[0]?.field === sortPath(f)
  const dir = (f: string) => (active(f) ? (sorts[0].desc ? 'descending' : 'ascending') : 'none')
  function sortBy(f: string) {
    const p = sortPath(f)
    const rest = sorts.filter((s) => s.field !== p).map((s) => ({ ...s }))
    onsort([{ field: p, desc: active(f) ? !sorts[0].desc : false }, ...rest])
  }
</script>

<div class="overflow-x-auto">
  <table class="w-full border-collapse text-left text-sm">
    <thead>
      <tr class="border-b border-line">
        {#each columns as f (f)}
          <th scope="col" aria-sort={dir(f)} class="px-2 py-1.5 text-[12.5px] font-semibold text-muted">
            <button type="button" class="flex items-center gap-1 capitalize" onclick={() => sortBy(f)}>
              {fieldTitle(f)}
              {#if active(f)}
                {#if sorts[0].desc}<ArrowDown size={12} />{:else}<ArrowUp size={12} />{/if}
              {/if}
            </button>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as r (r.id)}
        <tr class="cursor-pointer border-b border-line hover:bg-hover" onclick={() => onopen(r)}>
          {#each columns as f (f)}
            <td class="max-w-[320px] truncate px-2 py-2 {f === 'title' ? '' : 'text-muted'}">
              {#if f === 'title'}
                <button
                  type="button"
                  class="text-left"
                  onclick={(e) => {
                    e.stopPropagation()
                    onopen(r)
                  }}>{rowTitle(r)}</button
                >
              {:else}
                {cellText(r, f, cx)}
              {/if}
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
