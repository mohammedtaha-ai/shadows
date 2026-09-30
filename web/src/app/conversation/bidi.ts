// One job: giving each block of a rendered reply its own text direction, so
// an Arabic paragraph reads right to left beside an English one.

/** Blocks that hold text of their own: `dir="auto"` gives each the direction
 * of its first strong character, and with it the side its text starts on, its
 * list marker's side and its full stop's end. */
const TEXT_BLOCKS = new Set([
  'p',
  'li',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'td',
  'th',
  'dt',
  'dd',
])

/** Blocks that hold other blocks. `dir="auto"` would not do for them: it skips
 * every child that has a `dir` of its own, which is all of them, and falls
 * back to left to right. So their direction is decided here, from the first
 * strong character of their text, and sets the side of a quote's bar and a
 * nested list's indent. */
const CONTAINERS = new Set(['blockquote', 'ul', 'ol'])

const LETTER = /\p{L}/u
const RIGHT_TO_LEFT =
  /[\p{Script=Arabic}\p{Script=Hebrew}\p{Script=Syriac}\p{Script=Thaana}\p{Script=Nko}]/u

interface HastNode {
  type: string
  tagName?: string
  value?: string
  properties?: Record<string, unknown>
  children?: HastNode[]
}

/** The direction of the first letter outside code, or `null` with none. */
function firstStrong(node: HastNode): 'ltr' | 'rtl' | null {
  if (node.type === 'text') {
    const letter = LETTER.exec(node.value ?? '')?.[0]
    if (letter === undefined) return null
    return RIGHT_TO_LEFT.test(letter) ? 'rtl' : 'ltr'
  }
  if (node.tagName === 'code' || node.tagName === 'pre') return null
  for (const child of node.children ?? []) {
    const found = firstStrong(child)
    if (found !== null) return found
  }
  return null
}

function mark(node: HastNode): void {
  if (node.type === 'element' && node.tagName !== undefined) {
    // A fenced code block stays left to right whatever its text (index.css).
    if (node.tagName === 'pre') return
    // An inline code span is a left-to-right island that a block's
    // `dir="auto"` skips, so `cargo` never decides an Arabic sentence.
    if (node.tagName === 'code') {
      node.properties = { ...node.properties, dir: 'ltr' }
      return
    }
    if (TEXT_BLOCKS.has(node.tagName)) {
      node.properties = { ...node.properties, dir: 'auto' }
    } else if (CONTAINERS.has(node.tagName)) {
      node.properties = { ...node.properties, dir: firstStrong(node) ?? 'ltr' }
    }
  }
  node.children?.forEach(mark)
}

/** A rehype plugin: runs after Streamdown's own, on the tree it renders. */
export function rehypeDirAuto() {
  return (tree: HastNode) => mark(tree)
}
