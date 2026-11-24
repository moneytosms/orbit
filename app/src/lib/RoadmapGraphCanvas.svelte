<script lang="ts">
  import { onMount, tick } from 'svelte'
  import Circle from '@lucide/svelte/icons/circle'
  import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right'
  import Grid3x3 from '@lucide/svelte/icons/grid-3x3'
  import Hand from '@lucide/svelte/icons/hand'
  import ListPlus from '@lucide/svelte/icons/list-plus'
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2'
  import SquareDashed from '@lucide/svelte/icons/square-dashed'
  import Undo2 from '@lucide/svelte/icons/undo-2'
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
  import {
    NODE_H,
    NODE_W,
    borderPoint,
    convertTargets,
    edgeEnds,
    hiddenNodeIds,
    placeNew,
    tidyLayout,
    type TidyDir,
  } from '$lib/roadmapGraph'
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
    updatePhase,
    updateTask,
    type GraphEdge,
    type GraphEdgeKind,
    type GraphNode,
    type GraphNodeKind,
  } from '$lib/sync'

  let {
    id,
    reload = 0,
    onchange,
    onimport,
  }: { id: string; reload?: number; onchange: () => void; onimport?: () => void } = $props()
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
  type Tool = 'select' | 'hand' | 'task' | 'decision' | 'note' | 'heading' | 'connect'
  let tool = $state<Tool>('select')
  let existingOpen = $state(false)
  let tidyOpen = $state(false)
  let spaceDown = $state(false)
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
  const kindClass = (k: GraphNodeKind) =>
    k === 'decision'
      ? 'justify-center px-12 text-center'
      : k === 'note'
        ? 'rounded-md border border-dashed border-line bg-sunk'
        : k === 'heading' || k === 'text'
          ? 'border border-transparent'
          : 'rounded-xl border border-line border-l-4 border-l-accent bg-surface shadow-1'
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
    void reload
    load()
  })

  /** Where a node is drawn right now: connections follow it while it is being dragged. */
  const at = (n?: GraphNode) =>
    n && drag?.moved && selected.includes(n.id)
      ? { ...n, x: n.x + drag.x - drag.startX, y: n.y + drag.y - drag.startY }
      : n
  /** An arrow and its reverse bend apart so both stay visible. */
  const ends = (e: GraphEdge, a: GraphNode, b: GraphNode) =>
    edgeEnds(a, b, edges.some((o) => o.source_id === e.target_id && o.target_id === e.source_id) ? 34 : 0)
  const path = (p: ReturnType<typeof edgeEnds>) => `M ${p.x1} ${p.y1} Q ${p.cx} ${p.cy} ${p.x2} ${p.y2}`
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
    if ((e.target as HTMLElement).closest('[data-ui],input,select,textarea')) return
    if (e.button === 1 || (e.button === 0 && (spaceDown || tool === 'hand'))) {
      e.preventDefault()
      pan = { x: e.clientX, y: e.clientY }
      ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
      return
    }
    if (e.button !== 0) return
    contextMenu = null
    existingOpen = tidyOpen = false
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
    if (tool === 'connect' && target) {
      const id = target.dataset.node!
      if (connectFrom && connectFrom !== id) {
        void connect(connectFrom, id)
        connectFrom = null
        tool = 'select'
      } else {
        linkDrag = { from: id, ...world(e) }
        ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
      }
      return
    }
    if (!target && PLACING.includes(tool)) {
      const kind = tool as GraphNodeKind
      tool = 'select'
      void add(kind, world(e))
      return
    }
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
      if (target && target !== from) {
        await connect(from, target)
        tool = 'select'
      } else if (tool === 'connect') connectFrom = from
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
  const BAND_HELP =
    'A band is a labelled frame drawn around a group of items, so one stage of the plan reads as one block. Examples: "Week 1", "Phase: Research", "Before launch".\n\nWhat it does: it keeps related items visibly together, it can carry a start and end date (then it shows on the Timeline and its progress is counted from the tasks inside it), and you can fold it down to one line to hide the detail.\n\nHow to use it: select some items and press Band in the dock (or press B) to wrap them in a new band named "Stage 1". With one item selected, the Band list in this panel moves it into another band, or "No band" takes it out; rename the band and set its dates right under it. Click the band title on the canvas to collapse or expand it.\n\nA band is only a frame around its members, so it works in any layout: left to right, top to bottom, or scattered.'
  const DOCK_HELP =
    'This dock works like a drawing board. Pick a tool, then click empty space on the canvas to place it. Esc goes back to Select. Click an active tool a second time to drop one in the middle of the view, next to the selected item and joined to it.\n\nSelect (V): click an item, drag to move it, Shift-click or Shift-drag for several. Hand (H): drag to pan; holding Space or the middle mouse button does the same. Scroll or pinch to zoom.\nTask (T): something to do, a real task in your lists. Decision (D): a fork where you choose one way. Note (N): free text that is not a task. Heading (G): a title for part of the map.\nConnector (C): drag from one item to another to draw an arrow meaning "this, then that". Arrows can go in any direction.\nBand (B): frame the selected items as a stage. Add existing (X): put a task, project, goal or milestone you already have on the map; it is the same item, not a copy.\nTidy up: neatly arrange everything, left to right, top to bottom, right to left or bottom to top. Nothing moves unless you press it.\nSnap: items jump to a grid so they line up. Undo reverses the last move.'
  const CONVERT_HELP =
    'Convert changes what an item is without losing its title. A task turned into a note or decision stays in your task lists; only its card on the map changes. A note or decision turned into a task creates a real task with the same title. A goal, project or milestone is its own item, so it cannot become a task.'
  const CONNECT_HELP =
    'Connecting draws an arrow from this item to another to show the order: this, then that. Press Connect to, then click the other item, or use the Connector tool (C) and drag between two items. Arrows may point up, down, left or right, and may loop back. Click an arrow afterwards to change its kind or give it a label.'
  const EDGE_HELP =
    'Then: a plain "next step". Blocks: the first item has to be finished before the second can start. Depends on: the first item needs the second one done first (the reverse of Blocks). Decides: a branch that leaves a Decision; label it, for example "yes" or "funded". Pick a Decision and choose which branch you took to hide the others.'
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
    const title = DEFAULT_TITLE[kind] ?? ''
    const ref = kind === 'task' ? await createTask(title) : null
    // A task node shows its task's title, so the node itself stores none.
    const nodeId = await place(kind, ref ? '' : title, ref, at)
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
    if (node.ref_id) {
      await updateTask(node.ref_id, { title })
      await updateGraphNode(node.id, { title: '' })
    } else await updateGraphNode(node.id, { title })
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
  async function tidy(dir: TidyDir) {
    tidyOpen = false
    for (const [nodeId, p] of tidyLayout(nodes, visibleEdges, dir)) await moveGraphNode(nodeId, p.x, p.y)
    await load()
    fit()
    onchange()
  }
  async function doUndo() {
    for (const n of undo) await moveGraphNode(n.id, n.x, n.y)
    undo = []
    await load()
    onchange()
  }
  const TOOLS: { id: Tool; label: string; key: string; n: string; Icon: typeof Circle; hint: string; gap?: boolean }[] =
    [
      {
        id: 'select',
        label: 'Select',
        key: 'V',
        n: '1',
        Icon: MousePointer2,
        hint: 'Click, drag to move, Shift for many',
      },
      { id: 'hand', label: 'Hand', key: 'H', n: '2', Icon: Hand, hint: 'Drag to pan (or hold Space)' },
      {
        id: 'task',
        label: 'Task',
        key: 'T',
        n: '3',
        Icon: Circle,
        hint: 'A real task. Click the canvas to place',
        gap: true,
      },
      { id: 'decision', label: 'Decision', key: 'D', n: '4', Icon: Diamond, hint: 'A fork. Click the canvas to place' },
      { id: 'note', label: 'Note', key: 'N', n: '5', Icon: FileText, hint: 'Free text. Click the canvas to place' },
      { id: 'heading', label: 'Heading', key: 'G', n: '6', Icon: Heading, hint: 'A title. Click the canvas to place' },
      {
        id: 'connect',
        label: 'Connector',
        key: 'C',
        n: '7',
        Icon: ArrowUpRight,
        hint: 'Drag from one item to another',
        gap: true,
      },
    ]
  const PLACING = ['task', 'decision', 'note', 'heading']
  /** A tool is armed until the next click on the canvas; clicking an armed placing tool drops one at the centre. */
  function pick(id: Tool) {
    existingOpen = tidyOpen = false
    if (tool === id && PLACING.includes(id)) {
      tool = 'select'
      void add(id as GraphNodeKind)
      return
    }
    tool = id
    connectFrom = null
    el?.focus()
  }
  function toolKey(e: KeyboardEvent): boolean {
    if (e.ctrlKey || e.metaKey || e.altKey || (e.target as HTMLElement).closest('input,select,textarea')) return false
    const k = e.key.toLowerCase()
    const t = TOOLS.find((x) => x.key.toLowerCase() === k || x.n === k)
    if (t) pick(t.id)
    else if (k === 'b' || k === '8') void addBand()
    else if (k === 'x' || k === '9') existingOpen = !existingOpen
    else return false
    e.preventDefault()
    return true
  }
  async function key(e: KeyboardEvent) {
    if (e.key === ' ' && !(e.target as HTMLElement).closest('[data-ui],input,select,textarea')) {
      spaceDown = true
      e.preventDefault()
      return
    }
    if (toolKey(e)) return
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
      await doUndo()
      return
    }
    if (e.key === 'Escape') {
      // Esc first clears; once nothing is left to clear the window handler leaves the full canvas.
      if (
        selected.length ||
        selectedEdge ||
        connectFrom ||
        contextMenu ||
        existingOpen ||
        tidyOpen ||
        tool !== 'select'
      )
        e.preventDefault()
      tool = 'select'
      existingOpen = tidyOpen = false
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
      onkeyup={(e) => e.key === ' ' && (spaceDown = false)}
      onblur={() => (spaceDown = false)}
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
              markerUnits="userSpaceOnUse"
              markerWidth="12"
              markerHeight="12"
              orient="auto"><path d="M0 1L9 5L0 9z" fill="var(--icon)" /></marker
            ></defs
          >
          {#each visibleEdges as edge (edge.id)}
            {@const a = at(nodeById.get(edge.source_id))}{@const b = at(nodeById.get(edge.target_id))}
            {#if a && b}{@const p = ends(edge, a, b)}
              <path
                data-edge={edge.id}
                d={path(p)}
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
                x={(p.x1 + 2 * p.cx + p.x2) / 4}
                y={(p.y1 + 2 * p.cy + p.y2) / 4 - 6}
                fill="var(--muted)"
                font-size="12"
                stroke="var(--bg)"
                stroke-width="4"
                paint-order="stroke"
                text-anchor="middle">{edge.label || edge.edge_kind.replace('_', ' ')}</text
              >
              {#if selectedEdge === edge.id}
                <circle
                  data-edge-end="source:{edge.id}"
                  cx={p.x1}
                  cy={p.y1}
                  r="6"
                  fill="var(--surface)"
                  stroke="var(--accent)"
                  stroke-width="2"
                  class="pointer-events-auto cursor-crosshair"
                  aria-label="Drag to reroute connection source"
                />
                <circle
                  data-edge-end="target:{edge.id}"
                  cx={p.x2}
                  cy={p.y2}
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
            {#if source}{@const s0 = borderPoint(at(source)!, linkDrag)}<path
                d="M {s0.x} {s0.y} L {linkDrag.x} {linkDrag.y}"
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
            {#if a && b}{@const fixed =
                activeDrag.endpoint === 'source'
                  ? borderPoint(at(b)!, activeDrag)
                  : borderPoint(at(a)!, activeDrag)}<path
                d={activeDrag.endpoint === 'source'
                  ? `M ${activeDrag.x} ${activeDrag.y} L ${fixed.x} ${fixed.y}`
                  : `M ${fixed.x} ${fixed.y} L ${activeDrag.x} ${activeDrag.y}`}
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
            class="absolute flex h-16 w-60 items-center gap-2 px-3 text-left text-sm {kindClass(
              node.node_kind,
            )} {selected.includes(node.id) && node.node_kind !== 'decision'
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
            {#if node.node_kind === 'decision'}<svg
                aria-hidden="true"
                class="pointer-events-none absolute inset-0 size-full overflow-visible"
                viewBox="0 0 240 64"
                preserveAspectRatio="none"
                ><polygon
                  points="120,0 240,32 120,64 0,32"
                  fill="var(--warn-soft)"
                  stroke={selected.includes(node.id) ? 'var(--accent)' : 'var(--warn)'}
                  stroke-width={selected.includes(node.id) ? 3 : 1.5}
                  stroke-linejoin="round"
                /></svg
              >
            {:else if node.node_kind === 'note'}<FileText class="size-4 shrink-0 text-icon" />
            {:else if node.node_kind === 'heading'}<Heading class="size-4 shrink-0 text-accent" />
            {:else if node.node_kind === 'task'}<Circle class="size-4 shrink-0 text-accent" />
            {:else}<Link class="size-4 shrink-0 text-icon" />{/if}
            <span
              class="relative min-w-0 flex-1 line-clamp-2 whitespace-normal break-words {node.node_kind === 'heading'
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
              class="absolute z-20 h-16 w-60 rounded-xl border-2 border-accent bg-surface px-3 text-sm shadow-2 outline-none"
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
      class="absolute left-2 top-2 z-10 max-h-[45%] w-56 overflow-hidden rounded-xl border border-line bg-surface/95 shadow-1"
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
      class="absolute right-2 top-2 z-10 flex items-center gap-1 rounded-xl border border-line bg-surface/95 p-1 shadow-1"
    >
      {#if onimport}<Button size="sm" variant="ghost" onclick={onimport}>Import…</Button>{/if}
    </div>

    {#if existingOpen}
      <div
        data-ui
        class="absolute bottom-20 left-1/2 z-20 flex max-h-64 w-72 max-w-[calc(100%-1rem)] -translate-x-1/2 flex-col overflow-y-auto rounded-xl border border-line bg-surface p-1 shadow-2"
        role="menu"
        aria-label="Add an existing item"
      >
        <p class="px-2 py-1 text-xs text-muted">Put something you already have on the map. It stays the same item.</p>
        {#each choices.filter((c) => !nodes.some((n) => n.ref_id === c.id)) as choice (choice.id)}
          <button
            type="button"
            role="menuitem"
            class="truncate rounded-md px-2 py-1.5 text-left text-sm hover:bg-hover"
            onclick={() => {
              existingOpen = false
              void addExisting(choice.id, choice.kind)
            }}>{choice.kind === 'task' ? '' : `${KIND_LABEL[choice.kind] ?? choice.kind}: `}{choice.title}</button
          >
        {:else}<p class="px-2 py-2 text-sm text-muted">Everything you have is already on the map.</p>{/each}
      </div>
    {/if}
    {#if tidyOpen}
      <div
        data-ui
        class="absolute bottom-20 left-1/2 z-20 flex w-52 -translate-x-1/2 flex-col rounded-xl border border-line bg-surface p-1 shadow-2"
        role="menu"
        aria-label="Tidy up direction"
      >
        <p class="px-2 py-1 text-xs text-muted">Arrange along connections:</p>
        {#each [['lr', 'Left to right'], ['tb', 'Top to bottom'], ['rl', 'Right to left'], ['bt', 'Bottom to top']] as [dir, label] (dir)}
          <button
            type="button"
            role="menuitem"
            class="rounded-md px-2 py-1.5 text-left text-sm hover:bg-hover"
            onclick={() => tidy(dir as TidyDir)}>{label}</button
          >
        {/each}
      </div>
    {/if}
    {#if PLACING.includes(tool) || tool === 'connect'}
      <p
        data-ui
        role="status"
        class="absolute bottom-20 left-1/2 z-10 -translate-x-1/2 rounded-full border border-line bg-surface px-3 py-1 text-xs text-muted shadow-1"
      >
        {tool === 'connect'
          ? connectFrom
            ? 'Now click the item it leads to'
            : 'Drag from one item to another, or click two items'
          : `Click the canvas to place a ${tool}`} · Esc cancels
      </p>
    {/if}
    <div
      data-ui
      role="toolbar"
      aria-label="Roadmap tools"
      tabindex="-1"
      onkeydown={toolKey}
      class="absolute inset-x-2 bottom-3 z-20 mx-auto flex w-fit flex-wrap items-center justify-center gap-0.5 rounded-2xl border border-line bg-surface/95 p-1 shadow-2"
    >
      {#each TOOLS as t (t.id)}
        {#if t.gap}<span class="mx-0.5 h-6 w-px shrink-0 bg-line" aria-hidden="true"></span>{/if}
        <button
          type="button"
          aria-label="{t.label} ({t.key})"
          title="{t.label} · {t.key} or {t.n}. {t.hint}"
          aria-pressed={tool === t.id}
          onclick={() => pick(t.id)}
          class="relative grid size-9 shrink-0 place-items-center rounded-xl hover:bg-hover {tool === t.id
            ? 'bg-accent-soft text-accent'
            : 'text-icon'}"
          ><t.Icon class="size-[18px]" /><kbd
            class="absolute bottom-0.5 right-1 text-[0.5625rem] leading-none text-muted">{t.n}</kbd
          ></button
        >
      {/each}
      <button
        type="button"
        aria-label="Band (B)"
        title="Band · B or 8. Frame the selected items as a stage"
        onclick={() => void addBand()}
        class="relative grid size-9 shrink-0 place-items-center rounded-xl text-icon hover:bg-hover"
        ><SquareDashed class="size-[18px]" /><kbd
          class="absolute bottom-0.5 right-1 text-[0.5625rem] leading-none text-muted">8</kbd
        ></button
      >
      <button
        type="button"
        aria-label="Add existing task or item (X)"
        title="Add existing · X or 9. Put a task, project, goal or milestone you already have on the map"
        aria-pressed={existingOpen}
        onclick={() => {
          tidyOpen = false
          existingOpen = !existingOpen
        }}
        class="relative grid size-9 shrink-0 place-items-center rounded-xl hover:bg-hover {existingOpen
          ? 'bg-accent-soft text-accent'
          : 'text-icon'}"
        ><ListPlus class="size-[18px]" /><kbd
          class="absolute bottom-0.5 right-1 text-[0.5625rem] leading-none text-muted">9</kbd
        ></button
      >
      <button
        type="button"
        aria-label="Tidy up"
        title="Tidy up. Arrange everything neatly in a direction you choose"
        aria-pressed={tidyOpen}
        onclick={() => {
          existingOpen = false
          tidyOpen = !tidyOpen
        }}
        class="grid size-9 shrink-0 place-items-center rounded-xl hover:bg-hover {tidyOpen
          ? 'bg-accent-soft text-accent'
          : 'text-icon'}"><Sparkles class="size-[18px]" /></button
      >
      <span class="mx-0.5 h-6 w-px shrink-0 bg-line" aria-hidden="true"></span>
      <button
        type="button"
        aria-label="Undo move (Ctrl+Z)"
        title="Undo the last move · Ctrl+Z"
        disabled={!undo.length}
        onclick={() => void doUndo()}
        class="grid size-9 shrink-0 place-items-center rounded-xl text-icon hover:bg-hover disabled:opacity-40"
        ><Undo2 class="size-[18px]" /></button
      >
      <button
        type="button"
        aria-label="Zoom out"
        title="Zoom out · Ctrl+scroll"
        onclick={() => zoom(1 / 1.2)}
        class="grid size-9 shrink-0 place-items-center rounded-xl text-icon hover:bg-hover"
        ><Minus class="size-[18px]" /></button
      ><span class="hidden w-10 shrink-0 text-center text-xs tabular-nums sm:inline">{Math.round(view.z * 100)}%</span
      ><button
        type="button"
        aria-label="Zoom in"
        title="Zoom in · Ctrl+scroll"
        onclick={() => zoom(1.2)}
        class="grid size-9 shrink-0 place-items-center rounded-xl text-icon hover:bg-hover"
        ><Plus class="size-[18px]" /></button
      ><button
        type="button"
        aria-label="Fit to screen"
        title="Fit everything in view"
        onclick={fit}
        class="grid size-9 shrink-0 place-items-center rounded-xl text-icon hover:bg-hover"
        ><Maximize class="size-[18px]" /></button
      >
      <button
        type="button"
        aria-label="Snap to grid"
        title="Snap to grid. Items jump to a grid so they line up"
        aria-pressed={snap}
        onclick={() => (snap = !snap)}
        class="grid size-9 shrink-0 place-items-center rounded-xl hover:bg-hover {snap
          ? 'bg-accent-soft text-accent'
          : 'text-icon'}"><Grid3x3 class="size-[18px]" /></button
      ><button
        type="button"
        aria-label={expanded ? 'Exit full canvas (Esc)' : 'Expand canvas to full screen'}
        title={expanded ? 'Exit full canvas · Esc' : 'Expand to full screen'}
        onclick={toggleExpand}
        class="grid size-9 shrink-0 place-items-center rounded-xl text-icon hover:bg-hover"
        >{#if expanded}<Minimize2 class="size-[18px]" />{:else}<Maximize2 class="size-[18px]" />{/if}</button
      >
      <HelpTip label="the tool dock" text={DOCK_HELP} side="up" align="end" />
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
            Band<HelpTip label="bands" align="end" text={BAND_HELP} />
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
          {#if focusedNode.band_id}
            {@const band = bands.find((x) => x.id === focusedNode!.band_id)}
            {#if band}
              <div class="mt-1 grid grid-cols-2 gap-1">
                <input
                  aria-label="Band name"
                  value={band.name}
                  class="col-span-2 h-8 rounded-md border border-line bg-bg px-2 text-sm"
                  onblur={async (e) => {
                    const name = e.currentTarget.value.trim()
                    if (name && name !== band.name) {
                      await updatePhase(band.id, { name })
                      await load()
                      onchange()
                    }
                  }}
                  onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
                />
                {#each [['start', 'Starts'], ['end', 'Ends']] as [field, text] (field)}
                  <label class="text-[0.6875rem] text-muted"
                    >{text}<input
                      type="date"
                      aria-label="Band {field} date"
                      value={field === 'start' ? (band.start ?? '') : (band.end ?? '')}
                      class="mt-0.5 h-8 w-full rounded-md border border-line bg-bg px-1 text-xs text-text"
                      onchange={async (e) => {
                        await updatePhase(band.id, { [field]: e.currentTarget.value || null })
                        await load()
                        onchange()
                      }}
                    /></label
                  >
                {/each}
              </div>
            {/if}
          {/if}
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
            Convert to<HelpTip label="converting" align="end" text={CONVERT_HELP} />
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
            ><HelpTip label="connecting" align="end" text={CONNECT_HELP} /><Button
              size="sm"
              variant="ghost"
              onclick={() => duplicateNode(focusedNode!)}>Duplicate</Button
            ><Button
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
            Kind<HelpTip label="connection kinds" align="end" text={EDGE_HELP} />
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
    <div data-ui class="absolute bottom-16 right-2 z-10">
      <Button size="sm" variant="ghost" aria-label="Toggle minimap" onclick={() => (showMinimap = !showMinimap)}
        >Map</Button
      >
      {#if showMinimap}<svg
          aria-label="Roadmap minimap"
          class="mt-1 h-24 w-36 rounded-lg border border-line bg-surface p-1"
          viewBox="{minimapBounds.x} {minimapBounds.y} {minimapBounds.width} {minimapBounds.height}"
          >{#each visibleEdges as e (e.id)}{@const a = nodeById.get(e.source_id)}{@const b = nodeById.get(
              e.target_id,
            )}{#if a && b}{@const points = edgeEnds(a, b)}<path
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
            Tip: pick a tool in the dock below, then click the canvas. Double-click anywhere adds a step. The ? at the
            end of the dock explains every tool.
          </p>
        </div>
      </div>{/if}
  </div>
</div>
