import { Node } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import type { Node as PMNode } from '@tiptap/pm/model'
import type { Transaction } from '@tiptap/pm/state'

type Options = { pageId: string }
const stateKey = (pageId: string) => `orbit.page.${pageId}.sections`

function boundSections(nodes: any[], depth: number): { nodes: any[]; lifted: any[] } {
  const result: any[] = []
  const lifted: any[] = []
  for (const node of nodes) {
    if (node.type === 'section') {
      const overDepth = depth >= 2
      const children = boundSections(node.content ?? [], overDepth ? depth : depth + 1)
      const section = { ...node, content: children.nodes }
      if (overDepth) lifted.push(section, ...children.lifted)
      else result.push(section, ...children.lifted)
    } else if (node.content) {
      const children = boundSections(node.content, depth)
      result.push({ ...node, content: [...children.nodes, ...children.lifted] })
    } else result.push(node)
  }
  return { nodes: result, lifted }
}

function folded(pageId: string, id: string) {
  try {
    const saved = JSON.parse(localStorage.getItem(stateKey(pageId)) ?? '{}')
    return saved && saved[id] === true
  } catch { return false }
}

function saveFolded(pageId: string, id: string, value: boolean) {
  try {
    const saved = JSON.parse(localStorage.getItem(stateKey(pageId)) ?? '{}')
    localStorage.setItem(stateKey(pageId), JSON.stringify({ ...saved, [id]: value }))
  } catch { /* Storage may be unavailable. */ }
}

function normalizeNesting(doc: PMNode, tr: Transaction) {
  const lift: number[] = []
  doc.descendants((node, pos) => {
    if (node.type.name !== 'section') return
    const $pos = doc.resolve(pos)
    let depth = 0
    for (let i = 1; i <= $pos.depth; i++) if ($pos.node(i).type.name === 'section') depth++
    if (depth >= 2) lift.push(pos)
  })
  for (const pos of lift.sort((a, b) => b - a)) {
    const mappedPos = tr.mapping.map(pos, 1)
    const node = tr.doc.nodeAt(mappedPos)
    if (!node || node.type.name !== 'section') continue
    const $pos = tr.doc.resolve(mappedPos)
    let rootDepth = 1
    while (rootDepth <= $pos.depth && $pos.node(rootDepth).type.name !== 'section') rootDepth++
    const rootPos = $pos.before(rootDepth)
    const root = tr.doc.nodeAt(rootPos)
    if (!root) continue
    const rootEnd = rootPos + root.nodeSize
    tr.delete(mappedPos, mappedPos + node.nodeSize)
    tr.insert(tr.mapping.map(rootEnd, -1), node)
  }
  return tr
}

/** A foldable block whose ID is carried in the Markdown directive. */
export const Section = Node.create<Options>({
  name: 'section',
  group: 'block',
  content: 'block+',
  isolating: true,
  draggable: true,
  addOptions() { return { pageId: '' } },
  onCreate() {
    const tr = normalizeNesting(this.editor.state.doc, this.editor.state.tr)
    this.editor.view.dispatch(tr)
  },
  addProseMirrorPlugins() {
    return [new Plugin({
      key: new PluginKey('orbitSectionDepth'),
      appendTransaction: (_transactions, _oldState, state) => {
        const tr = normalizeNesting(state.doc, state.tr)
        return tr.docChanged ? tr : null
      },
    })]
  },
  addAttributes() { return { id: { default: null } } },
  parseHTML() { return [{ tag: 'div[data-orbit-section]', getAttrs: (el) => ({ id: (el as HTMLElement).dataset.sectionId }) }] },
  renderHTML({ node }) { return ['div', { 'data-orbit-section': '', 'data-section-id': node.attrs.id }, 0] },
  markdownTokenizer: {
    name: 'section',
    level: 'block',
    start: (src) => src.indexOf('::: section'),
    tokenize: (src, _tokens, lexer) => {
      const open = /^::: section ([\w-]+)\n/.exec(src)
      if (!open) return undefined
      const lines = src.slice(open[0].length).match(/.*(?:\n|$)/g)?.filter(Boolean) ?? []
      let depth = 1
      let fenced = false
      let offset = open[0].length
      let end = -1
      for (const line of lines) {
        if (/^\s*```/.test(line)) fenced = !fenced
        if (!fenced && /^::: section [\w-]+\s*$/.test(line)) depth++
        else if (!fenced && /^:::\s*$/.test(line) && --depth === 0) { end = offset; break }
        offset += line.length
      }
      if (end < 0) return undefined
      const close = /^:::\s*\n?/.exec(src.slice(end))![0]
      const raw = src.slice(0, end + close.length)
      return { type: 'section', raw, id: open[1], tokens: lexer.blockTokens(src.slice(open[0].length, end)) }
    },
  },
  parseMarkdown(token, helpers) {
    const children = boundSections(helpers.parseChildren(token.tokens ?? []), 1)
    return { type: 'section', attrs: { id: token.id }, content: [...children.nodes, ...children.lifted] }
  },
  renderMarkdown(node, helpers) {
    return `::: section ${node.attrs?.id}\n${helpers.renderChildren(node.content ?? [], '\n\n')}\n:::`
  },
  addNodeView() {
    const pageId = this.options.pageId
    return ({ node }) => {
      const dom = document.createElement('div')
      dom.className = 'orbit-section'
      dom.dataset.orbitSection = ''
      dom.dataset.sectionId = String(node.attrs.id ?? '')
      const toggle = document.createElement('button')
      toggle.className = 'orbit-section-toggle'
      toggle.type = 'button'
      toggle.setAttribute('aria-label', 'Collapse section')
      toggle.contentEditable = 'false'
      toggle.draggable = true
      toggle.dataset.dragHandle = 'true'
      const contentDOM = document.createElement('div')
      contentDOM.className = 'orbit-section-content'
      let isFolded = folded(pageId, String(node.attrs.id ?? ''))
      const update = () => {
        contentDOM.hidden = isFolded
        toggle.textContent = isFolded ? '▸' : '▾'
        toggle.setAttribute('aria-label', isFolded ? 'Expand section' : 'Collapse section')
      }
      toggle.onclick = () => {
        isFolded = !isFolded
        saveFolded(pageId, String(node.attrs.id ?? ''), isFolded)
        update()
      }
      dom.append(toggle, contentDOM)
      update()
      return { dom, contentDOM, update: (next) => {
        if (next.type.name !== 'section') return false
        if (next.attrs.id !== node.attrs.id) { isFolded = folded(pageId, String(next.attrs.id ?? '')); update() }
        return true
      } }
    }
  },
})
