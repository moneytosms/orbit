import { Mention } from '@tiptap/extension-mention'
import { PluginKey } from '@tiptap/pm/state'
import { Suggestion, type SuggestionOptions } from '@tiptap/suggestion'

export type LinkState = 'ok' | 'trashed' | 'archived' | 'purged'
/** What a link target is now; `title` is its live title. */
export type LinkTarget = { title: string; kind: string; state: LinkState; href: string | null }
/** One suggestion row. `create` means "Create Page" with `label` as the new title. */
export type LinkItem = { id: string; label: string; kind: string; create?: boolean }

export type OrbitLinkOptions = {
  /** Candidates for a trigger (`[[` or `@`) and the text typed after it. */
  items: (char: string, query: string) => Promise<LinkItem[]>
  /** Creates a Page from a "Create Page" row and returns its id. */
  create: (title: string) => Promise<string>
  /** The target's live state; null while unknown. */
  resolve: (id: string) => Promise<LinkTarget | null>
  restore: (id: string) => Promise<void>
  open: (href: string) => void
  /** The suggestion popup; the Svelte editor supplies it. */
  render: SuggestionOptions['render']
}

const escapeLabel = (s: string) => s.replace(/[\\[\]]/g, '\\$&')

const idOf = (href: string) => (href.startsWith('orbit:') ? href.slice('orbit:'.length) : '')

const NOOP: OrbitLinkOptions = {
  items: async () => [],
  create: async () => '',
  resolve: async () => null,
  restore: async () => {},
  open: () => {},
  render: undefined,
}

/** Draws a link chip from its live state. */
function paint(dom: HTMLElement, id: string, stored: string, t: LinkTarget | null, o: OrbitLinkOptions) {
  dom.replaceChildren()
  dom.className = 'orbit-link'
  dom.removeAttribute('role')
  dom.removeAttribute('tabindex')
  dom.dataset.state = t ? t.state : 'purged'
  if (!t || t.state === 'purged') {
    dom.className = 'orbit-link orbit-link-purged'
    dom.textContent = stored
    return
  }
  const text = document.createElement('span')
  text.textContent = t.title
  if (t.state === 'trashed') {
    text.className = 'orbit-link-struck'
    dom.append(text)
    const restore = document.createElement('button')
    restore.type = 'button'
    restore.textContent = 'Restore'
    restore.className = 'orbit-link-restore'
    restore.onclick = async (e) => {
      e.preventDefault()
      e.stopPropagation()
      await o.restore(id)
      paint(dom, id, stored, await o.resolve(id), o)
    }
    dom.append(restore)
    return
  }
  if (t.state === 'archived') {
    const mark = document.createElement('span')
    mark.className = 'orbit-link-archive'
    mark.textContent = '▤ '
    mark.title = 'Archived'
    dom.append(mark)
  }
  dom.append(text)
  if (t.href) {
    const href = t.href
    dom.setAttribute('role', 'link')
    dom.setAttribute('tabindex', '0')
    dom.onclick = () => o.open(href)
    dom.onkeydown = (e) => {
      if (e.key === 'Enter') o.open(href)
    }
  }
}

/**
 * The one link node. `[[` links Pages and `@` any other item; both are stored as
 * `[Title](orbit:<uuid>)`. The title is a snapshot: the chip shows the live one.
 */
export const OrbitLink = Mention.extend<OrbitLinkOptions>({
  name: 'orbitLink',
  priority: 1100,
  markdownTokenName: 'link',
  markdownTokenizer: undefined,
  addOptions() {
    return { ...NOOP } as never
  },
  onBeforeCreate() {
    // The Markdown extension assigns its manager after this runs (our priority is higher), so patch on assignment.
    // The stock escaping turns a typed `[[` into `\[\[`; one backslash is enough to keep it from linking.
    let manager: object | undefined
    Object.defineProperty(this.editor, 'markdown', {
      configurable: true,
      get: () => manager,
      set: (m: { escapeMarkdownSyntax: (text: string) => string }) => {
        m.escapeMarkdownSyntax = (text) => text.replace(/([\\`*_~])|(?<!\[)\[|\](?=\()/g, (m, c) => `\\${c ?? m}`)
        manager = m
      },
    })
  },
  parseHTML() {
    return [{ tag: 'span[data-type="orbitLink"]' }]
  },
  parseMarkdown(token, helpers) {
    const id = idOf(String(token.href ?? ''))
    // The first handler for a token wins, so ordinary links are passed on to the Link mark here.
    if (!id)
      return helpers.applyMark('link', helpers.parseInline(token.tokens ?? []), {
        href: token.href,
        title: token.title || null,
      })
    const label = String(token.text ?? '').replace(/\\([\\[\]])/g, '$1')
    return helpers.createNode('orbitLink', { id, label, mentionSuggestionChar: '@' })
  },
  renderMarkdown(node) {
    return `[${escapeLabel(String(node.attrs?.label ?? ''))}](orbit:${node.attrs?.id})`
  },
  addNodeView() {
    const o = this.options
    return ({ node }) => {
      const dom = document.createElement('span')
      const id = String(node.attrs.id)
      const stored = String(node.attrs.label ?? '')
      dom.className = 'orbit-link'
      dom.dataset.state = 'pending'
      dom.textContent = stored
      o.resolve(id).then((t) => paint(dom, id, stored, t, o))
      return { dom }
    }
  },
  addProseMirrorPlugins() {
    const o = this.options
    const name = this.name
    const make = (char: string, allowSpaces: boolean): SuggestionOptions<LinkItem, LinkItem> => ({
      editor: this.editor,
      char,
      allowSpaces,
      pluginKey: new PluginKey(`orbitLink${char}`),
      items: ({ query }) => o.items(char, query),
      render: o.render,
      command: ({ editor, range, props }) => {
        const insert = (id: string, label: string) =>
          editor
            .chain()
            .focus()
            .insertContentAt(range, [
              { type: name, attrs: { id, label, mentionSuggestionChar: char } },
              { type: 'text', text: ' ' },
            ])
            .run()
        if (props.create) o.create(props.label).then((id) => id && insert(id, props.label))
        else insert(props.id, props.label)
      },
      allow: ({ state, range }) => {
        const type = state.schema.nodes[name]
        return !!state.doc.resolve(range.from).parent.type.contentMatch.matchType(type)
      },
    })
    return [Suggestion(make('[[', true)), Suggestion(make('@', false))]
  },
})
