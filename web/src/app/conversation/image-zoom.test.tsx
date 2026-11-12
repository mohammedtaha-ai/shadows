// @vitest-environment happy-dom
//
// The viewer renders in a portal, so the dialog is read from document.body.

import { type Root, createRoot } from 'react-dom/client'
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { until } from '@/test/test-app'
import { ImageZoom } from './image-zoom'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

let host: HTMLElement | null = null
let root: Root | null = null
afterEach(() => {
  act(() => root?.unmount())
  host?.remove()
  host = null
  root = null
})

async function show() {
  host = document.createElement('div')
  document.body.append(host)
  root = createRoot(host)
  await act(async () => root!.render(<ImageZoom alt="a.png" src="http://daemon/a.png" />))
  return document.body.querySelector<HTMLButtonElement>('button[aria-label="View a.png"]')!
}

const dialog = () => document.body.querySelector('[role="dialog"]')

it('the thumbnail opens the whole image in a named dialog', async () => {
  const thumbnail = await show()
  act(() => thumbnail.click())
  await until(() => dialog() !== null)
  expect(dialog()?.getAttribute('aria-labelledby')).not.toBeNull()
  expect(dialog()?.textContent).toContain('a.png')
  const large = dialog()!.querySelector('img[alt="a.png"][crossorigin="anonymous"]')
  expect(large?.getAttribute('src')).toBe('http://daemon/a.png')
})

it.each(['Esc', 'the close button'] as const)('%s closes the viewer and focus returns to the thumbnail', async how => {
  const thumbnail = await show()
  act(() => thumbnail.click())
  await until(() => dialog() !== null)
  if (how === 'Esc') act(() => { document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })) })
  else act(() => [...dialog()!.querySelectorAll('button')].find(b => b.textContent === 'Close')!.click())
  await until(() => dialog() === null)
  await until(() => document.activeElement === thumbnail)
})
