// @vitest-environment happy-dom
//
// PlanGraph at its React Flow boundary: the first view is readable, zoom is
// never locked, and resizing keeps the view unless a task is being inspected.

import { getNodesBounds, getViewportForBounds, type FitViewOptions, type Viewport } from '@xyflow/react'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { planFixture, planTask } from '@/test/contract-fixtures'
import { FakeResizeObserver, resize } from '@/test/fake-resize-observer'
import { layoutPlan } from './layout'
import { PlanGraph } from './plan-graph'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

const observed = vi.hoisted(() => ({
  flow: null as { minZoom?: number; fitViewOptions?: FitViewOptions } | null,
  controls: null as { fitViewOptions?: FitViewOptions } | null,
  fits: [] as (FitViewOptions | undefined)[],
  viewports: [] as Viewport[],
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
    useNodesInitialized: () => true,
    useReactFlow: () => {
      const api = actual.useReactFlow()
      return useMemo(
        () => ({
          ...api,
          fitView: (options?: FitViewOptions) => {
            observed.fits.push(options)
            return api.fitView(options)
          },
          setViewport: (viewport: Viewport) => {
            observed.viewports.push(viewport)
            return api.setViewport(viewport)
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
  observed.viewports = []
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

function chain(count: number) {
  return planFixture({
    tasks: Array.from({ length: count }, (_, index) => planTask(index + 1, `Task ${index + 1}`)),
    links: Array.from({ length: count - 1 }, (_, index) => ({
      task: index + 2, after: index + 1, kind: 'needs' as const, label: 'next', waiting_items: [],
    })),
  })
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

it('the fit control can show a linear sixty-task plan end to end', async () => {
  const plan = chain(60)
  await mount(<PlanGraph plan={plan} />)
  const bounds = getNodesBounds(layoutPlan(plan).nodes)
  const view = getViewportForBounds(bounds, 1100, 700, observed.controls?.fitViewOptions?.minZoom ?? 1, 1, 0.15)
  expect(view.x + bounds.x * view.zoom).toBeGreaterThanOrEqual(0)
  expect(view.x + (bounds.x + bounds.width) * view.zoom).toBeLessThanOrEqual(1100)
})

it('opens a wide plan at its start column at a readable zoom', async () => {
  const plan = chain(20)
  const container = await mount(<PlanGraph plan={plan} />)
  const canvas = container.querySelector('[data-plan-canvas]')
  if (canvas === null) throw new Error('no canvas')

  await act(async () => resize(canvas, 1100, 700))
  await act(() => new Promise(requestAnimationFrame))
  expect(observed.viewports).toHaveLength(1)
  const bounds = getNodesBounds(layoutPlan(plan).nodes)
  const initial = getViewportForBounds(bounds, 1100, 700, 0.8, 1, 0.15)
  expect(observed.viewports[0]?.zoom).toBe(initial.zoom)
  const leftPadding = Math.floor((1100 - 1100 / 1.15) * 0.5)
  expect(observed.viewports[0]?.x).toBeCloseTo(leftPadding - bounds.x * initial.zoom)
})

it('keeps the current view when the canvas changes size without a selected task', async () => {
  const container = await mount(<PlanGraph plan={planFixture()} />)
  const canvas = container.querySelector('[data-plan-canvas]')
  const flow = container.querySelector('.react-flow')
  if (canvas === null || flow === null) throw new Error('no canvas')

  // The first size is the first fit's own.
  await act(async () => resize(canvas, 1100, 700))
  await act(() => new Promise(requestAnimationFrame))
  expect(observed.fits).toEqual([])

  // A window resize must not reset the person's pan or zoom.
  await act(async () => resize(canvas, 800, 700))
  await act(() => new Promise(requestAnimationFrame))
  expect(observed.fits).toEqual([])
  expect(container.querySelector('.react-flow')).toBe(flow)
})
