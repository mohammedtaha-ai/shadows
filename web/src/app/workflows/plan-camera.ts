// One job: where the plan graph's view looks — its first fit, the fit its
// controls ask for, and the refit when its canvas changes size.
//
// The first fit never zooms out past a readable size; a plan wider than the
// canvas at that size is panned. Zoom itself is never locked, so the wheel
// and the Controls' fit button can still show a 60-task plan whole.

import { type FitViewOptions, useNodesInitialized, useReactFlow } from '@xyflow/react'
import { type RefObject, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { taskNodeId } from './layout'

/** How far the person may zoom out, by wheel or button. */
export const MIN_ZOOM = 0.1

/** The Controls' fit button: the whole plan, however small that makes it. */
export const WHOLE_PLAN: FitViewOptions = { minZoom: MIN_ZOOM, maxZoom: 1, padding: 0.15 }

/** The first view: readable text, the page's or a card's (W2) own minimum. */
export function firstFit(compact: boolean, focusTask: number | undefined): FitViewOptions {
  return {
    nodes: focusTask === undefined ? undefined : [{ id: taskNodeId(focusTask) }],
    minZoom: compact ? 0.5 : 0.8,
    maxZoom: 1,
    padding: compact ? 0.1 : 0.15,
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

/** Called inside `<ReactFlow>`. Centres `focusTask` whenever it changes, and when
 * the canvas changes size (Inspect opening beside it, a window resized)
 * looks again without remounting, so selection survives: at the task the
 * person last clicked, at its current zoom, or else as the first view did. */
export function useCamera({
  size,
  compact,
  focusTask,
  lastClicked,
}: {
  size: CanvasSize | null
  compact: boolean
  focusTask: number | undefined
  lastClicked: number | null
}): void {
  const { fitView, getZoom } = useReactFlow()
  const ready = useNodesInitialized()

  useEffect(() => {
    if (focusTask === undefined || !ready) return
    void fitView({ ...firstFit(compact, focusTask), padding: 0.4, duration: 200 })
  }, [focusTask, ready, compact, fitView])

  // The latest values, read when a resize lands: only a new size refits.
  const latest = useRef({ compact, focusTask, lastClicked, fitView, getZoom })
  useLayoutEffect(() => {
    latest.current = { compact, focusTask, lastClicked, fitView, getZoom }
  })
  const seen = useRef<CanvasSize | null>(null)
  useEffect(() => {
    const before = seen.current
    seen.current = size
    // The first size is the first fit's, which React Flow does itself.
    if (size === null || before === null) return
    // After React Flow has read the new pane size from the same layout.
    const frame = requestAnimationFrame(() => {
      const now = latest.current
      const task = now.focusTask ?? now.lastClicked
      if (task === null) {
        void now.fitView(firstFit(now.compact, undefined))
      } else {
        const zoom = now.getZoom()
        void now.fitView({ nodes: [{ id: taskNodeId(task) }], minZoom: zoom, maxZoom: zoom, duration: 200 })
      }
    })
    return () => cancelAnimationFrame(frame)
  }, [size])
}
