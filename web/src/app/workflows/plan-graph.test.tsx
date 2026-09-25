// @vitest-environment happy-dom
//
// PlanGraph at its React Flow boundary: the first view is readable, zoom is
// never locked, and a canvas that changes size is refitted, not remounted.

import type { FitViewOptions } from '@xyflow/react'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { planFixture } from '@/test/contract-fixtures'
import { FakeResizeObserver, resize } from '@/test/fake-resize-observer'
import { PlanGraph } from './plan-graph'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

const observed = vi.hoisted(() => ({
  flow: null as { minZoom?: number; fitViewOptions?: FitViewOptions } | null,
  controls: null as { fitViewOptions?: FitViewOptions } | null,
  fits: [] as (FitViewOptions | undefined)[],
}))

vi.mock('@xyflow/react', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@xyflow/react')>()
  const { createElement, useMemo } = await import('react')
  return {
    ...actual,
    ReactFlow: (props: React.ComponentProps<typeof actual.ReactFlow>) => {
      observed.flow = props
      return createElement(actual.ReactFlow, props)
    },
    Controls: (props: React.ComponentProps<typeof actual.Controls>) => {
      observed.controls = props
      return createElement(actual.Controls, props)
    },
    useReactFlow: () => {
      const api = actual.useReactFlow()
      return useMemo(
        () => ({
          ...api,
          fitView: (options?: FitViewOptions) => {
            observed.fits.push(options)
            return api.fitView(options)
          },
        }),
        [api],
      )
    },
  }
})

let cleanup: (() => void) | undefined
beforeEach(() => {
  vi.stubGlobal('ResizeObserver', FakeResizeObserver)
  observed.fits = []
})
afterEach(() => {
  cleanup?.()
  vi.unstubAllGlobals()
})

async function mount(element: React.ReactElement): Promise<HTMLElement> {
  const container = document.createElement('div')
  document.body.append(container)
  const root = createRoot(container)
  cleanup = () => {
    act(() => root.unmount())
    container.remove()
  }
  await act(async () => root.render(element))
  return container
}

it('fits readably at first but lets the person zoom out to the whole plan', async () => {
  await mount(<PlanGraph plan={planFixture()} />)

  // Wheel and zoom-out reach far enough for a 60-task plan.
  expect(observed.flow?.minZoom).toBeLessThanOrEqual(0.2)
  // The first view never shrinks text below a readable size, nor enlarges it.
  expect(observed.flow?.fitViewOptions?.minZoom).toBeGreaterThanOrEqual(0.75)
  expect(observed.flow?.fitViewOptions?.maxZoom).toBe(1)
  // The fit button shows the whole plan.
  expect(observed.controls?.fitViewOptions?.minZoom).toBeLessThanOrEqual(0.2)
})

it('a card fits with its own options', async () => {
  const container = await mount(<PlanGraph plan={planFixture()} compact />)

  const card = observed.flow?.fitViewOptions?.minZoom ?? 1
  expect(card).toBeLessThan(0.75)
  expect(observed.flow?.minZoom).toBeLessThanOrEqual(0.2)
  expect(container.querySelector('.react-flow__minimap')).toBeNull()
})

it('refits through React Flow when its canvas changes size, on the same canvas', async () => {
  const container = await mount(<PlanGraph plan={planFixture()} />)
  const canvas = container.querySelector('[data-plan-canvas]')
  const flow = container.querySelector('.react-flow')
  if (canvas === null || flow === null) throw new Error('no canvas')

  // The first size is the first fit's own.
  await act(async () => resize(canvas, 1100, 700))
  await act(() => new Promise(requestAnimationFrame))
  expect(observed.fits).toEqual([])

  // Inspect opened beside it: narrower.
  await act(async () => resize(canvas, 800, 700))
  await act(() => new Promise(requestAnimationFrame))
  expect(observed.fits).toHaveLength(1)
  expect(observed.fits[0]?.minZoom).toBeGreaterThanOrEqual(0.75)
  expect(container.querySelector('.react-flow')).toBe(flow)
})
