/** One pasted line ready for quick-add: its text and how deep it is nested under the line above. */
export type PasteLine = { text: string; depth: number }

const MARKER = /^(?:[-*+•](?:\s+|$)(?:\[[ xX]\](?:\s+|$))?|\d+[.)](?:\s+|$)|\[[ xX]\](?:\s+|$))/
const MAX_DEPTH = 2

const indentOf = (s: string) => {
  let n = 0
  for (const c of s) {
    if (c === ' ') n += 1
    else if (c === '\t') n += 2
    else break
  }
  return n
}

/**
 * Splits pasted text into quick-add lines. Blank lines drop; a leading list marker (`- `, `* `, `1. `, `[ ] `) is
 * stripped; deeper indentation than the line above nests one level (capped at 2 levels of subtasks).
 */
export function pasteLines(text: string): PasteLine[] {
  const out: PasteLine[] = []
  const stack: number[] = []
  for (const raw of text.split(/\r?\n/)) {
    if (!raw.trim()) continue
    const indent = indentOf(raw)
    const line = raw.trim().replace(MARKER, '').trim()
    if (!line) continue
    while (stack.length && stack[stack.length - 1] >= indent) stack.pop()
    const depth = Math.min(stack.length, MAX_DEPTH)
    if (stack.length <= MAX_DEPTH) stack.push(indent)
    out.push({ text: line, depth })
  }
  return out
}
