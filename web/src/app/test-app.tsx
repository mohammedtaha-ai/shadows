// One job: running the whole app in a test against a faked daemon — its
// routes answered from a table, its streams driven by hand. Test-only; nothing
// outside a `*.test.tsx` imports it.

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { type Root, createRoot } from 'react-dom/client'
import { expect, vi } from 'vitest'
import { FakeSource } from '@/stream/fake-event-source'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

/** An answer: a JSON body, or a function of the request for anything else. */
export type Answer = unknown | ((request: Request) => Response | Promise<Response>)

export interface TestApp {
  readonly container: HTMLElement
  /** Every request, as `METHOD /path`, in order. */
  readonly calls: string[]
  /** The JSON body of each request that had one, in order. */
  readonly bodies: unknown[]
  readonly sources: FakeSource[]
  button(name: string): HTMLButtonElement | undefined
  unmount(): void
}

/** Starts the app's real router at `url`, with each `METHOD /path` in
 * `answers` answered as given and any other request answered 404. */
export async function startApp(url: string, answers: Record<string, Answer>): Promise<TestApp> {
  const calls: string[] = []
  const bodies: unknown[] = []
  const sources: FakeSource[] = []
  vi.stubGlobal(
    'EventSource',
    class extends FakeSource {
      constructor(source: string) {
        super(source)
        sources.push(this)
      }
    },
  )
  vi.stubGlobal('fetch', async (request: Request) => {
    const key = `${request.method} ${new URL(request.url).pathname}`
    calls.push(key)
    const text = await request.clone().text()
    if (text !== '') bodies.push(JSON.parse(text))
    const answer = answers[key]
    if (answer === undefined) return new Response('no such route in this test', { status: 404 })
    return typeof answer === 'function' ? answer(request) : Response.json(answer)
  })
  window.history.replaceState(null, '', url)
  const container = document.createElement('div')
  document.body.append(container)

  // happy-dom never finishes an animation, and a badge crossfade waits for
  // one. Set on the module instance the app is about to import: the app is
  // imported fresh (see `vi.resetModules` in `stop`) so its router reads `url`.
  const { MotionGlobalConfig } = await import('motion/react')
  MotionGlobalConfig.skipAnimations = true
  const { router } = await import('@/router')
  const { RouterProvider } = await import('@tanstack/react-router')

  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const root: Root = createRoot(container)
  await act(async () => {
    root.render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    )
  })

  return {
    container,
    calls,
    bodies,
    sources,
    button: (name) =>
      [...document.querySelectorAll('button')].find(
        (b) => b.textContent?.trim() === name || b.getAttribute('aria-label') === name,
      ),
    unmount: () => {
      act(() => root.unmount())
      container.remove()
      vi.unstubAllGlobals()
      vi.resetModules()
    },
  }
}

/** Lets pending fetches and renders settle until `ready` holds, for up to
 * three seconds: a cold first run (modules still being transformed) is slow. */
export async function until(ready: () => boolean): Promise<void> {
  const deadline = Date.now() + 3000
  while (!ready() && Date.now() < deadline) {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 5))
    })
  }
  expect(ready()).toBe(true)
}

/** Types into a controlled input the way React hears it. */
export function typeInto(input: HTMLInputElement | HTMLTextAreaElement, text: string): void {
  const prototype = Object.getPrototypeOf(input) as object
  const setter = Object.getOwnPropertyDescriptor(prototype, 'value')?.set
  act(() => {
    setter?.call(input, text)
    input.dispatchEvent(new Event('input', { bubbles: true }))
  })
}
