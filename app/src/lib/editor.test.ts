// @vitest-environment happy-dom
import { Editor } from '@tiptap/core'
import { Markdown } from '@tiptap/markdown'
import { StarterKit } from '@tiptap/starter-kit'
import { describe, expect, it } from 'vitest'
import { OrbitLink } from './Mention'

const roundTrip = (md: string) => {
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: [StarterKit, Markdown, OrbitLink],
    content: md,
    contentType: 'markdown',
  })
  const out = editor.getMarkdown()
  const links = editor.getJSON().content?.[0]?.content?.filter((n) => n.type === 'orbitLink') ?? []
  editor.destroy()
  return { out, links }
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
