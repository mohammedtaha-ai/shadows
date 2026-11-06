// @vitest-environment happy-dom
//
// The message box takes its direction from its text: Arabic runs right to
// left, English left to right. A caller cannot turn that off.

import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { describe, expect, it } from 'vitest'
import { ComposerInput } from './composer-surface'

describe('ComposerInput', () => {
  it('the textarea takes its direction from its text and aligns to it', () => {
    const host = document.createElement('div')
    document.body.append(host)
    const root = createRoot(host)
    act(() => root.render(
      <ComposerInput input={{ 'aria-label': 'Message', dir: 'ltr' }} action={null} />,
    ))
    const box = host.querySelector('textarea')
    expect(box?.getAttribute('dir')).toBe('auto')
    expect(box?.classList.contains('text-start')).toBe(true)
    act(() => root.unmount())
    host.remove()
  })
})
