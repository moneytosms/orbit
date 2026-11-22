// @vitest-environment happy-dom
import { Editor, type JSONContent } from '@tiptap/core'
import { Markdown } from '@tiptap/markdown'
import { StarterKit } from '@tiptap/starter-kit'
import { Table, TableCell, TableHeader, TableRow } from '@tiptap/extension-table'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import Paragraph from '@tiptap/extension-paragraph'
import { describe, expect, it } from 'vitest'
import { BlobRef, OrbitLink } from './Mention'
import { Section } from './Section'

const roundTrip = (md: string) => {
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: [StarterKit.configure({ paragraph: false }), Paragraph.extend({ draggable: true }), Table, TableRow, TableHeader, TableCell, TaskList, TaskItem, Section, BlobRef, Markdown.configure({ markedOptions: { gfm: true } }), OrbitLink],
    content: md,
    contentType: 'markdown',
  })
  const out = editor.getMarkdown()
  const json = editor.getJSON()
  const links = editor.getJSON().content?.[0]?.content?.filter((n) => n.type === 'orbitLink') ?? []
  editor.destroy()
  return { out, links, json }
}

describe('OrbitLink', () => {
  it('round-trips an orbit link and a literal [[', () => {
    const md = 'a [Foo](orbit:p1) b \\[[lit]]'
    const { out, links } = roundTrip(md)
    expect(out).toBe(md)
    expect(links).toHaveLength(1)
  })

  it('keeps a real uuid link as a node', () => {
    const md = 'see [Plan](orbit:0190aaaa-0000-7000-8000-000000000001)'
    const { out, links } = roundTrip(md)
    expect(out).toBe(md)
    expect((links[0] as { attrs: object }).attrs).toMatchObject({
      id: '0190aaaa-0000-7000-8000-000000000001',
      label: 'Plan',
    })
  })

  it('leaves ordinary links to the Link mark', () => {
    const md = 'a [site](https://example.com) b'
    const { out, links } = roundTrip(md)
    expect(out).toBe(md)
    expect(links).toHaveLength(0)
  })

  it('escapes brackets in a title', () => {
    const md = '[a \\[b\\]](orbit:p1)'
    expect(roundTrip(md).out).toBe(md)
  })
})

describe('page blocks', () => {
  it.each([
    ['# Heading', '# Heading'],
    ['## Heading', '## Heading'],
    ['### Heading', '### Heading'],
    ['Plain text', 'Plain text'],
    ['- One\n- Two', '- One\n- Two'],
    ['1. One\n2. Two', '1. One\n2. Two'],
    ['> Quote', '> Quote'],
    ['```\ncode\n```', '```\ncode\n```'],
    ['---', '---'],
    ['**bold**', '**bold**'],
    ['*italic*', '*italic*'],
    ['~~strike~~', '~~strike~~'],
    ['`inline`', '`inline`'],
    ['![photo](blob:hash)', '![photo](blob:hash)'],
  ])('round-trips %s', (md, expected) => {
    expect(roundTrip(md).out).toBe(expected)
  })

  it('round-trips a GFM table', () => {
    const md = '| A | B |\n| --- | --- |\n| one | two |'
    const canonical = '\n| A   | B   |\n| --- | --- |\n| one | two |\n'
    expect(roundTrip(md).out).toBe(canonical)
    expect(roundTrip(canonical).out).toBe(canonical)
  })

  it('round-trips a task list', () => {
    const md = '- [ ] Draft\n- [x] Ship'
    expect(roundTrip(md).out).toBe(md)
  })

  it('round-trips a section directive with a stable ID', () => {
    const md = '::: section sec_abc-123\n## Plan\n\nWrite it\n:::'
    expect(roundTrip(md).out).toBe(md)
  })

  it('lifts sections beyond one nested level after markdown reload without changing IDs', () => {
    const md = [
      '::: section root', '## Root', '::: section child', '### Child',
      '::: section deep', '#### Deep', ':::', ':::', ':::',
    ].join('\n')
    const { out, json } = roundTrip(md)
    const maxDepth = (nodes: JSONContent[], depth = 0): number =>
      Math.max(0, ...nodes.map((node) => Math.max(
        node.type === 'section' ? depth + 1 : depth,
        node.content ? maxDepth(node.content, node.type === 'section' ? depth + 1 : depth) : 0,
      )))
    expect(maxDepth(json.content ?? [])).toBeLessThanOrEqual(2)
    expect(out).toContain('::: section deep')
    expect(out).toContain('#### Deep')
    expect(roundTrip(out).out).toBe(out)
  })

  it('enforces section depth after pasted HTML', () => {
    const editor = new Editor({
      element: document.createElement('div'),
      extensions: [StarterKit.configure({ paragraph: false }), Paragraph.extend({ draggable: true }), Table, TableRow, TableHeader, TableCell, TaskList, TaskItem, Section, Markdown.configure({ markedOptions: { gfm: true } }), OrbitLink],
      content: 'Start',
      contentType: 'markdown',
    })
    const pasted = '<div data-orbit-section data-section-id="root"><h2>Root</h2><div data-orbit-section data-section-id="child"><h3>Child</h3><div data-orbit-section data-section-id="deep"><h4>Deep</h4></div></div></div>'
    editor.commands.insertContentAt(0, pasted, { contentType: 'html' })
    const sectionDepth = (nodes: JSONContent[], depth = 0): number =>
      Math.max(0, ...nodes.map((node) => Math.max(
        node.type === 'section' ? depth + 1 : depth,
        node.content ? sectionDepth(node.content, node.type === 'section' ? depth + 1 : depth) : 0,
      )))
    expect(sectionDepth(editor.getJSON().content ?? [])).toBeLessThanOrEqual(2)
    expect(JSON.stringify(editor.getJSON())).toContain('"id":"deep"')
    expect(editor.getMarkdown()).toContain('::: section deep')
    editor.destroy()
  })

  it('persists fold view state locally without adding it to Markdown', () => {
    const md = '::: section sec_fold\n## Fold me\n:::'
    const element = document.createElement('div')
    localStorage.removeItem('orbit.page.page-a.sections')
    const extensions = [StarterKit.configure({ paragraph: false }), Paragraph.extend({ draggable: true }), Table, TableRow, TableHeader, TableCell, TaskList, TaskItem, Section.configure({ pageId: 'page-a' }), BlobRef, Markdown.configure({ markedOptions: { gfm: true } }), OrbitLink]
    const first = new Editor({ element, extensions, content: md, contentType: 'markdown' })
    element.querySelector<HTMLButtonElement>('.orbit-section-toggle')!.click()
    expect(localStorage.getItem('orbit.page.page-a.sections')).toContain('sec_fold')
    expect(first.getMarkdown()).toBe(md)
    first.destroy()

    const reloaded = new Editor({ element, extensions, content: md, contentType: 'markdown' })
    expect(element.querySelector('.orbit-section-toggle')?.getAttribute('aria-label')).toBe('Expand section')
    expect(reloaded.getMarkdown()).toBe(md)
    reloaded.destroy()
    localStorage.removeItem('orbit.page.page-a.sections')
  })
})
