// @vitest-environment happy-dom

import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import { planFixture } from '@/test/contract-fixtures'
import { PlanGraph } from './plan-graph'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

const observed = vi.hoisted(() => ({
  props: null as { minZoom?: number; fitViewOptions?: { minZoom?: number } } | null,
}))

vi.mock('@xyflow/react', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@xyflow/react')>()
  const { createElement } = await import('react')
  return {
    ...actual,
    ReactFlow: (props: React.ComponentProps<typeof actual.ReactFlow>) => {
      observed.props = props
      return createElement(actual.ReactFlow, props)
    },
  }
})

let cleanup: (() => void) | undefined
afterEach(() => cleanup?.())

it('passes a readable minimum zoom to React Flow', async () => {
  const container = document.createElement('div')
  document.body.append(container)
  const root = createRoot(container)
  cleanup = () => {
    act(() => root.unmount())
    container.remove()
  }

  await act(async () => root.render(<PlanGraph plan={planFixture()} />))

  expect(observed.props?.minZoom).toBeGreaterThanOrEqual(1)
  expect(observed.props?.fitViewOptions?.minZoom).toBeGreaterThanOrEqual(1)
})
