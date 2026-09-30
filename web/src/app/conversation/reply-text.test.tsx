// @vitest-environment happy-dom
//
// Each block of a reply carries its own direction; code stays left to right.
// happy-dom lays out no bidi text, so this reads the attributes that do.

import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it } from 'vitest'
import { ReplyText } from './reply-text'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

const REPLY = [
  'شكراً.',
  '',
  'Thanks.',
  '',
  '## عنوان',
  '',
  '- أولاً `cargo test` ثم',
  '- second',
  '',
  '> اقتباس',
  '',
  '| a | ب |',
  '|---|---|',
  '| x | ي |',
  '',
  '```rust',
  'let x = 1;',
  '```',
].join('\n')

it('gives every block its own direction, and code left to right', async () => {
  const container = document.createElement('div')
  document.body.append(container)
  const root = createRoot(container)
  await act(async () => root.render(<ReplyText text={REPLY} />))

  for (const tag of ['p', 'li', 'h2', 'td', 'th']) {
    const blocks = [...container.querySelectorAll(tag)]
    expect(blocks.length, tag).toBeGreaterThan(0)
    for (const block of blocks) expect(block.getAttribute('dir'), tag).toBe('auto')
  }
  expect(container.querySelector('p')?.textContent).toBe('شكراً.')
  // A block of blocks takes the direction of its first letter outside code.
  expect(container.querySelector('ul')?.getAttribute('dir')).toBe('rtl')
  expect(container.querySelector('blockquote')?.getAttribute('dir')).toBe('rtl')
  expect(container.querySelector('li code')?.getAttribute('dir')).toBe('ltr')
  // index.css gives these their rules: the reply's class, and code left alone.
  expect(container.querySelector('.reply-bidi')).not.toBeNull()
  expect(container.querySelector('pre')?.getAttribute('dir')).toBeNull()

  act(() => root.unmount())
  container.remove()
})
