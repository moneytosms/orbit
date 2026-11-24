<script lang="ts">
  import { onMount, tick } from 'svelte'
  import Circle from '@lucide/svelte/icons/circle'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Diamond from '@lucide/svelte/icons/diamond'
  import FileText from '@lucide/svelte/icons/file-text'
  import Heading from '@lucide/svelte/icons/heading'
  import Link from '@lucide/svelte/icons/link'
  import Maximize from '@lucide/svelte/icons/maximize'
  import Maximize2 from '@lucide/svelte/icons/maximize-2'
  import Minimize2 from '@lucide/svelte/icons/minimize-2'
  import Pencil from '@lucide/svelte/icons/pencil'
  import Minus from '@lucide/svelte/icons/minus'
  import Plus from '@lucide/svelte/icons/plus'
  import Sparkles from '@lucide/svelte/icons/sparkles'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import X from '@lucide/svelte/icons/x'
  import { Button } from '$lib/components/ui/button'
  import HelpTip from '$lib/HelpTip.svelte'
  import { NODE_H, NODE_W, convertTargets, hiddenNodeIds, placeNew, tidyLayout } from '$lib/roadmapGraph'
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
    updateTask,
    type GraphEdge,
    type GraphEdgeKind,
    type GraphNode,
    type GraphNodeKind,
  } from '$lib/sync'

  let { id, onchange, onimport }: { id: string; onchange: () => void; onimport?: () => void } = $props()
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
  let expanded = $state(false)
  let addMenu = $state(false)
  let editing = $state<string | null>(null)
  let fitted = false
  let frame = $state<HTMLDivElement>()
  let box = $state({ ml: 0, w: 0, h: 560 })
  let contextMenu = $state<{ x: number; y: number; nodeId?: string; edgeId?: string } | null>(null)

  const nodeById = $derived(new Map(nodes.map((n) => [n.id, n])))
  const taskChoices = $derived(choices.filter((c) => c.kind === 'task' && !nodes.some((n) => n.ref_id === c.id)))
  const hiddenNodes = $derived(hiddenNodeIds(nodes, edges))
  const collapseKey = (roadmapId: string) => `orbit.roadmap.${roadmapId}.collapsed`
  // Collapsed bands are a per-device view choice, like folded Page sections.
  let collapsed = $state<string[]>([])
  $effect(() => {
    try {
      collapsed = JSON.parse(localStorage.getItem(collapseKey(id)) ?? '[]')
    } catch {
      collapsed = []
    }
  })
  function toggleBand(bandId: string) {
    collapsed = collapsed.includes(bandId) ? collapsed.filter((x) => x !== bandId) : [...collapsed, bandId]
    try {
      localStorage.setItem(collapseKey(id), JSON.stringify(collapsed))
    } catch {
      /* Storage may be unavailable. */
    }
  }
  const unhidden = $derived(nodes.filter((n) => !hiddenNodes.has(n.id)))
  const renderedNodes = $derived(unhidden.filter((n) => !(n.band_id && collapsed.includes(n.band_id))))
  const renderedIds = $derived(new Set(renderedNodes.map((n) => n.id)))
  const visibleEdges = $derived(
    edges.filter((e) => {
      const source = nodeById.get(e.source_id)
      return (
        source &&
        renderedIds.has(e.source_id) &&
        renderedIds.has(e.target_id) &&
        (!source.decided_at || source.taken_edge_id === e.id)
      )
    }),
  )
  const focusedNode = $derived(nodes.find((n) => n.id === selected[0]))
  const focusedEdge = $derived(edges.find((e) => e.id === selectedEdge))
  const bandBoxes = $derived(
    bands.flatMap((band) => {
      const members = unhidden.filter((node) => node.band_id === band.id)
      if (!members.length) return []
      const x = Math.min(...members.map((n) => n.x)) - 24
      const y = Math.min(...members.map((n) => n.y)) - 36
      const right = Math.max(...members.map((n) => n.x + 240)) + 24
      const bottom = Math.max(...members.map((n) => n.y + 64)) + 24
      const shut = collapsed.includes(band.id)
      return [{ ...band, x, y, w: shut ? 288 : right - x, h: shut ? 44 : bottom - y, shut, count: members.length }]
    }),
  )
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
    if (!fitted && el) {
      fitted = true
      await tick()
      fit()
    }
  }
  $effect(() => {
    void id
    load()
  })

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
  const edgeColor = (edge: GraphEdge) =>
    selectedEdge === edge.id
      ? 'var(--accent)'
      : edge.edge_kind === 'blocks'
        ? 'var(--warn)'
        : edge.edge_kind === 'depends_on'
          ? 'var(--accent)'
          : edge.edge_kind === 'decides'
            ? 'var(--warn)'
            : 'var(--icon)'
  /** Whole graph in view, clear of the toolbar; never zoomed in past 100%. */
  function fit() {
    if (!el || !renderedNodes.length) return (view = { x: 32, y: 72, z: 1 })
    const minX = Math.min(...renderedNodes.map((n) => n.x)) - 24
    const minY = Math.min(...renderedNodes.map((n) => n.y)) - 36
    const maxX = Math.max(...renderedNodes.map((n) => n.x + NODE_W)) + 24
    const maxY = Math.max(...renderedNodes.map((n) => n.y + NODE_H + 12)) + 24
    const z = Math.max(0.4, Math.min(1, (el.clientWidth - 48) / (maxX - minX), (el.clientHeight - 96) / (maxY - minY)))
    view = { z, x: (el.clientWidth - (maxX - minX) * z) / 2 - minX * z, y: 72 - minY * z }
  }
  /** Full size by default: stretch to the whole content area below the page header, edge to edge. */
  function measure() {
    const main = frame?.closest('main')
    const pad = frame?.parentElement
    if (!frame || !main || !pad) return
    const m = main.getBoundingClientRect()
    const p = pad.getBoundingClientRect()
    const f = frame.getBoundingClientRect()
    const k = p.width / pad.offsetWidth || 1
    const left = p.left + parseFloat(getComputedStyle(pad).paddingLeft) * k
    const reserve = matchMedia('(min-width: 701px)').matches ? 0 : 80
    box = {
      ml: (m.left - left) / k,
      w: m.width / k,
      h: Math.max(320, main.clientHeight - (f.top - m.top) / k - main.scrollTop - reserve),
    }
  }
  onMount(() => {
    measure()
    const ro = new ResizeObserver(measure)
    const main = frame?.closest('main')
    if (main) ro.observe(main)
    return () => ro.disconnect()
  })
  async function toggleExpand() {
    expanded = !expanded
    await tick()
    fit()
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
      const x = Math.min(marquee.x, p.x),
        y = Math.min(marquee.y, p.y)
      const w = Math.abs(p.x - marquee.x),
        h = Math.abs(p.y - marquee.y)
      marquee = { x, y, w, h }
      selected = nodes.filter((n) => n.x + 240 >= x && n.x <= x + w && n.y + 64 >= y && n.y <= y + h).map((n) => n.id)
    }
  }
  async function up(e: PointerEvent) {
    pan = null
    if (endpointDrag) {
      const current = endpointDrag
      endpointDrag = null
      const targetId = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-node]')
        ?.dataset.node
      const edge = edges.find((item) => item.id === current.edgeId)
      const nodeId = current.endpoint === 'source' ? edge?.source_id : edge?.target_id
      if (edge && targetId && targetId !== nodeId) await rerouteEdge(edge, current.endpoint, targetId)
      return
    }
    if (linkDrag) {
      const from = linkDrag.from
      const target = document
        .elementFromPoint(
          linkDrag.x * view.z + view.x + el!.getBoundingClientRect().left,
          linkDrag.y * view.z + view.y + el!.getBoundingClientRect().top,
        )
        ?.closest<HTMLElement>('[data-node]')?.dataset.node
      linkDrag = null
      if (target && target !== from) await connect(from, target)
      marquee = null
      return
    }
    if (drag) {
      const d = drag
      drag = null
      if (d.moved) {
        const dx0 = d.x - d.startX,
          dy0 = d.y - d.startY
        const dx = snap ? Math.round(dx0 / 24) * 24 : dx0
        const dy = snap ? Math.round(dy0 / 24) * 24 : dy0
        const moving = selected.length ? selected : [d.id]
        undo = moving.map((id) => {
          const n = nodeById.get(id)!
          return { id, x: n.x, y: n.y }
        })
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
  /** Where a new node goes: next to the selected node, else where the user is looking (or `at`). */
  function spot(at?: { x: number; y: number }) {
    const anchor = at ? undefined : focusedNode
    const r = el!.getBoundingClientRect()
    const mid = at ?? world({ clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 })
    const free = placeNew(nodes, anchor, scalePoint(mid.x - NODE_W / 2, mid.y - NODE_H / 2))
    return { anchor, p: anchor || !snap ? free : scalePoint(free.x, free.y) }
  }
  /** Keeps a new node in sight and in its neighbour's band. */
  function reveal(p: { x: number; y: number }) {
    const sx = view.x + p.x * view.z
    const sy = view.y + p.y * view.z
    if (sx < 0 || sy < 56 || sx + NODE_W * view.z > el!.clientWidth || sy + NODE_H * view.z > el!.clientHeight)
      view = {
        ...view,
        x: el!.clientWidth / 2 - (p.x + NODE_W / 2) * view.z,
        y: el!.clientHeight / 2 - (p.y + NODE_H / 2) * view.z,
      }
  }
  const FLOW: GraphNodeKind[] = ['task', 'decision', 'project', 'goal', 'milestone']
  async function place(kind: GraphNodeKind, title: string, ref: string | null, at?: { x: number; y: number }) {
    const { anchor, p } = spot(at)
    const nodeId = await createGraphNode(id, kind, p.x, p.y, title, ref)
    if (anchor?.band_id) await updateGraphNode(nodeId, { band_id: anchor.band_id })
    // A step added after a step continues the flow; notes and headings stand alone.
    if (anchor && FLOW.includes(anchor.node_kind) && (kind === 'task' || kind === 'decision')) {
      await createGraphEdge(id, anchor.id, nodeId, 'then')
      if (anchor.node_kind === 'decision') await decideGraphNode(anchor.id, null)
    }
    await load()
    selected = [nodeId]
    selectedEdge = null
    reveal(p)
    onchange()
    return nodeId
  }
  const DEFAULT_TITLE: Partial<Record<GraphNodeKind, string>> = {
    task: 'New task',
    decision: 'Decision',
    note: 'Note',
    heading: 'Heading',
  }
  async function add(kind: GraphNodeKind, at?: { x: number; y: number }) {
    addMenu = false
    const title = DEFAULT_TITLE[kind] ?? ''
    const ref = kind === 'task' ? await createTask(title) : null
    const nodeId = await place(kind, title, ref, at)
    const made = nodeById.get(nodeId)
    if (made) startEdit(made)
  }
  async function addExisting(ref: string, kind: GraphNodeKind) {
    await place(kind, '', ref)
  }
  const renamable = (n: GraphNode) => !n.ref_id || n.node_kind === 'task'
  function startEdit(node: GraphNode) {
    if (!renamable(node)) return
    contextMenu = null
    editing = node.id
    draftTitle = node.title
  }
  async function commitEdit() {
    const node = nodeById.get(editing ?? '')
    if (!editing) return
    editing = null
    const title = draftTitle.trim()
    if (!node || !title || title === node.title) return
    if (node.ref_id) await updateTask(node.ref_id, { title })
    else await updateGraphNode(node.id, { title })
    await load()
    onchange()
  }
  const focusSelect = (input: HTMLInputElement) => {
    input.focus()
    input.select()
  }
  /** Linked nodes keep their item and become plain nodes; a plain node becomes a task by getting a new real task. */
  async function convertNode(node: GraphNode, kind: GraphNodeKind) {
    contextMenu = null
    if (node.node_kind === 'decision' && node.decided_at) await decideGraphNode(node.id, null)
    if (kind === 'task') {
      const ref = await createTask(node.title || 'New task')
      await updateGraphNode(node.id, { kind, ref_id: ref, title: '' })
    } else await updateGraphNode(node.id, { kind, ref_id: null, title: node.title })
    await load()
    onchange()
  }
  const KIND_LABEL: Record<string, string> = { task: 'Task', decision: 'Decision', note: 'Note', heading: 'Heading' }
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
      y: Math.max(8, Math.min(bounds.height - 280, e.clientY - bounds.top)),
      ...item,
    }
    selected = item.nodeId ? [item.nodeId] : []
    selectedEdge = item.edgeId ?? null
  }
  async function tidy() {
    for (const [nodeId, p] of tidyLayout(nodes, visibleEdges)) await moveGraphNode(nodeId, p.x, p.y)
    await load()
    fit()
    onchange()
  }
  async function key(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest('[data-ui],input,select,textarea')) return
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      const node = nodeById.get(selected[0])
      if (node && el) {
        contextMenu = {
          x: Math.max(8, Math.min(el.clientWidth - 180, view.x + node.x * view.z)),
          y: Math.max(8, Math.min(el.clientHeight - 280, view.y + node.y * view.z)),
          nodeId: node.id,
        }
        e.preventDefault()
      }
      return
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z' && undo.length) {
      e.preventDefault()
      for (const n of undo) await moveGraphNode(n.id, n.x, n.y)
      undo = []
      await load()
      onchange()
      return
    }
    if (e.key === 'Escape') {
      // Esc first clears; once nothing is left to clear the window handler leaves the full canvas.
      if (selected.length || selectedEdge || connectFrom || contextMenu || addMenu) e.preventDefault()
      addMenu = false
      selected = []
      selectedEdge = null
      connectFrom = null
      contextMenu = null
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      if (selectedEdge) await deleteGraphEdge(selectedEdge)
      else for (const nodeId of selected) await deleteItem(nodeId)
      selected = []
      selectedEdge = null
      await load()
      onchange()
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
      undo = selected.flatMap((nodeId) => {
        const n = nodeById.get(nodeId)
        return n ? [{ id: n.id, x: n.x, y: n.y }] : []
      })
      for (const nodeId of selected) {
        const n = nodeById.get(nodeId)
        if (n) await moveGraphNode(nodeId, n.x + dx, n.y + dy)
      }
      await load()
      onchange()
    } else if (e.key === 'F2' && focusedNode) {
      e.preventDefault()
      startEdit(focusedNode)
    } else if (e.key === 'Enter' && focusedNode?.ref_id) {
      const kind = focusedNode.node_kind
      location.hash =
        kind === 'task'
          ? `#/task/${focusedNode.ref_id}`
          : kind === 'project'
            ? `#/project/${focusedNode.ref_id}`
            : '#/goals'
    }
  }
  function zoom(f: number) {
    if (!el) return
    const cx = el.clientWidth / 2,
      cy = el.clientHeight / 2
    const z = Math.max(0.4, Math.min(2, view.z * f))
    view = { x: cx - ((cx - view.x) * z) / view.z, y: cy - ((cy - view.y) * z) / view.z, z }
  }
  let bandHint = $state(false)
  $effect(() => {
    if (selected.length) bandHint = false
  })
  /** A band is drawn around its nodes, so a new one takes the selection; with nothing selected it waits for nodes. */
  async function addBand() {
    const bandId = await createPhase(id, `Stage ${bands.length + 1}`)
    for (const nodeId of selected) await updateGraphNode(nodeId, { band_id: bandId })
    bandHint = !selected.length
    await load()
    onchange()
  }
  async function saveTitle() {
    if (focusedNode && draftTitle.trim() && draftTitle.trim() !== focusedNode.title) {
      await updateGraphNode(focusedNode.id, { title: draftTitle.trim() })
      await load()
      onchange()
    }
  }
  function branch(edge: GraphEdge) {
    void decideGraphNode(edge.source_id, edge.id).then(async () => {
      await load()
      onchange()
    })
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (
      expanded &&
      e.key === 'Escape' &&
      !e.defaultPrevented &&
      !(e.target as HTMLElement).closest('input,select,textarea')
    ) {
      expanded = false
      void tick().then(fit)
    }
  }}
  onresize={measure}
/>
<div bind:this={frame} style="margin-left:{box.ml}px;width:{box.w}px;height:{box.h}px">
  <div
    class={expanded
      ? 'fixed inset-0 z-50 overflow-hidden bg-bg'
      : 'relative size-full overflow-hidden border-y border-line bg-bg'}
  >
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
      onpointercancel={() => {
        pan = null
        drag = null
        linkDrag = null
        endpointDrag = null
        marquee = null
      }}
      ondragover={dragOverTask}
      ondrop={(e) => dropTask(e)}
      ondblclick={(e) => {
        if (!(e.target as HTMLElement).closest('[data-node],[data-ui]')) void add('task', world(e))
      }}
      onwheel={(e) => {
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault()
          zoom(Math.exp(-e.deltaY * 0.002))
        } else view = { ...view, x: view.x - e.deltaX, y: view.y - e.deltaY }
      }}
      class="absolute inset-0 touch-none outline-none focus-visible:ring-2 focus-visible:ring-accent"
      style="background-image: radial-gradient(color-mix(in srgb, var(--border-strong) 70%, transparent) 1px, transparent 1px); background-size: {24 *
        view.z}px {24 * view.z}px; background-position: {view.x}px {view.y}px"
    >
      <div
        class="absolute left-0 top-0 origin-top-left"
        style="transform: translate({view.x}px,{view.y}px) scale({view.z})"
      >
        {#each bandBoxes as band (band.id)}
          <div
            class="pointer-events-none absolute rounded-xl border border-dashed border-border-strong bg-accent-soft/15"
            style="left:{band.x}px;top:{band.y}px;width:{band.w}px;height:{band.h}px"
          >
            <button
              type="button"
              data-ui
              aria-expanded={!band.shut}
              aria-label="{band.shut ? 'Expand' : 'Collapse'} band {band.name}"
              class="pointer-events-auto absolute left-2 top-1.5 flex max-w-[calc(100%-1rem)] items-center gap-1 rounded-md border border-line bg-surface px-1.5 py-0.5 text-xs font-semibold text-text shadow-1 hover:bg-hover"
              onclick={() => toggleBand(band.id)}
            >
              <span aria-hidden="true">{band.shut ? '▸' : '▾'}</span>
              <span class="truncate">{band.name}</span>
              {#if band.start || band.end}<span class="font-normal"
                  >{[band.start, band.end].filter(Boolean).join(' to ')}</span
                >{/if}
              <span class="font-normal text-muted">· {band.count}</span>
            </button>
          </div>
        {/each}
        <svg class="pointer-events-none absolute h-px w-px overflow-visible" aria-hidden="true">
          <defs
            ><marker
              id="graph-arrow"
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="7"
              markerHeight="7"
              orient="auto"><path d="M0 1L9 5L0 9z" fill="var(--icon)" /></marker
            ></defs
          >
          {#each visibleEdges as edge (edge.id)}
            {@const a = nodeById.get(edge.source_id)}{@const b = nodeById.get(edge.target_id)}
            {#if a && b}
              <path
                data-edge={edge.id}
                d={path(a, b)}
                fill="none"
                stroke={edgeColor(edge)}
                stroke-width={selectedEdge === edge.id ? 3 : 1.7}
                stroke-dasharray={edge.edge_kind === 'depends_on' ? '6 4' : undefined}
                marker-end="url(#graph-arrow)"
                class="pointer-events-auto cursor-pointer"
                role="button"
                tabindex="0"
                aria-label="Select connection {edge.label || edge.edge_kind}"
                onclick={(e) => {
                  e.stopPropagation()
                  selectedEdge = edge.id
                  selected = []
                }}
                oncontextmenu={(e) => openContext(e, { edgeId: edge.id })}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    selectedEdge = edge.id
                    selected = []
                    e.preventDefault()
                  }
                }}
              />
              <text
                x={(a.x + b.x + 240) / 2}
                y={(a.y + b.y + 56) / 2 - 8}
                fill="var(--muted)"
                font-size="12"
                text-anchor="middle">{edge.label || edge.edge_kind.replace('_', ' ')}</text
              >
              {#if selectedEdge === edge.id}
                {@const points = edgePoints(a, b)}
                <circle
                  data-edge-end="source:{edge.id}"
                  cx={points.x1 + (points.x2 >= points.x1 ? 7 : -7)}
                  cy={points.y1}
                  r="6"
                  fill="var(--surface)"
                  stroke="var(--accent)"
                  stroke-width="2"
                  class="pointer-events-auto cursor-crosshair"
                  aria-label="Drag to reroute connection source"
                />
                <circle
                  data-edge-end="target:{edge.id}"
                  cx={points.x2 + (points.x2 >= points.x1 ? -7 : 7)}
                  cy={points.y2}
                  r="6"
                  fill="var(--surface)"
                  stroke="var(--accent)"
                  stroke-width="2"
                  class="pointer-events-auto cursor-crosshair"
                  aria-label="Drag to reroute connection target"
                />
              {/if}
            {/if}
          {/each}
          {#if linkDrag}
            {@const source = nodeById.get(linkDrag.from)}
            {#if source}<path
                d="M {source.x + 240} {source.y + 28} L {linkDrag.x} {linkDrag.y}"
                fill="none"
                stroke="var(--accent)"
                stroke-width="2"
                stroke-dasharray="5 4"
              />{/if}
          {/if}
          {#if endpointDrag}
            {@const activeDrag = endpointDrag}
            {@const edge = edges.find((item) => item.id === activeDrag.edgeId)}
            {@const a = edge ? nodeById.get(edge.source_id) : undefined}{@const b = edge
              ? nodeById.get(edge.target_id)
              : undefined}
            {#if a && b}{@const points = edgePoints(a, b)}<path
                d={activeDrag.endpoint === 'source'
                  ? `M ${activeDrag.x} ${activeDrag.y} L ${points.x2} ${points.y2}`
                  : `M ${points.x1} ${points.y1} L ${activeDrag.x} ${activeDrag.y}`}
                fill="none"
                stroke="var(--accent)"
                stroke-width="2"
                stroke-dasharray="5 4"
              />{/if}
          {/if}
          {#if alignmentGuides.x !== undefined}<line
              x1={alignmentGuides.x}
              x2={alignmentGuides.x}
              y1={minimapBounds.y}
              y2={minimapBounds.y + minimapBounds.height}
              stroke="var(--accent)"
              stroke-width="1"
              stroke-dasharray="4 4"
            />{/if}
          {#if alignmentGuides.y !== undefined}<line
              x1={minimapBounds.x}
              x2={minimapBounds.x + minimapBounds.width}
              y1={alignmentGuides.y}
              y2={alignmentGuides.y}
              stroke="var(--accent)"
              stroke-width="1"
              stroke-dasharray="4 4"
            />{/if}
        </svg>
        {#each renderedNodes as node (node.id)}
          <button
            type="button"
            data-node={node.id}
            aria-label="{node.node_kind}: {node.title || node.node_kind}"
            title={node.title || node.node_kind}
            class="absolute flex min-h-14 w-60 items-center gap-2 border border-line bg-surface px-3 py-2 text-left text-sm shadow-1 {node.node_kind ===
            'decision'
              ? 'rounded-[26px] border-warn/50 bg-warn-soft'
              : node.node_kind === 'note'
                ? 'rounded-md border-dashed bg-sunk'
                : node.node_kind === 'heading' || node.node_kind === 'text'
                  ? 'rounded-none border-transparent bg-transparent shadow-none'
                  : 'rounded-xl border-l-4 border-l-accent'} {selected.includes(node.id)
              ? 'ring-2 ring-accent'
              : ''} {node.decided_at ? 'opacity-60' : ''}"
            style="left:{node.x +
              (drag?.moved && selected.includes(node.id) ? drag.x - drag.startX : 0)}px;top:{node.y +
              (drag?.moved && selected.includes(node.id) ? drag.y - drag.startY : 0)}px"
            onclick={(e) => {
              if (!e.shiftKey) {
                selected = [node.id]
                selectedEdge = null
              }
            }}
            onpointerdown={(e) => {
              if (!e.shiftKey && !connectFrom) {
                selected = [node.id]
                selectedEdge = null
              }
            }}
            oncontextmenu={(e) => openContext(e, { nodeId: node.id })}
            ondblclick={() => startEdit(node)}
            ondragover={dragOverTask}
            ondrop={(e) => dropTask(e, node)}
            onfocus={() => {
              selected = [node.id]
              selectedEdge = null
            }}
          >
            {#if node.node_kind === 'decision'}<Diamond class="size-4 shrink-0 text-warn" />
            {:else if node.node_kind === 'note'}<FileText class="size-4 shrink-0 text-icon" />
            {:else if node.node_kind === 'heading'}<Heading class="size-4 shrink-0 text-accent" />
            {:else if node.node_kind === 'task'}<Circle class="size-4 shrink-0 text-accent" />
            {:else}<Link class="size-4 shrink-0 text-icon" />{/if}
            <span
              class="min-w-0 flex-1 whitespace-normal break-words {node.node_kind === 'heading'
                ? 'text-base font-semibold'
                : ''}"
              title={node.title}>{node.title || `Untitled ${node.node_kind}`}</span
            >
            <span
              data-connect={node.id}
              title="Drag to connect"
              class="absolute -right-1 top-1/2 size-2 -translate-y-1/2 cursor-crosshair rounded-full border border-accent bg-surface"
              aria-hidden="true"
            ></span>
          </button>
        {/each}
        {#if editing}{@const target = nodeById.get(editing)}{#if target}<input
              data-ui
              use:focusSelect
              bind:value={draftTitle}
              aria-label="Node title"
              class="absolute z-20 h-14 w-60 rounded-xl border-2 border-accent bg-surface px-3 text-sm shadow-2 outline-none"
              style="left:{target.x}px;top:{target.y}px"
              onblur={commitEdit}
              onkeydown={(e) => {
                e.stopPropagation()
                if (e.key === 'Enter') e.currentTarget.blur()
                else if (e.key === 'Escape') editing = null
              }}
            />{/if}{/if}
        {#if marquee}<div
            class="pointer-events-none absolute border border-accent bg-accent-soft/30"
            style="left:{marquee.x}px;top:{marquee.y}px;width:{marquee.w}px;height:{marquee.h}px"
          ></div>{/if}
      </div>
    </div>

    <details
      data-ui
      class="absolute left-2 top-14 z-10 max-h-[45%] w-56 overflow-hidden rounded-xl border border-line bg-surface/95 shadow-1"
    >
      <summary class="cursor-pointer px-3 py-2 text-xs font-semibold">Task source · {taskChoices.length}</summary>
      <div class="max-h-56 overflow-y-auto border-t border-line p-1">
        {#each taskChoices as task (task.id)}
          <button
            type="button"
            draggable="true"
            ondragstart={(e) => {
              e.dataTransfer?.setData('application/x-orbit-task', task.id)
              e.dataTransfer?.setData('text/plain', task.title)
              if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy'
            }}
            onclick={() => addExisting(task.id, 'task')}
            class="block w-full truncate rounded-md px-2 py-1.5 text-left text-xs hover:bg-hover"
            title="Drag onto the canvas or a node; click to add at the default position">{task.title}</button
          >
        {:else}<p class="px-2 py-2 text-xs text-muted">All open tasks are already on this graph.</p>{/each}
      </div>
    </details>

    <div
      data-ui
      class="absolute left-2 top-2 z-10 flex max-w-[calc(100%-7rem)] flex-wrap items-center gap-1 rounded-xl border border-line bg-surface/95 p-1 shadow-1"
    >
      <div class="relative flex items-center gap-1">
        <div class="flex">
          <Button size="sm" variant="primary" class="rounded-r-none" onclick={() => add('task')}
            ><Plus class="size-4" /><span class="hidden sm:inline">Add</span></Button
          ><Button
            size="sm"
            variant="primary"
            class="rounded-l-none border-l border-bg/30 px-1.5"
            aria-label="Choose what to add"
            aria-haspopup="menu"
            aria-expanded={addMenu}
            onclick={() => (addMenu = !addMenu)}><ChevronDown class="size-4" /></Button
          >
        </div>
        <HelpTip
          label="adding to the roadmap"
          text="Add puts a new task next to the selected item and connects it (or in the middle of the screen if nothing is selected). The arrow adds other things: a Decision, Note, Heading or Band. You can also double-click empty space to add a task exactly there."
        />
        {#if addMenu}
          <div
            role="menu"
            class="absolute left-0 top-full z-30 mt-1 flex w-64 flex-col rounded-xl border border-line bg-surface p-1 shadow-2"
          >
            {#each [{ kind: 'task', label: 'Task', hint: 'Something to do. A real task in your lists.', Icon: Circle }, { kind: 'decision', label: 'Decision', hint: 'A fork: pick which branch you take.', Icon: Diamond }, { kind: 'note', label: 'Note', hint: 'A free-text card, not a task.', Icon: FileText }, { kind: 'heading', label: 'Heading', hint: 'A label to title a part of the map.', Icon: Heading }] as item (item.kind)}
              <button
                type="button"
                role="menuitem"
                class="flex items-start gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-hover"
                onclick={() => add(item.kind as GraphNodeKind)}
                ><item.Icon class="mt-0.5 size-4 shrink-0 text-icon" /><span
                  ><span class="block font-medium">{item.label}</span><span class="block text-xs text-muted"
                    >{item.hint}</span
                  ></span
                ></button
              >
            {/each}
            <button
              type="button"
              role="menuitem"
              class="flex items-start gap-2 rounded-md border-t border-line px-2 py-1.5 text-left text-sm hover:bg-hover"
              onclick={() => {
                addMenu = false
                void addBand()
              }}
              ><Plus class="mt-0.5 size-4 shrink-0 text-icon" /><span
                ><span class="block font-medium">Band</span><span class="block text-xs text-muted"
                  >A labelled stage drawn around the selected nodes.</span
                ></span
              ></button
            >
          </div>
        {/if}
      </div>
      <span class="mx-0.5 h-5 w-px bg-line" aria-hidden="true"></span>
      <select
        aria-label="Add an existing task, project, goal or milestone"
        class="h-8 max-w-36 rounded-md border border-line bg-surface px-1.5 text-xs"
        onchange={(e) => {
          const [kind, ref] = e.currentTarget.value.split('|')
          if (ref) void addExisting(ref, kind as GraphNodeKind)
          e.currentTarget.value = ''
        }}
      >
        <option value="">Add existing…</option
        >{#each choices.filter((c) => !nodes.some((n) => n.ref_id === c.id)) as choice (choice.id)}<option
            value="{choice.kind}|{choice.id}"
            >{choice.kind === 'task' ? '' : `${KIND_LABEL[choice.kind] ?? choice.kind}: `}{choice.title}</option
          >{/each}
      </select>
      <HelpTip
        label="adding an existing item"
        text="Puts something you already have (a task, project, goal or milestone) on the map as a node. It is the same item, not a copy: renaming or completing it here changes it everywhere. You can also drag a task from Task source."
      />
      <span class="mx-0.5 h-5 w-px bg-line" aria-hidden="true"></span>
      <Button size="sm" variant="ghost" aria-label="Tidy up graph" onclick={tidy}
        ><Sparkles class="size-4" /><span class="hidden sm:inline">Tidy up</span></Button
      >
      <label class="flex h-8 items-center gap-1 px-1 text-xs text-muted"
        ><input type="checkbox" bind:checked={snap} /> Snap</label
      >
      <HelpTip
        label="tidy up and snap"
        text="Tidy up arranges the nodes in neat columns following their connections. Snap makes nodes jump to a grid so they line up. Nothing moves on its own unless you press Tidy up."
      />
    </div>
    <div
      data-ui
      class="absolute right-2 top-2 z-10 flex items-center gap-1 rounded-xl border border-line bg-surface/95 p-1 shadow-1"
    >
      {#if onimport}<Button size="sm" variant="ghost" onclick={onimport}>Import…</Button>{/if}
      <Button
        size="sm"
        variant="ghost"
        aria-label={expanded ? 'Exit full canvas' : 'Expand canvas to full screen'}
        onclick={toggleExpand}
        >{#if expanded}<Minimize2 class="size-4" /><span class="hidden sm:inline">Exit</span><kbd
            class="hidden rounded-sm bg-sunk px-1 text-[0.6875rem] text-muted sm:inline">Esc</kbd
          >{:else}<Maximize2 class="size-4" /><span class="hidden sm:inline">Expand</span>{/if}</Button
      >
    </div>

    {#if focusedNode || focusedEdge}
      <aside
        data-ui
        class="absolute right-2 top-14 z-10 w-[min(19rem,calc(100%-16px))] rounded-xl border border-line bg-surface p-3 shadow-2"
      >
        <div class="mb-2 flex items-center">
          <strong class="flex-1 text-sm">{focusedEdge ? 'Connection' : focusedNode?.node_kind}</strong><Button
            size="sm"
            variant="ghost"
            aria-label="Close selection"
            onclick={() => {
              selected = []
              selectedEdge = null
            }}><X class="size-4" /></Button
          >
        </div>
        {#if focusedNode}
          {#if !focusedNode.ref_id}<input
              aria-label="Node title"
              value={focusedNode.title}
              onfocus={(e) => (draftTitle = e.currentTarget.value)}
              oninput={(e) => (draftTitle = e.currentTarget.value)}
              onblur={saveTitle}
              onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
              class="h-9 w-full rounded-md border border-line bg-bg px-2 text-sm"
              placeholder="Write a title"
            />{/if}
          <div class="mt-2 flex items-center gap-1 text-xs text-muted">
            Band<HelpTip
              label="bands"
              text="A band is a labelled stage, like Week 1 or Research, drawn as a box around the nodes you put in it. Use bands to group steps by phase or time. A band can have dates, and it drives the phase's progress in List, Board and Timeline."
            />
          </div>
          <label class="block"
            ><select
              aria-label="Node band"
              value={focusedNode.band_id ?? ''}
              onchange={async (e) => {
                await updateGraphNode(focusedNode!.id, { band_id: e.currentTarget.value || null })
                await load()
                onchange()
              }}
              class="mt-1 h-8 w-full rounded-md border border-line bg-bg px-2 text-sm"
              ><option value="">No band</option>{#each bands as band (band.id)}<option value={band.id}
                  >{band.name}</option
                >{/each}</select
            ></label
          >
          {#if focusedNode.node_kind === 'decision'}
            <p class="mt-2 text-xs text-muted">
              Choose a branch to hide the untaken path. Hidden branches remain saved.
            </p>
            <div class="mt-2 flex flex-col gap-1">
              {#each edges.filter((e) => e.source_id === focusedNode.id) as edge (edge.id)}<Button
                  size="sm"
                  variant={focusedNode.taken_edge_id === edge.id ? 'primary' : 'ghost'}
                  onclick={() => branch(edge)}>{edge.label || 'Choose branch'}</Button
                >{/each}
            </div>
          {/if}
          <div class="mt-2 flex items-center gap-1 text-xs text-muted">
            Convert to<HelpTip
              label="converting"
              text="Change what this node is. A task turned into a note or decision stays in your task lists; only its card on the map changes. A note or decision turned into a task creates a real task with the same title. Titles are never lost."
            />
          </div>
          <div class="mt-1 flex flex-wrap gap-1">
            {#each convertTargets(focusedNode) as t (t.kind)}
              <Button
                size="sm"
                variant="ghost"
                disabled={!t.ok}
                title={t.reason ?? `Turn this into a ${t.kind}`}
                onclick={() => convertNode(focusedNode!, t.kind)}>{KIND_LABEL[t.kind]}</Button
              >
            {/each}
          </div>
          <div class="mt-2 flex flex-wrap items-center gap-1">
            {#if renamable(focusedNode)}<Button size="sm" variant="ghost" onclick={() => startEdit(focusedNode!)}
                ><Pencil class="size-4" />Rename</Button
              >{/if}
            <Button
              size="sm"
              variant="ghost"
              onclick={() => {
                connectFrom = focusedNode!.id
                selectedEdge = null
              }}>Connect to…</Button
            ><HelpTip
              label="connecting"
              text="Connecting draws an arrow from this node to another to show the order: this, then that. You can also drag the little dot on a node's right edge onto another node. Click an arrow to change its kind or label."
            /><Button size="sm" variant="ghost" onclick={() => duplicateNode(focusedNode!)}>Duplicate</Button><Button
              size="sm"
              variant="ghost"
              aria-label="Delete node"
              onclick={async () => {
                await deleteItem(focusedNode!.id)
                selected = []
                await load()
                onchange()
              }}><Trash2 class="size-4" /></Button
            >
          </div>
        {:else if focusedEdge}
          <div class="mb-1 flex items-center gap-1 text-xs text-muted">
            Kind<HelpTip
              label="connection kinds"
              text="Then: the next step. Blocks: the first must finish before the second can start. Depends on: the first needs the second done first. Decides: a branch leaving a Decision."
            />
          </div>
          <select
            aria-label="Connection kind"
            value={focusedEdge.edge_kind}
            onchange={async (e) => {
              await updateGraphEdge(focusedEdge!.id, { kind: e.currentTarget.value as GraphEdgeKind })
              await load()
              onchange()
            }}
            class="h-8 w-full rounded-md border border-line bg-bg px-2 text-sm"
            ><option value="then">Then</option><option value="blocks">Blocks</option><option value="depends_on"
              >Depends on</option
            ><option value="decides">Decides</option></select
          >
          <input
            aria-label="Connection label"
            value={focusedEdge.label}
            onblur={async (e) => {
              await updateGraphEdge(focusedEdge!.id, { label: e.currentTarget.value })
              await load()
              onchange()
            }}
            placeholder="Label (for example yes)"
            class="mt-2 h-8 w-full rounded-md border border-line bg-bg px-2 text-sm"
          />
          <p class="mt-2 text-xs text-muted">
            Drag either endpoint to a node to reroute while keeping this connection.
          </p>
          <div class="mt-2 flex gap-1">
            <select
              aria-label="Reconnect source"
              value={focusedEdge.source_id}
              onchange={(e) => rerouteEdge(focusedEdge!, 'source', e.currentTarget.value)}
              class="h-8 min-w-0 flex-1 rounded-md border border-line bg-bg px-1 text-xs"
              >{#each nodes as n (n.id)}<option value={n.id} disabled={n.id === focusedEdge.target_id}
                  >{n.title || n.node_kind}</option
                >{/each}</select
            ><select
              aria-label="Reconnect target"
              value={focusedEdge.target_id}
              onchange={(e) => rerouteEdge(focusedEdge!, 'target', e.currentTarget.value)}
              class="h-8 min-w-0 flex-1 rounded-md border border-line bg-bg px-1 text-xs"
              >{#each nodes as n (n.id)}<option value={n.id} disabled={n.id === focusedEdge.source_id}
                  >{n.title || n.node_kind}</option
                >{/each}</select
            ><Button
              size="sm"
              variant="ghost"
              aria-label="Delete connection"
              onclick={async () => {
                await deleteGraphEdge(focusedEdge!.id)
                selectedEdge = null
                await load()
                onchange()
              }}><Trash2 class="size-4" /></Button
            >
          </div>
        {/if}
      </aside>
    {/if}

    {#if connectFrom}
      <div
        data-ui
        class="absolute bottom-14 left-2 z-10 flex flex-wrap items-center gap-1 rounded-lg border border-line bg-surface p-2 shadow-1"
      >
        <span class="px-1 text-xs text-muted">Choose a node to connect</span><select
          aria-label="New connection type"
          bind:value={edgeKind}
          class="h-8 rounded-md border border-line bg-bg px-1 text-xs"
          ><option value="then">Then</option><option value="blocks">Blocks</option><option value="depends_on"
            >Depends on</option
          ><option value="decides">Decides</option></select
        ><input
          aria-label="New connection label"
          bind:value={edgeLabel}
          placeholder="Label"
          class="h-8 w-24 rounded-md border border-line bg-bg px-2 text-xs"
        /><Button size="sm" variant="ghost" onclick={() => (connectFrom = null)}>Cancel</Button>
      </div>
    {/if}
    <div
      data-ui
      class="absolute bottom-2 left-2 z-10 flex items-center gap-0.5 rounded-lg border border-line bg-surface p-1 shadow-1"
    >
      <Button size="sm" variant="ghost" aria-label="Zoom out" onclick={() => zoom(1 / 1.2)}
        ><Minus class="size-4" /></Button
      ><span class="w-10 text-center text-xs tabular-nums">{Math.round(view.z * 100)}%</span><Button
        size="sm"
        variant="ghost"
        aria-label="Zoom in"
        onclick={() => zoom(1.2)}><Plus class="size-4" /></Button
      ><Button size="sm" variant="ghost" aria-label="Fit to screen" onclick={fit}><Maximize class="size-4" /></Button>
    </div>
    <div data-ui class="absolute bottom-2 right-2 z-10">
      <Button size="sm" variant="ghost" aria-label="Toggle minimap" onclick={() => (showMinimap = !showMinimap)}
        >Map</Button
      >
      {#if showMinimap}<svg
          aria-label="Roadmap minimap"
          class="mt-1 h-24 w-36 rounded-lg border border-line bg-surface p-1"
          viewBox="{minimapBounds.x} {minimapBounds.y} {minimapBounds.width} {minimapBounds.height}"
          >{#each visibleEdges as e (e.id)}{@const a = nodeById.get(e.source_id)}{@const b = nodeById.get(
              e.target_id,
            )}{#if a && b}{@const points = edgePoints(a, b)}<path
                d="M{points.x1} {points.y1} L{points.x2} {points.y2}"
                stroke={edgeColor(e)}
                stroke-width="5"
              />{/if}{/each}{#each renderedNodes as n (n.id)}<rect
              x={n.x}
              y={n.y}
              width="240"
              height="64"
              rx="8"
              fill={selected.includes(n.id) ? 'var(--accent)' : 'var(--sunk)'}
              stroke="var(--border-strong)"
              stroke-width="3"
            />{/each}</svg
        >{/if}
    </div>
    {#if contextMenu}
      <div
        data-ui
        role="menu"
        class="absolute z-30 flex min-w-36 flex-col rounded-lg border border-line bg-surface p-1 shadow-2"
        style="left:{contextMenu.x}px;top:{contextMenu.y}px"
      >
        {#if contextMenu.nodeId}
          <Button
            role="menuitem"
            size="sm"
            variant="ghost"
            onclick={() => {
              const node = nodeById.get(contextMenu!.nodeId!)
              if (node) void duplicateNode(node)
            }}>Duplicate</Button
          >
          {#if nodeById.get(contextMenu.nodeId) && renamable(nodeById.get(contextMenu.nodeId)!)}<Button
              role="menuitem"
              size="sm"
              variant="ghost"
              onclick={() => startEdit(nodeById.get(contextMenu!.nodeId!)!)}>Rename</Button
            >{/if}
          <Button
            role="menuitem"
            size="sm"
            variant="ghost"
            onclick={() => {
              connectFrom = contextMenu!.nodeId!
              contextMenu = null
            }}>Connect to…</Button
          >
          {#if nodeById.get(contextMenu.nodeId)}
            <p class="px-2 pt-1 text-[0.6875rem] font-semibold uppercase tracking-wide text-muted">Convert to</p>
            {#each convertTargets(nodeById.get(contextMenu.nodeId)!) as t (t.kind)}
              <Button
                role="menuitem"
                size="sm"
                variant="ghost"
                disabled={!t.ok}
                title={t.reason ?? `Turn this into a ${t.kind}`}
                onclick={() => convertNode(nodeById.get(contextMenu!.nodeId!)!, t.kind)}>{KIND_LABEL[t.kind]}</Button
              >
            {/each}
          {/if}
          <Button
            role="menuitem"
            size="sm"
            variant="ghost"
            onclick={async () => {
              await deleteItem(contextMenu!.nodeId!)
              selected = []
              contextMenu = null
              await load()
              onchange()
            }}>Delete</Button
          >
        {:else if contextMenu.edgeId}
          <Button
            role="menuitem"
            size="sm"
            variant="ghost"
            onclick={async () => {
              await deleteGraphEdge(contextMenu!.edgeId!)
              selectedEdge = null
              contextMenu = null
              await load()
              onchange()
            }}>Delete connection</Button
          >
        {/if}
      </div>
    {/if}
    {#if bandHint}<p
        data-ui
        role="status"
        class="absolute bottom-14 right-2 z-10 max-w-60 rounded-lg border border-line bg-surface p-2 text-xs text-muted shadow-1"
      >
        Band created. Select nodes and choose it under Band in the side panel; the band appears around them.
      </p>{/if}
    {#if !nodes.length}<div class="pointer-events-none absolute inset-0 grid place-items-center p-6">
        <div
          data-ui
          class="pointer-events-auto flex max-w-sm flex-col items-center gap-3 rounded-2xl border border-line bg-surface p-6 text-center shadow-1"
        >
          <p class="text-base font-semibold">This roadmap is empty</p>
          <p class="text-sm text-muted">
            A roadmap is a map of the steps toward a goal. Add your first step, then add the next one after it: new
            steps connect to the selected one automatically.
          </p>
          <div class="flex flex-wrap justify-center gap-2">
            <Button variant="primary" onclick={() => add('task')}><Plus class="size-4" />Add first step</Button>
            {#if onimport}<Button variant="ghost" onclick={onimport}>Import from code…</Button>{/if}
          </div>
          <p class="text-xs text-muted">
            Tip: double-click anywhere to add a step. Drag the dot on a node's right edge to connect it. The ? buttons
            explain each tool.
          </p>
        </div>
      </div>{/if}
  </div>
</div>
