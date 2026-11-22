<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte'
  import { StarterKit } from '@tiptap/starter-kit'
  import { Table, TableCell, TableHeader, TableRow } from '@tiptap/extension-table'
  import TaskList from '@tiptap/extension-task-list'
  import TaskItem from '@tiptap/extension-task-item'
  import Paragraph from '@tiptap/extension-paragraph'
  import { Markdown } from '@tiptap/markdown'
  import { createEditor, EditorContent } from 'svelte-tiptap'
  import type { SuggestionProps } from '@tiptap/suggestion'
  import { mount, unmount } from 'svelte'
  import MentionList from './MentionList.svelte'
  import { BlobRef, OrbitLink, type LinkItem } from './Mention'
  import { BlockCommands, runBlockCommand } from './BlockCommands'
  import { Section } from './Section'
  import { addFile, blobUrl } from './blobs'
  import { createPage, linkTarget, restoreItem, searchLinks } from './sync'

  let {
    value,
    onchange,
    label,
    owner = null,
    editable = true,
    chrome = 'boxed',
    pageId = '',
  }: {
    value: string
    onchange: (markdown: string) => void
    label: string
    /** Where "Create Page" puts a new Page; none (Inbox Tasks) hides it. */
    owner?: string | null
    editable?: boolean
    chrome?: 'boxed' | 'page'
    pageId?: string
  } = $props()

  // The editor owns the text once mounted: `value` is only the starting markdown.
  let saved = untrack(() => value)
  let mounted = $state(false)
  let timer: ReturnType<typeof setTimeout> | undefined

  /** The suggestion popup: a MentionList mounted next to the caret. */
  function popup() {
    let el: HTMLElement | undefined
    let list: ReturnType<typeof mount> | undefined
    const props = $state<{ items: LinkItem[]; command: (item: LinkItem) => void }>({ items: [], command: () => {} })
    const place = (p: SuggestionProps<LinkItem, LinkItem>) => {
      const r = p.clientRect?.()
      if (el && r) {
        const box = el.getBoundingClientRect()
        el.style.left = `${Math.max(8, Math.min(r.left, innerWidth - box.width - 8))}px`
        el.style.top = `${r.bottom + box.height + 8 < innerHeight ? r.bottom + 4 : Math.max(8, r.top - box.height - 4)}px`
      }
    }
    return {
      onStart: (p: SuggestionProps<LinkItem, LinkItem>) => {
        el = document.createElement('div')
        el.style.cssText = 'position:fixed;z-index:60'
        document.body.append(el)
        props.items = p.items
        props.command = p.command
        list = mount(MentionList, { target: el, props })
        place(p)
      },
      onUpdate: (p: SuggestionProps<LinkItem, LinkItem>) => {
        props.items = p.items
        props.command = p.command
        place(p)
      },
      onKeyDown: ({ event }: { event: KeyboardEvent }) => {
        if (event.key === 'Escape') return false
        return (list as unknown as { onKeyDown: (e: KeyboardEvent) => boolean } | undefined)?.onKeyDown(event) ?? false
      },
      onExit: () => {
        if (list) unmount(list)
        el?.remove()
        list = el = undefined
      },
    }
  }

  let problem = $state('')

  /** Stores each file locally and inserts its markdown; oversize files are refused with a message. */
  async function attach(files: File[]) {
    problem = ''
    for (const f of files) {
      try {
        const { hash, markdown } = await addFile(f)
        const image = markdown.startsWith('!')
        const name = f.name || 'file'
        $editor
          .chain()
          .focus()
          .insertContent([
            { type: 'blobRef', attrs: { hash, name, image } },
            { type: 'text', text: ' ' },
          ])
          .run()
      } catch (e) {
        problem = e instanceof Error ? e.message : String(e)
      }
    }
  }

  const editor = createEditor({
    editable: untrack(() => editable),
    extensions: [
      StarterKit.configure({ paragraph: false }),
      Paragraph.extend({ draggable: true }),
      Table.configure({ resizable: true }),
      TableRow,
      TableHeader,
      TableCell,
      TaskList,
      TaskItem.configure({ nested: false }),
      Markdown.configure({ markedOptions: { gfm: true } }),
      BlobRef.configure({ url: blobUrl }),
      OrbitLink.configure({
        items: async (char, query) => {
          const found = await searchLinks(char, query)
          const parent = untrack(() => owner)
          const title = query.trim()
          return char === '[[' && parent && title && !found.some((i) => i.label === title)
            ? [...found, { id: '', label: title, kind: 'page', create: true }]
            : found
        },
        create: (title) =>
          createPage(
            untrack(() => owner)!,
            title,
          ),
        resolve: linkTarget,
        restore: restoreItem,
        open: (href) => (location.hash = href),
        render: popup,
      }),
      BlockCommands.configure({
        render: popup,
        command: (id) => runBlockCommand($editor, id, { owner: untrack(() => owner), attach, pageId: untrack(() => pageId) }),
      }),
      Section.configure({ pageId: untrack(() => pageId) }),
    ],
    content: saved,
    contentType: 'markdown',
    editorProps: {
      attributes: {
        'aria-label': untrack(() => label),
        'aria-multiline': 'true',
        role: 'textbox',
        class: 'editor-body',
      },
      handleDrop: (_view, event) => {
        const files = [...(event.dataTransfer?.files ?? [])]
        if (!files.length) return false
        event.preventDefault()
        void attach(files)
        return true
      },
      handlePaste: (_view, event) => {
        const files = [...(event.clipboardData?.files ?? [])]
        if (!files.length) return false
        event.preventDefault()
        void attach(files)
        return true
      },
    },
    onUpdate: () => {
      clearTimeout(timer)
      timer = setTimeout(flush, 500)
    },
    onBlur: flush,
  })

  onMount(() => {
    mounted = true
    if (chrome === 'page' && editable) $editor.commands.focus('end')
  })

  $effect(() => {
    if (!mounted || chrome !== 'page' || !editable) return
    const markdown = $editor.getMarkdown()
    if (markdown !== saved) {
      saved = markdown
      onchange(markdown)
    }
  })

  /** Reports the markdown if it differs from what was last reported. */
  function flush() {
    clearTimeout(timer)
    timer = undefined
    const md = $editor.getMarkdown()
    if (md === saved) return
    saved = md
    onchange(md)
  }

  onDestroy(() => {
    if (timer !== undefined) flush()
  })
</script>

<div class="editor {chrome === 'page' ? 'page-editor min-h-[60vh]' : 'rounded-lg border border-line bg-surface px-3 py-2 focus-within:border-accent-line'}">
  <EditorContent editor={$editor} />
  {#if problem}<p role="alert" class="mt-1 text-sm text-red-600">{problem}</p>{/if}
</div>

<style>
  .editor :global(.editor-body) {
    min-height: 6rem;
    outline: none;
    font-size: 15px;
    line-height: var(--lh, 1.55);
  }
  .editor.page-editor :global(.editor-body) { cursor: text; }
  .editor.page-editor :global(.editor-body) { min-height: 60vh; }
  .editor :global(.orbit-section) { position: relative; }
  .editor :global(.orbit-section-toggle) { position: absolute; left: -1.5rem; top: .1rem; color: var(--color-muted); }
  .editor :global(.orbit-section-content[hidden]) { display: none; }
  .editor :global(.editor-body > * + *) {
    margin-top: 0.5rem;
  }
  .editor :global(h1) {
    font-size: 1.4em;
    font-weight: 700;
  }
  .editor :global(h2) {
    font-size: 1.2em;
    font-weight: 700;
  }
  .editor :global(h3) {
    font-size: 1.05em;
    font-weight: 700;
  }
  .editor :global(ul) {
    list-style: disc;
    padding-left: 1.4rem;
  }
  .editor :global(ul[data-type='taskList']) { list-style: none; padding-left: 0; }
  .editor :global(ul[data-type='taskList'] li) { display: flex; align-items: baseline; gap: .45rem; }
  .editor :global(table) { width: 100%; border-collapse: collapse; }
  .editor :global(th), .editor :global(td) { border: 1px solid var(--color-line); padding: .35rem .5rem; text-align: left; }
  .editor :global(ol) {
    list-style: decimal;
    padding-left: 1.4rem;
  }
  .editor :global(blockquote) {
    border-left: 3px solid var(--color-line, currentColor);
    padding-left: 0.75rem;
    opacity: 0.85;
  }
  .editor :global(code) {
    border-radius: 4px;
    background: var(--color-sunk, rgb(0 0 0 / 0.06));
    padding: 0 0.25rem;
  }
  .editor :global(pre) {
    border-radius: 6px;
    background: var(--color-sunk, rgb(0 0 0 / 0.06));
    padding: 0.5rem 0.75rem;
    overflow-x: auto;
  }
  .editor :global(a) {
    text-decoration: underline;
  }
  .editor :global(.orbit-link) {
    border-radius: 4px;
    background: var(--color-sunk, rgb(0 0 0 / 0.06));
    padding: 0 0.3rem;
    cursor: pointer;
  }
  .editor :global(.orbit-link-purged) {
    background: none;
    padding: 0;
    cursor: text;
    opacity: 0.6;
  }
  .editor :global(.orbit-link-struck) {
    text-decoration: line-through;
    opacity: 0.7;
  }
  .editor :global(.orbit-link-restore) {
    margin-left: 0.4rem;
    text-decoration: underline;
    font-size: 0.85em;
  }
  .editor :global(.orbit-blob-image) {
    display: block;
    max-width: 100%;
    border-radius: 6px;
  }
  .editor :global(hr) {
    border-top: 1px solid var(--color-line, currentColor);
  }
</style>
