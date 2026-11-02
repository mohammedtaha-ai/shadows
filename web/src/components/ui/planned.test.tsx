// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, expect, it } from 'vitest'
import { Planned } from './planned'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true
let root: Root | null = null
let host: HTMLElement | null = null
afterEach(() => { act(() => root?.unmount()); host?.remove(); root = null; host = null })
function show(node: React.ReactNode) {
  host = document.createElement('div')
  document.body.append(host)
  root = createRoot(host)
  act(() => root!.render(node))
  return host
}

it('shows the badge and the reason inside a named region', () => {
  const dom = show(<Planned title="Memory" reason="Waits for the memory request.">
    <button type="button">Clear</button>
  </Planned>)
  const region = dom.querySelector('section[aria-label="Memory (planned)"]')
  expect(region).not.toBeNull()
  expect(region!.textContent).toContain('Planned')
  expect(region!.textContent).toContain('Waits for the memory request.')
})

it('keeps its children out of focus and clicks', () => {
  const dom = show(<Planned title="Memory"><button type="button">Clear</button></Planned>)
  const live = dom.querySelector('[inert]')
  expect(live).not.toBeNull()
  expect(live!.getAttribute('aria-hidden')).toBe('true')
  expect(live!.querySelector('button')).not.toBeNull()
})
