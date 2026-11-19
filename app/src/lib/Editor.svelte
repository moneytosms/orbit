<script lang="ts">
  import { onDestroy, untrack } from 'svelte'
  import { StarterKit } from '@tiptap/starter-kit'
  import { Markdown } from '@tiptap/markdown'
  import { createEditor, EditorContent } from 'svelte-tiptap'

  let { value, onchange, label }: { value: string; onchange: (markdown: string) => void; label: string } = $props()

  // The editor owns the text once mounted: `value` is only the starting markdown.
  let saved = untrack(() => value)
  let timer: ReturnType<typeof setTimeout> | undefined

  const editor = createEditor({
    extensions: [StarterKit, Markdown],
    content: saved,
    contentType: 'markdown',
    editorProps: {
      attributes: {
        'aria-label': untrack(() => label),
        'aria-multiline': 'true',
        role: 'textbox',
        class: 'editor-body',
      },
    },
    onUpdate: () => {
      clearTimeout(timer)
      timer = setTimeout(flush, 500)
    },
    onBlur: flush,
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

<div class="editor rounded-lg border border-line bg-surface px-3 py-2 focus-within:border-accent-line">
  <EditorContent editor={$editor} />
</div>

<style>
  .editor :global(.editor-body) {
    min-height: 6rem;
    outline: none;
    font-size: 15px;
    line-height: 1.55;
  }
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
  .editor :global(hr) {
    border-top: 1px solid var(--color-line, currentColor);
  }
</style>
