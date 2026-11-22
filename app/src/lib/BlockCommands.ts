import { Extension } from '@tiptap/core'
import { PluginKey } from '@tiptap/pm/state'
import { Suggestion, type SuggestionOptions } from '@tiptap/suggestion'
import { createPage } from './sync'
import type { LinkItem } from './Mention'

const rows = [
  ['h1', 'Heading 1'], ['h2', 'Heading 2'], ['h3', 'Heading 3'], ['text', 'Text'],
  ['bullet', 'Bullet list'], ['number', 'Numbered list'], ['todo', 'To-do list'],
  ['quote', 'Quote'], ['code', 'Code block'], ['divider', 'Divider'], ['table', 'Table'],
  ['image', 'Image'], ['page', 'Page'], ['bold', 'Bold'], ['italic', 'Italic'],
  ['strike', 'Strikethrough'], ['inline-code', 'Inline code'], ['section', 'Section'],
] as const

type BlockOptions = {
  render: SuggestionOptions['render']
  command: (id: string) => void
}

export const BlockCommands = Extension.create<BlockOptions>({
  name: 'blockCommands',
  addOptions() { return { render: undefined, command: () => {} } },
  addProseMirrorPlugins() {
    const o = this.options
    const suggestion: SuggestionOptions<LinkItem, LinkItem> = {
      editor: this.editor,
      char: '/',
      allowSpaces: false,
      pluginKey: new PluginKey('orbitBlockCommands'),
      items: ({ query }) => rows
        .filter(([id, label]) => `${id} ${label}`.toLowerCase().includes(query.toLowerCase()))
        .map(([id, label]) => ({ id: `/${id}`, label, kind: 'command' })),
      render: o.render,
      allow: ({ state, range }) => {
        const $from = state.doc.resolve(range.from)
        return $from.parent.isTextblock && $from.parent.textBetween(0, $from.parentOffset) === ''
      },
      command: ({ editor, range, props }) => {
        editor.chain().focus().deleteRange(range).run()
        o.command(props.id.slice(1))
      },
    }
    return [Suggestion(suggestion)]
  },
})

type EditorRef = { chain: () => any; state: { selection: { from: number; to: number; $from: { before: () => number; after: () => number; depth: number; node: (depth: number) => { type: { name: string } } }; }; doc: { textBetween: (a: number, b: number) => string } } }
type Context = { owner: string | null; attach: (files: File[]) => Promise<void>; pageId: string }

export function runBlockCommand(editor: EditorRef, id: string, context: Context) {
  const chain = editor.chain().focus()
  if (id === 'h1' || id === 'h2' || id === 'h3') chain.setNode('heading', { level: Number(id[1]) }).run()
  else if (id === 'text') chain.setNode('paragraph').run()
  else if (id === 'bullet') chain.toggleBulletList().run()
  else if (id === 'number') chain.toggleOrderedList().run()
  else if (id === 'todo') chain.toggleTaskList().run()
  else if (id === 'quote') chain.toggleBlockquote().run()
  else if (id === 'code') chain.toggleCodeBlock().run()
  else if (id === 'divider') chain.setHorizontalRule().run()
  else if (id === 'table') chain.insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()
  else if (id === 'bold') chain.toggleBold().run()
  else if (id === 'italic') chain.toggleItalic().run()
  else if (id === 'strike') chain.toggleStrike().run()
  else if (id === 'inline-code') chain.toggleCode().run()
  else if (id === 'section') {
    const { $from } = editor.state.selection
    let sectionDepth = 0
    for (let depth = 0; depth <= $from.depth; depth++) if ($from.node(depth).type.name === 'section') sectionDepth++
    if (sectionDepth >= 2) return
    const pos = $from.before()
    const end = $from.after()
    const text = editor.state.doc.textBetween(pos + 1, end - 1)
    chain.deleteRange({ from: pos, to: end }).insertContentAt(pos, { type: 'section', attrs: { id: crypto.randomUUID() }, content: [
      { type: 'heading', attrs: { level: 2 }, content: text ? [{ type: 'text', text }] : [] },
    ] }).run()
  } else if (id === 'image') {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = 'image/*'
    input.onchange = () => { if (input.files?.length) void context.attach([...input.files]) }
    input.click()
  } else if (id === 'page' && context.owner) {
    const title = prompt('Page title')?.trim()
    if (title) void createPage(context.owner, title).then((page) => editor.chain().focus().insertContent([
      { type: 'orbitLink', attrs: { id: page, label: title, mentionSuggestionChar: '[[' } },
      { type: 'text', text: ' ' },
    ]).run())
  }
}
