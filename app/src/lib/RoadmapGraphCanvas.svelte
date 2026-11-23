<script lang="ts">
  import Circle from '@lucide/svelte/icons/circle'
  import Diamond from '@lucide/svelte/icons/diamond'
  import FileText from '@lucide/svelte/icons/file-text'
  import Heading from '@lucide/svelte/icons/heading'
  import Link from '@lucide/svelte/icons/link'
  import Maximize from '@lucide/svelte/icons/maximize'
  import Minus from '@lucide/svelte/icons/minus'
  import Plus from '@lucide/svelte/icons/plus'
  import Sparkles from '@lucide/svelte/icons/sparkles'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import X from '@lucide/svelte/icons/x'
  import { Button } from '$lib/components/ui/button'
  import { hiddenNodeIds } from '$lib/roadmapGraph'
  import {
    createGraphEdge,
    createGraphNode,
    createPhase,
    createTask,
    decideGraphNode,
    deleteGraphEdge,
    deleteItem,
    moveGraphNode,
    roadmapGraph,
    updateGraphEdge,
    updateGraphNode,
    type GraphEdge,
    type GraphEdgeKind,
    type GraphNode,
    type GraphNodeKind,
  } from '$lib/sync'

  let { id, onchange }: { id: string; onchange: () => void } = $props()
  let nodes = $state<GraphNode[]>([])
  let edges = $state<GraphEdge[]>([])
  let bands = $state<import('$lib/sync').Phase[]>([])
  let choices = $state<{ id: string; kind: GraphNodeKind; title: string }[]>([])
  let el = $state<HTMLDivElement>()
  let view = $state({ x: 32, y: 32, z: 1 })
  let selected = $state<string[]>([])
  let selectedEdge = $state<string | null>(null)
  let connectFrom = $state<string | null>(null)
  let linkDrag = $state<{ from: string; x: number; y: number } | null>(null)
  let endpointDrag = $state<{ edgeId: string; endpoint: 'source' | 'target'; x: number; y: number } | null>(null)
  let drag = $state<{ id: string; startX: number; startY: number; x: number; y: number; moved: boolean } | null>(null)
  let pan = $state<{ x: number; y: number } | null>(null)
  let marquee = $state<{ x: number; y: number; w: number; h: number } | null>(null)
  let undo = $state<{ id: string; x: number; y: number }[]>([])
  let snap = $state(true)
  let showMinimap = $state(false)
  let edgeKind = $state<GraphEdgeKind>('then')
  let edgeLabel = $state('')
  let draftTitle = $state('')
  let spawnCount = $state(0)
  let contextMenu = $state<{ x: number; y: number; nodeId?: string; edgeId?: string } | null>(null)

  const nodeById = $derived(new Map(nodes.map((n) => [n.id, n])))
  const taskChoices = $derived(choices.filter((c) => c.kind === 'task' && !nodes.some((n) => n.ref_id === c.id)))
  const hiddenNodes = $derived(hiddenNodeIds(nodes, edges))
  const renderedNodes = $derived(nodes.filter((n) => !hiddenNodes.has(n.id)))
  const visibleEdges = $derived(edges.filter((e) => {
    const source = nodeById.get(e.source_id)
    return source && !hiddenNodes.has(e.source_id) && !hiddenNodes.has(e.target_id) && (!source.decided_at || source.taken_edge_id === e.id)
  }))
  const focusedNode = $derived(nodes.find((n) => n.id === selected[0]))
  const focusedEdge = $derived(edges.find((e) => e.id === selectedEdge))
  const bandBoxes = $derived(bands.flatMap((band) => {
    const members = renderedNodes.filter((node) => node.band_id === band.id)
    if (!members.length) return []
    const x = Math.min(...members.map((n) => n.x)) - 24
    const y = Math.min(...members.map((n) => n.y)) - 36
    const right = Math.max(...members.map((n) => n.x + 240)) + 24
    const bottom = Math.max(...members.map((n) => n.y + 64)) + 24
    return [{ ...band, x, y, w: right - x, h: bottom - y }]
  }))
  const minimapBounds = $derived.by(() => {
    if (!renderedNodes.length) return { x: 0, y: 0, width: 300, height: 180 }
    const left = Math.min(...renderedNodes.map((n) => n.x)) - 80
    const top = Math.min(...renderedNodes.map((n) => n.y)) - 64
    const right = Math.max(...renderedNodes.map((n) => n.x + 240)) + 80
    const bottom = Math.max(...renderedNodes.map((n) => n.y + 64)) + 64
    return { x: left, y: top, width: right - left, height: bottom - top }
  })
  const alignmentGuides = $derived.by(() => {
    if (!drag?.moved) return { x: undefined as number | undefined, y: undefined as number | undefined }
    const moving = nodeById.get(drag.id)
    if (!moving) return { x: undefined, y: undefined }
    const x = moving.x + drag.x - drag.startX
    const y = moving.y + drag.y - drag.startY
    let guideX: number | undefined
    let guideY: number | undefined
    for (const other of renderedNodes) {
      if (other.id === moving.id) continue
      if (guideX === undefined && Math.abs(x + 120 - (other.x + 120)) < 7) guideX = other.x + 120
      if (guideY === undefined && Math.abs(y - other.y) < 7) guideY = other.y
    }
    return { x: guideX, y: guideY }
  })
  const scalePoint = (x: number, y: number) => ({ x: Math.round(x / 24) * 24, y: Math.round(y / 24) * 24 })
  const world = (e: { clientX: number; clientY: number }) => {
    const r = el!.getBoundingClientRect()
    return { x: (e.clientX - r.left - view.x) / view.z, y: (e.clientY - r.top - view.y) / view.z }
  }

  async function load() {
    const graph = await roadmapGraph(id)
    nodes = graph.nodes
    edges = graph.edges
    bands = graph.bands
    choices = graph.choices
  }
  $effect(() => { void id; load() })

  function edgePoints(a: GraphNode, b: GraphNode) {
    const forward = b.x >= a.x
    const x1 = a.x + (forward ? 240 : 0)
    const y1 = a.y + (a.node_kind === 'decision' ? 42 : 28)
    const x2 = b.x + (forward ? 0 : 240)
    const y2 = b.y + 28
    return { x1, y1, x2, y2 }
  }
  function path(a: GraphNode, b: GraphNode) {
    const { x1, y1, x2, y2 } = edgePoints(a, b)
    const k = Math.max(36, Math.abs(x2 - x1) * 0.45)
    const dir = x2 >= x1 ? 1 : -1
    return `M ${x1} ${y1} C ${x1 + k * dir} ${y1}, ${x2 - k * dir} ${y2}, ${x2} ${y2}`
  }
  const edgeColor = (edge: GraphEdge) => selectedEdge === edge.id ? 'var(--accent)' : edge.edge_kind === 'blocks' ? 'var(--warn)' : edge.edge_kind === 'depends_on' ? 'var(--accent)' : edge.edge_kind === 'decides' ? 'var(--warn)' : 'var(--icon)'
  function fit() {
    if (!el || !renderedNodes.length) return (view = { x: 32, y: 32, z: 1 })
    const minX = Math.min(...renderedNodes.map((n) => n.x))
    const minY = Math.min(...renderedNodes.map((n) => n.y))
    const maxX = Math.max(...renderedNodes.map((n) => n.x + 240))
    const maxY = Math.max(...renderedNodes.map((n) => n.y + 76))
    const z = Math.max(0.4, Math.min(1.3, (el.clientWidth - 48) / (maxX - minX), (el.clientHeight - 48) / (maxY - minY)))
    view = { z, x: 24 - minX * z, y: 24 - minY * z }
  }
  function down(e: PointerEvent) {
    if (e.button !== 0 || (e.target as HTMLElement).closest('[data-ui],input,select,textarea')) return
    contextMenu = null
    const endpoint = (e.target as SVGElement).closest<SVGElement>('[data-edge-end]')
    if (endpoint) {
      const [side, edgeId] = endpoint.dataset.edgeEnd!.split(':')
      endpointDrag = { edgeId, endpoint: side as 'source' | 'target', ...world(e) }
      selectedEdge = edgeId
      selected = []
      ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
      return
    }
    const port = (e.target as HTMLElement).closest<HTMLElement>('[data-connect]')
    if (port) {
      linkDrag = { from: port.dataset.connect!, ...world(e) }
      ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
      return
    }
    const target = (e.target as HTMLElement).closest<HTMLElement>('[data-node]')
    if (target) {
      const id = target.dataset.node!
      if (connectFrom) {
        if (id !== connectFrom) void connect(connectFrom, id)
        connectFrom = null
        return
      }
      if (!e.shiftKey && !selected.includes(id)) selected = [id]
      else if (e.shiftKey) selected = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id]
      selectedEdge = null
      const p = world(e)
      drag = { id, startX: p.x, startY: p.y, x: p.x, y: p.y, moved: false }
      ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
      return
    }
    if (e.shiftKey) {
      const p = world(e)
      marquee = { x: p.x, y: p.y, w: 0, h: 0 }
      selected = []
    } else {
      selected = []
      selectedEdge = null
      pan = { x: e.clientX, y: e.clientY }
    }
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  function move(e: PointerEvent) {
    if (endpointDrag) {
      endpointDrag = { ...endpointDrag, ...world(e) }
    } else if (linkDrag) {
      linkDrag = { ...linkDrag, ...world(e) }
    } else if (pan) {
      view = { ...view, x: view.x + e.clientX - pan.x, y: view.y + e.clientY - pan.y }
      pan = { x: e.clientX, y: e.clientY }
    } else if (drag) {
      const p = world(e)
      drag = { ...drag, x: p.x, y: p.y, moved: drag.moved || Math.hypot(p.x - drag.startX, p.y - drag.startY) > 3 }
    } else if (marquee) {
      const p = world(e)
      const x = Math.min(marquee.x, p.x), y = Math.min(marquee.y, p.y)
      const w = Math.abs(p.x - marquee.x), h = Math.abs(p.y - marquee.y)
      marquee = { x, y, w, h }
      selected = nodes.filter((n) => n.x + 240 >= x && n.x <= x + w && n.y + 64 >= y && n.y <= y + h).map((n) => n.id)
    }
  }
  async function up(e: PointerEvent) {
    pan = null
    if (endpointDrag) {
      const current = endpointDrag
      endpointDrag = null
      const targetId = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-node]')?.dataset.node
      const edge = edges.find((item) => item.id === current.edgeId)
      const nodeId = current.endpoint === 'source' ? edge?.source_id : edge?.target_id
      if (edge && targetId && targetId !== nodeId) await rerouteEdge(edge, current.endpoint, targetId)
      return
    }
    if (linkDrag) {
      const from = linkDrag.from
      const target = document.elementFromPoint(linkDrag.x * view.z + view.x + el!.getBoundingClientRect().left, linkDrag.y * view.z + view.y + el!.getBoundingClientRect().top)?.closest<HTMLElement>('[data-node]')?.dataset.node
      linkDrag = null
      if (target && target !== from) await connect(from, target)
      marquee = null
      return
    }
    if (drag) {
      const d = drag
      drag = null
      if (d.moved) {
        const dx0 = d.x - d.startX, dy0 = d.y - d.startY
        const dx = snap ? Math.round(dx0 / 24) * 24 : dx0
        const dy = snap ? Math.round(dy0 / 24) * 24 : dy0
        const moving = selected.length ? selected : [d.id]
        undo = moving.map((id) => { const n = nodeById.get(id)!; return { id, x: n.x, y: n.y } })
        for (const id of moving) {
          const n = nodeById.get(id)
          if (n) await moveGraphNode(id, n.x + dx, n.y + dy)
        }
        await load()
        onchange()
      }
    }
    marquee = null
  }
  async function connect(from: string, to: string) {
    const id = await createGraphEdge(idValue(), from, to, edgeKind, edgeLabel)
    if (nodeById.get(from)?.node_kind === 'decision') await decideGraphNode(from, null)
    edgeLabel = ''
    await load()
    selectedEdge = id
    onchange()
  }
  async function rerouteEdge(edge: GraphEdge, endpoint: 'source' | 'target', nodeId: string) {
    if (endpoint === 'source' && nodeId !== edge.source_id && nodeById.get(edge.source_id)?.taken_edge_id === edge.id) {
      await decideGraphNode(edge.source_id, null)
    }
    await updateGraphEdge(edge.id, { [`${endpoint}_id`]: nodeId })
    await load()
    selectedEdge = edge.id
    onchange()
  }
  const idValue = () => id
  async function add(kind: GraphNodeKind) {
    const slot = Math.max(nodes.length, spawnCount)
    spawnCount = slot + 1
    const p = scalePoint(80 + (slot % 3) * 280, 96 + Math.floor(slot / 3) * 120)
    const title = kind === 'task' ? 'New task' : kind === 'decision' ? 'Decision' : kind === 'note' ? 'Note' : kind === 'heading' ? 'Heading' : ''
    let ref: string | null = null
    if (kind === 'task') ref = await createTask(title)
    const nodeId = await createGraphNode(id, kind, p.x, p.y, title, ref)
    await load()
    selected = [nodeId]
    onchange()
  }
  async function addExisting(ref: string, kind: GraphNodeKind) {
    const slot = Math.max(nodes.length, spawnCount)
    spawnCount = slot + 1
    const p = scalePoint(80 + (slot % 3) * 280, 96 + Math.floor(slot / 3) * 120)
    const nodeId = await createGraphNode(id, kind, p.x, p.y, '', ref)
    await load(); selected = [nodeId]; onchange()
  }
  async function addDraggedTask(refId: string, point: { x: number; y: number }, parentId?: string) {
    const p = snap ? scalePoint(point.x, point.y) : point
    const nodeId = await createGraphNode(id, 'task', p.x, p.y, '', refId)
    if (parentId) await createGraphEdge(id, parentId, nodeId, 'then')
    await load()
    selected = [nodeId]
    onchange()
  }
  function dragOverTask(e: DragEvent) {
    if (e.dataTransfer?.types.includes('application/x-orbit-task')) e.preventDefault()
  }
  function dropTask(e: DragEvent, parent?: GraphNode) {
    const refId = e.dataTransfer?.getData('application/x-orbit-task')
    if (!refId || !el) return
    e.preventDefault()
    e.stopPropagation()
    const p = world(e)
    void addDraggedTask(refId, parent ? { x: parent.x + 320, y: parent.y } : p, parent?.id)
  }
  async function duplicateNode(node: GraphNode) {
    const copy = await createGraphNode(id, node.node_kind, node.x + 32, node.y + 32, node.title, node.ref_id)
    await load()
    selected = [copy]
    selectedEdge = null
    contextMenu = null
    onchange()
  }
  function openContext(e: MouseEvent, item: { nodeId?: string; edgeId?: string }) {
    e.preventDefault()
    if (!el) return
    const bounds = el.getBoundingClientRect()
    contextMenu = {
      x: Math.max(8, Math.min(bounds.width - 180, e.clientX - bounds.left)),
      y: Math.max(8, Math.min(bounds.height - 112, e.clientY - bounds.top)),
      ...item,
    }
    selected = item.nodeId ? [item.nodeId] : []
    selectedEdge = item.edgeId ?? null
  }
  async function tidy() {
    const incoming = new Map(nodes.map((n) => [n.id, 0]))
    const next = new Map<string, string[]>()
    for (const e of visibleEdges) {
      incoming.set(e.target_id, (incoming.get(e.target_id) ?? 0) + 1)
      next.set(e.source_id, [...(next.get(e.source_id) ?? []), e.target_id])
    }
    const rank = new Map<string, number>()
    const queue = nodes.filter((n) => !incoming.get(n.id)).map((n) => n.id)
    for (const n of queue) rank.set(n, 0)
    for (let i = 0; i < queue.length; i++) for (const to of next.get(queue[i]) ?? []) {
      const r = (rank.get(queue[i]) ?? 0) + 1
      if (r > (rank.get(to) ?? -1)) rank.set(to, r)
      if ((incoming.get(to) ?? 0) > 0) {
        incoming.set(to, incoming.get(to)! - 1)
        if (!incoming.get(to)) queue.push(to)
      }
    }
    const ordered = [...nodes].sort((a, b) => (rank.get(a.id) ?? 0) - (rank.get(b.id) ?? 0) || a.title.localeCompare(b.title))
    const slots = new Map<number, number>()
    for (const n of ordered) {
      const r = rank.get(n.id) ?? 0, slot = slots.get(r) ?? 0
      slots.set(r, slot + 1)
      await moveGraphNode(n.id, 80 + r * 320, 80 + slot * 120)
    }
    await load(); fit(); onchange()
  }
  async function key(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest('[data-ui],input,select,textarea')) return
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      const node = nodeById.get(selected[0])
      if (node && el) {
        contextMenu = {
          x: Math.max(8, Math.min(el.clientWidth - 180, view.x + node.x * view.z)),
          y: Math.max(8, Math.min(el.clientHeight - 112, view.y + node.y * view.z)),
          nodeId: node.id,
        }
        e.preventDefault()
      }
      return
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z' && undo.length) {
      e.preventDefault()
      for (const n of undo) await moveGraphNode(n.id, n.x, n.y)
      undo = []; await load(); onchange(); return
    }
    if (e.key === 'Escape') { selected = []; selectedEdge = null; connectFrom = null; contextMenu = null }
    else if (e.key === 'Delete' || e.key === 'Backspace') {
      if (selectedEdge) await deleteGraphEdge(selectedEdge)
      else for (const nodeId of selected) await deleteItem(nodeId)
      selected = []; selectedEdge = null; await load(); onchange()
    } else if (e.key === 'Tab' && nodes.length) {
      e.preventDefault()
      const at = nodes.findIndex((n) => n.id === selected[0])
      selected = [nodes[(at + 1) % nodes.length].id]
      selectedEdge = null
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      if (!selected.length) return
      e.preventDefault()
      const dx = e.key === 'ArrowLeft' ? -12 : e.key === 'ArrowRight' ? 12 : 0
      const dy = e.key === 'ArrowUp' ? -12 : e.key === 'ArrowDown' ? 12 : 0
      undo = selected.flatMap((nodeId) => { const n = nodeById.get(nodeId); return n ? [{ id: n.id, x: n.x, y: n.y }] : [] })
      for (const nodeId of selected) { const n = nodeById.get(nodeId); if (n) await moveGraphNode(nodeId, n.x + dx, n.y + dy) }
      await load(); onchange()
    } else if (e.key === 'Enter' && focusedNode?.ref_id) {
      const kind = focusedNode.node_kind
      location.hash = kind === 'task' ? `#/task/${focusedNode.ref_id}` : kind === 'project' ? `#/project/${focusedNode.ref_id}` : '#/goals'
    }
  }
  function zoom(f: number) {
    if (!el) return
    const cx = el.clientWidth / 2, cy = el.clientHeight / 2
    const z = Math.max(0.4, Math.min(2, view.z * f))
    view = { x: cx - (cx - view.x) * z / view.z, y: cy - (cy - view.y) * z / view.z, z }
  }
  async function saveTitle() {
    if (focusedNode && draftTitle.trim() && draftTitle.trim() !== focusedNode.title) {
      await updateGraphNode(focusedNode.id, { title: draftTitle.trim() }); await load(); onchange()
    }
  }
  function branch(edge: GraphEdge) {
    void decideGraphNode(edge.source_id, edge.id).then(async () => { await load(); onchange() })
  }
</script>

<div class="relative h-[62vh] min-h-[25rem] overflow-hidden rounded-2xl border border-line bg-bg md:h-[calc(100vh/var(--zoom,1)-190px)]">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    bind:this={el}
    role="application"
    aria-label="Roadmap graph. Drag nodes to move them. Shift drag to select an area. Drag the background to pan."
    tabindex="0"
    onkeydown={key}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => { pan = null; drag = null; linkDrag = null; endpointDrag = null; marquee = null }}
    ondragover={dragOverTask}
    ondrop={(e) => dropTask(e)}
    onwheel={(e) => { if (e.ctrlKey || e.metaKey) { e.preventDefault(); zoom(Math.exp(-e.deltaY * 0.002)) } else view = { ...view, x: view.x - e.deltaX, y: view.y - e.deltaY } }}
    class="absolute inset-0 touch-none outline-none focus-visible:ring-2 focus-visible:ring-accent"
    style="background-image: radial-gradient(color-mix(in srgb, var(--border-strong) 70%, transparent) 1px, transparent 1px); background-size: {24 * view.z}px {24 * view.z}px; background-position: {view.x}px {view.y}px"
  >
    <div class="absolute left-0 top-0 origin-top-left" style="transform: translate({view.x}px,{view.y}px) scale({view.z})">
      {#each bandBoxes as band (band.id)}
        <div aria-hidden="true" class="absolute rounded-xl border border-dashed border-border-strong bg-sunk/40" style="left:{band.x}px;top:{band.y}px;width:{band.w}px;height:{band.h}px">
          <span class="absolute -top-6 left-1 text-xs font-semibold text-muted">{band.name}</span>
        </div>
      {/each}
      <svg class="pointer-events-none absolute h-px w-px overflow-visible" aria-hidden="true">
        <defs><marker id="graph-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 1L9 5L0 9z" fill="var(--icon)" /></marker></defs>
        {#each visibleEdges as edge (edge.id)}
          {@const a = nodeById.get(edge.source_id)}{@const b = nodeById.get(edge.target_id)}
          {#if a && b}
            <path data-edge={edge.id} d={path(a,b)} fill="none" stroke={edgeColor(edge)} stroke-width={selectedEdge === edge.id ? 3 : 1.7} stroke-dasharray={edge.edge_kind === 'depends_on' ? '6 4' : undefined} marker-end="url(#graph-arrow)" class="pointer-events-auto cursor-pointer" role="button" tabindex="0" aria-label="Select connection {edge.label || edge.edge_kind}" onclick={(e) => { e.stopPropagation(); selectedEdge = edge.id; selected = [] }} oncontextmenu={(e) => openContext(e, { edgeId: edge.id })} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { selectedEdge = edge.id; selected = []; e.preventDefault() } }} />
            <text x={(a.x + b.x + 240) / 2} y={(a.y + b.y + 56) / 2 - 8} fill="var(--muted)" font-size="12" text-anchor="middle">{edge.label || edge.edge_kind.replace('_', ' ')}</text>
            {#if selectedEdge === edge.id}
              {@const points = edgePoints(a, b)}
              <circle data-edge-end="source:{edge.id}" cx={points.x1 + (points.x2 >= points.x1 ? 7 : -7)} cy={points.y1} r="6" fill="var(--surface)" stroke="var(--accent)" stroke-width="2" class="pointer-events-auto cursor-crosshair" aria-label="Drag to reroute connection source" />
              <circle data-edge-end="target:{edge.id}" cx={points.x2 + (points.x2 >= points.x1 ? -7 : 7)} cy={points.y2} r="6" fill="var(--surface)" stroke="var(--accent)" stroke-width="2" class="pointer-events-auto cursor-crosshair" aria-label="Drag to reroute connection target" />
            {/if}
          {/if}
        {/each}
        {#if linkDrag}
          {@const source = nodeById.get(linkDrag.from)}
          {#if source}<path d="M {source.x + 240} {source.y + 28} L {linkDrag.x} {linkDrag.y}" fill="none" stroke="var(--accent)" stroke-width="2" stroke-dasharray="5 4" />{/if}
        {/if}
        {#if endpointDrag}
          {@const activeDrag = endpointDrag}
          {@const edge = edges.find((item) => item.id === activeDrag.edgeId)}
          {@const a = edge ? nodeById.get(edge.source_id) : undefined}{@const b = edge ? nodeById.get(edge.target_id) : undefined}
        {#if a && b}{@const points = edgePoints(a, b)}<path d={activeDrag.endpoint === 'source' ? `M ${activeDrag.x} ${activeDrag.y} L ${points.x2} ${points.y2}` : `M ${points.x1} ${points.y1} L ${activeDrag.x} ${activeDrag.y}`} fill="none" stroke="var(--accent)" stroke-width="2" stroke-dasharray="5 4" />{/if}
        {/if}
        {#if alignmentGuides.x !== undefined}<line x1={alignmentGuides.x} x2={alignmentGuides.x} y1={minimapBounds.y} y2={minimapBounds.y + minimapBounds.height} stroke="var(--accent)" stroke-width="1" stroke-dasharray="4 4" />{/if}
        {#if alignmentGuides.y !== undefined}<line x1={minimapBounds.x} x2={minimapBounds.x + minimapBounds.width} y1={alignmentGuides.y} y2={alignmentGuides.y} stroke="var(--accent)" stroke-width="1" stroke-dasharray="4 4" />{/if}
      </svg>
      {#each renderedNodes as node (node.id)}
        <button
          type="button"
          data-node={node.id}
          aria-label="{node.node_kind}: {node.title || node.node_kind}"
          title={node.title || node.node_kind}
          class="absolute flex min-h-14 w-60 items-center gap-2 border border-line bg-surface px-3 py-2 text-left text-sm shadow-1 {node.node_kind === 'decision' ? 'rounded-[26px] border-warn/50 bg-warn-soft' : node.node_kind === 'note' ? 'rounded-md bg-sunk' : node.node_kind === 'heading' || node.node_kind === 'text' ? 'rounded-none border-transparent bg-transparent shadow-none' : 'rounded-xl'} {selected.includes(node.id) ? 'ring-2 ring-accent' : ''} {node.decided_at ? 'opacity-60' : ''}"
          style="left:{node.x + (drag?.moved && selected.includes(node.id) ? drag.x - drag.startX : 0)}px;top:{node.y + (drag?.moved && selected.includes(node.id) ? drag.y - drag.startY : 0)}px"
          onclick={(e) => { if (!e.shiftKey) { selected = [node.id]; selectedEdge = null } }}
          onpointerdown={(e) => { if (!e.shiftKey && !connectFrom) { selected = [node.id]; selectedEdge = null } }}
          oncontextmenu={(e) => openContext(e, { nodeId: node.id })}
          ondragover={dragOverTask}
          ondrop={(e) => dropTask(e, node)}
          onfocus={() => { selected = [node.id]; selectedEdge = null }}
        >
          {#if node.node_kind === 'decision'}<Diamond class="size-4 shrink-0 text-warn" />
          {:else if node.node_kind === 'note'}<FileText class="size-4 shrink-0 text-icon" />
          {:else if node.node_kind === 'heading'}<Heading class="size-4 shrink-0 text-accent" />
          {:else if node.node_kind === 'task'}<Circle class="size-4 shrink-0 text-accent" />
          {:else}<Link class="size-4 shrink-0 text-icon" />{/if}
          <span class="min-w-0 flex-1 whitespace-normal break-words" title={node.title}>{node.title || `Untitled ${node.node_kind}`}</span>
          <span data-connect={node.id} title="Drag to connect" class="absolute -right-1 top-1/2 size-2 -translate-y-1/2 cursor-crosshair rounded-full border border-accent bg-surface" aria-hidden="true"></span>
        </button>
      {/each}
      {#if marquee}<div class="pointer-events-none absolute border border-accent bg-accent-soft/30" style="left:{marquee.x}px;top:{marquee.y}px;width:{marquee.w}px;height:{marquee.h}px"></div>{/if}
    </div>
  </div>

  <details data-ui class="absolute left-2 top-14 z-10 max-h-[45%] w-56 overflow-hidden rounded-xl border border-line bg-surface/95 shadow-1">
    <summary class="cursor-pointer px-3 py-2 text-xs font-semibold">Task source · {taskChoices.length}</summary>
    <div class="max-h-56 overflow-y-auto border-t border-line p-1">
      {#each taskChoices as task (task.id)}
        <button type="button" draggable="true" ondragstart={(e) => { e.dataTransfer?.setData('application/x-orbit-task', task.id); e.dataTransfer?.setData('text/plain', task.title); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy' }} onclick={() => addExisting(task.id, 'task')} class="block w-full truncate rounded-md px-2 py-1.5 text-left text-xs hover:bg-hover" title="Drag onto the canvas or a node; click to add at the default position">{task.title}</button>
      {:else}<p class="px-2 py-2 text-xs text-muted">All open tasks are already on this graph.</p>{/each}
    </div>
  </details>

  <div data-ui class="absolute left-2 top-2 z-10 flex max-w-[calc(100%-16px)] flex-wrap items-center gap-1 rounded-xl border border-line bg-surface/95 p-1 shadow-1">
    <Button size="sm" variant="primary" onclick={() => add('task')}><Plus class="size-4"/><span class="hidden sm:inline">Add</span></Button>
    <Button size="sm" variant="ghost" aria-label="Add decision" onclick={() => add('decision')}><Diamond class="size-4"/><span class="hidden sm:inline">Decision</span></Button>
    <Button size="sm" variant="ghost" aria-label="Add note" onclick={() => add('note')}><FileText class="size-4"/><span class="hidden sm:inline">Note</span></Button>
    <Button size="sm" variant="ghost" aria-label="Add roadmap band" onclick={async () => { await createPhase(id, `Stage ${bands.length + 1}`); await load(); onchange() }}><Plus class="size-4"/><span class="hidden sm:inline">Band</span></Button>
    <select aria-label="Add existing item" class="h-8 max-w-32 rounded-md border border-line bg-surface px-1.5 text-xs" onchange={(e) => { const [kind, ref] = e.currentTarget.value.split('|'); if (ref) void addExisting(ref, kind as GraphNodeKind); e.currentTarget.value = '' }}>
      <option value="">Add existing…</option>{#each choices.filter((c) => !nodes.some((n) => n.ref_id === c.id)) as choice (choice.id)}<option value="{choice.kind}|{choice.id}">{choice.title}</option>{/each}
    </select>
    <Button size="sm" variant="ghost" aria-label="Tidy up graph" onclick={tidy}><Sparkles class="size-4"/><span class="hidden sm:inline">Tidy up</span></Button>
    <label class="flex h-8 items-center gap-1 px-1 text-xs text-muted"><input type="checkbox" bind:checked={snap}/> Snap</label>
  </div>

  {#if focusedNode || focusedEdge}
    <aside data-ui class="absolute right-2 top-14 z-10 w-[min(19rem,calc(100%-16px))] rounded-xl border border-line bg-surface p-3 shadow-2 md:top-2">
      <div class="mb-2 flex items-center"><strong class="flex-1 text-sm">{focusedEdge ? 'Connection' : focusedNode?.node_kind}</strong><Button size="sm" variant="ghost" aria-label="Close selection" onclick={() => { selected=[]; selectedEdge=null }}><X class="size-4"/></Button></div>
      {#if focusedNode}
        {#if !focusedNode.ref_id}<input aria-label="Node title" value={focusedNode.title} onfocus={(e) => draftTitle=e.currentTarget.value} oninput={(e) => draftTitle=e.currentTarget.value} onblur={saveTitle} onkeydown={(e) => e.key==='Enter' && e.currentTarget.blur()} class="h-9 w-full rounded-md border border-line bg-bg px-2 text-sm" placeholder="Write a title" />{/if}
        <label class="mt-2 block text-xs text-muted">Band<select aria-label="Node band" value={focusedNode.band_id ?? ''} onchange={async (e) => { await updateGraphNode(focusedNode!.id, { band_id: e.currentTarget.value || null }); await load(); onchange() }} class="mt-1 h-8 w-full rounded-md border border-line bg-bg px-2 text-sm"><option value="">No band</option>{#each bands as band (band.id)}<option value={band.id}>{band.name}</option>{/each}</select></label>
        {#if focusedNode.node_kind === 'decision'}
          <p class="mt-2 text-xs text-muted">Choose a branch to hide the untaken path. Hidden branches remain saved.</p>
          <div class="mt-2 flex flex-col gap-1">{#each edges.filter((e) => e.source_id === focusedNode.id) as edge (edge.id)}<Button size="sm" variant={focusedNode.taken_edge_id === edge.id ? 'primary' : 'ghost'} onclick={() => branch(edge)}>{edge.label || 'Choose branch'}</Button>{/each}</div>
        {/if}
        <div class="mt-2 flex flex-wrap gap-1"><Button size="sm" variant="ghost" onclick={() => { connectFrom=focusedNode!.id; selectedEdge=null }}>Connect to…</Button><Button size="sm" variant="ghost" onclick={() => duplicateNode(focusedNode!)}>Duplicate</Button><Button size="sm" variant="ghost" aria-label="Delete node" onclick={async () => { await deleteItem(focusedNode!.id); selected=[]; await load(); onchange() }}><Trash2 class="size-4"/></Button></div>
      {:else if focusedEdge}
        <select aria-label="Connection kind" value={focusedEdge.edge_kind} onchange={async (e) => { await updateGraphEdge(focusedEdge!.id,{kind:e.currentTarget.value as GraphEdgeKind}); await load(); onchange() }} class="h-8 w-full rounded-md border border-line bg-bg px-2 text-sm"><option value="then">Then</option><option value="blocks">Blocks</option><option value="depends_on">Depends on</option><option value="decides">Decides</option></select>
        <input aria-label="Connection label" value={focusedEdge.label} onblur={async(e) => { await updateGraphEdge(focusedEdge!.id,{label:e.currentTarget.value}); await load(); onchange() }} placeholder="Label (for example yes)" class="mt-2 h-8 w-full rounded-md border border-line bg-bg px-2 text-sm" />
        <p class="mt-2 text-xs text-muted">Drag either endpoint to a node to reroute while keeping this connection.</p>
        <div class="mt-2 flex gap-1"><select aria-label="Reconnect source" value={focusedEdge.source_id} onchange={(e) => rerouteEdge(focusedEdge!, 'source', e.currentTarget.value)} class="h-8 min-w-0 flex-1 rounded-md border border-line bg-bg px-1 text-xs">{#each nodes as n (n.id)}<option value={n.id} disabled={n.id === focusedEdge.target_id}>{n.title||n.node_kind}</option>{/each}</select><select aria-label="Reconnect target" value={focusedEdge.target_id} onchange={(e) => rerouteEdge(focusedEdge!, 'target', e.currentTarget.value)} class="h-8 min-w-0 flex-1 rounded-md border border-line bg-bg px-1 text-xs">{#each nodes as n (n.id)}<option value={n.id} disabled={n.id === focusedEdge.source_id}>{n.title||n.node_kind}</option>{/each}</select><Button size="sm" variant="ghost" aria-label="Delete connection" onclick={async () => { await deleteGraphEdge(focusedEdge!.id); selectedEdge=null; await load(); onchange() }}><Trash2 class="size-4"/></Button></div>
      {/if}
    </aside>
  {/if}

  {#if connectFrom}
    <div data-ui class="absolute bottom-14 left-2 z-10 flex flex-wrap items-center gap-1 rounded-lg border border-line bg-surface p-2 shadow-1">
      <span class="px-1 text-xs text-muted">Choose a node to connect</span><select aria-label="New connection type" bind:value={edgeKind} class="h-8 rounded-md border border-line bg-bg px-1 text-xs"><option value="then">Then</option><option value="blocks">Blocks</option><option value="depends_on">Depends on</option><option value="decides">Decides</option></select><input aria-label="New connection label" bind:value={edgeLabel} placeholder="Label" class="h-8 w-24 rounded-md border border-line bg-bg px-2 text-xs"/><Button size="sm" variant="ghost" onclick={() => connectFrom=null}>Cancel</Button>
    </div>
  {/if}
  <div data-ui class="absolute bottom-2 left-2 z-10 flex items-center gap-0.5 rounded-lg border border-line bg-surface p-1 shadow-1">
    <Button size="sm" variant="ghost" aria-label="Zoom out" onclick={() => zoom(1/1.2)}><Minus class="size-4"/></Button><span class="w-10 text-center text-xs tabular-nums">{Math.round(view.z*100)}%</span><Button size="sm" variant="ghost" aria-label="Zoom in" onclick={() => zoom(1.2)}><Plus class="size-4"/></Button><Button size="sm" variant="ghost" aria-label="Fit to screen" onclick={fit}><Maximize class="size-4"/></Button>
  </div>
  <div data-ui class="absolute bottom-2 right-2 z-10">
    <Button size="sm" variant="ghost" aria-label="Toggle minimap" onclick={() => showMinimap=!showMinimap}>Map</Button>
    {#if showMinimap}<svg aria-label="Roadmap minimap" class="mt-1 h-24 w-36 rounded-lg border border-line bg-surface p-1" viewBox="{minimapBounds.x} {minimapBounds.y} {minimapBounds.width} {minimapBounds.height}">{#each visibleEdges as e (e.id)}{@const a=nodeById.get(e.source_id)}{@const b=nodeById.get(e.target_id)}{#if a&&b}{@const points = edgePoints(a,b)}<path d="M{points.x1} {points.y1} L{points.x2} {points.y2}" stroke={edgeColor(e)} stroke-width="5"/>{/if}{/each}{#each renderedNodes as n (n.id)}<rect x={n.x} y={n.y} width="240" height="64" rx="8" fill={selected.includes(n.id)?'var(--accent)':'var(--sunk)'} stroke="var(--border-strong)" stroke-width="3"/>{/each}</svg>{/if}
  </div>
  {#if contextMenu}
    <div data-ui role="menu" class="absolute z-30 flex min-w-36 flex-col rounded-lg border border-line bg-surface p-1 shadow-2" style="left:{contextMenu.x}px;top:{contextMenu.y}px">
      {#if contextMenu.nodeId}
        <Button role="menuitem" size="sm" variant="ghost" onclick={() => { const node = nodeById.get(contextMenu!.nodeId!); if (node) void duplicateNode(node) }}>Duplicate</Button>
        <Button role="menuitem" size="sm" variant="ghost" onclick={() => { connectFrom = contextMenu!.nodeId!; contextMenu = null }}>Connect to…</Button>
        <Button role="menuitem" size="sm" variant="ghost" onclick={async () => { await deleteItem(contextMenu!.nodeId!); selected = []; contextMenu = null; await load(); onchange() }}>Delete</Button>
      {:else if contextMenu.edgeId}
        <Button role="menuitem" size="sm" variant="ghost" onclick={async () => { await deleteGraphEdge(contextMenu!.edgeId!); selectedEdge = null; contextMenu = null; await load(); onchange() }}>Delete connection</Button>
      {/if}
    </div>
  {/if}
  {#if !nodes.length}<div class="pointer-events-none absolute inset-0 grid place-items-center p-8 text-center text-sm text-muted"><p>Start with a task, decision or note. Drag to arrange the plan, then connect the steps.</p></div>{/if}
</div>
