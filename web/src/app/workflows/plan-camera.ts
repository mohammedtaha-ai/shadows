// One job: where the plan graph's view looks — its first fit, the fit its
// controls ask for, and the refit when its canvas changes size.
//
// The first fit never zooms out past a readable size; a plan wider than the
// canvas at that size opens at its start. Zoom itself is never locked, so the wheel
// and the Controls' fit button can still show a 60-task plan whole.

import { getNodesBounds, getViewportForBounds, type FitViewOptions, useNodesInitialized, useReactFlow } from '@xyflow/react'
import { type RefObject, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { type PlanNode, taskNodeId } from './layout'

/** How far the person may zoom out, by wheel or button. */
export const MIN_ZOOM = 0.01
const PAGE_PADDING = 0.15

/** The Controls' fit button: the whole plan, however small that makes it. */
export const WHOLE_PLAN: FitViewOptions = { minZoom: MIN_ZOOM, maxZoom: 1, padding: PAGE_PADDING }

/** The first view: readable text, the page's or a card's (W2) own minimum. */
export function firstFit(compact: boolean, focusTask: number | undefined): FitViewOptions {
  return {
    nodes: focusTask === undefined ? undefined : [{ id: taskNodeId(focusTask) }],
    minZoom: compact ? 0.5 : 0.8,
    maxZoom: 1,
    padding: compact ? 0.1 : PAGE_PADDING,
  }
}

export interface CanvasSize {
  width: number
  height: number
}

/** The element's size as the browser lays it out, `null` until first observed. */
export function useCanvasSize(ref: RefObject<HTMLElement | null>): CanvasSize | null {
  const [size, setSize] = useState<CanvasSize | null>(null)
  useEffect(() => {
    const element = ref.current
    if (element === null) return
    const observer = new ResizeObserver((entries) => {
      const box = entries.at(-1)?.contentRect
      if (box === undefined) return
      setSize((was) =>
        was?.width === box.width && was.height === box.height
          ? was
          : { width: box.width, height: box.height },
      )
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [ref])
  return size
}

/** Called inside `<ReactFlow>`. Opens a wide plan at its start, and keeps a
 * clicked or focused task in view when Inspect changes the canvas width. */
export function useCamera({
  size,
  compact,
  focusTask,
  lastClicked,
  nodes,
}: {
  size: CanvasSize | null
  compact: boolean
  focusTask: number | undefined
  lastClicked: number | null
  nodes: PlanNode[]
}): void {
  const { fitView, getZoom, setViewport } = useReactFlow()
  const ready = useNodesInitialized()

  const opened = useRef(false)
  useEffect(() => {
    if (opened.current || !ready || size === null || size.width === 0 || size.height === 0) return
    if (compact || focusTask !== undefined) {
      opened.current = true
      return
    }
    const frame = requestAnimationFrame(() => {
      const bounds = getNodesBounds(nodes)
      const view = getViewportForBounds(bounds, size.width, size.height, 0.8, 1, PAGE_PADDING)
      // Numeric React Flow padding is a fraction of the fitted bounds.
      const padding = Math.floor((size.width - size.width / (1 + PAGE_PADDING)) * 0.5)
      if (bounds.width * view.zoom > size.width - 2 * padding) {
        void setViewport({ ...view, x: padding - bounds.x * view.zoom })
      }
      opened.current = true
    })
    return () => cancelAnimationFrame(frame)
  }, [ready, size, compact, focusTask, nodes, setViewport])

  useEffect(() => {
    if (focusTask === undefined || !ready) return
    void fitView({ ...firstFit(compact, focusTask), padding: 0.4, duration: 200 })
  }, [focusTask, ready, compact, fitView])

  // The latest values, read when a resize lands: only a new size refits.
  const latest = useRef({ focusTask, lastClicked, fitView, getZoom })
  useLayoutEffect(() => {
    latest.current = { focusTask, lastClicked, fitView, getZoom }
  })
  const seen = useRef<CanvasSize | null>(null)
  useEffect(() => {
    const before = seen.current
    seen.current = size
    // The first size is handled by React Flow's fit and the start alignment above.
    if (size === null || before === null) return
    // After React Flow has read the new pane size from the same layout.
    const frame = requestAnimationFrame(() => {
      const now = latest.current
      const task = now.focusTask ?? now.lastClicked
      if (task !== null) {
        const zoom = now.getZoom()
        void now.fitView({ nodes: [{ id: taskNodeId(task) }], minZoom: zoom, maxZoom: zoom, duration: 200 })
      }
    })
    return () => cancelAnimationFrame(frame)
  }, [size])
}
