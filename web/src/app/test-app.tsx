// One job: running the whole app in a test against a faked daemon — its
// routes answered from a table, its streams driven by hand. Test-only; nothing
// outside a `*.test.tsx` imports it.

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { type Root, createRoot } from 'react-dom/client'
import { expect, vi } from 'vitest'
import { FakeSource } from '@/stream/fake-event-source'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

// Load the whole app once while the test file is collected, outside every
// test's timeout. The first import transforms every app module and loads the
// dependencies — about two seconds idle, past five under load — and if a test
// pays for it, the first test in each file times out and leaves its app
// mounted for the next. `resetModules` then drops the instance loaded here, so
// `startApp` still imports the app fresh; what stays warm is the transform
// cache, which makes that import a few milliseconds.
await import('motion/react')
await import('@tanstack/react-router')
await import('@/router')
vi.resetModules()

/** An answer: a JSON body, or a function of the request for anything else. */
export type Answer = unknown | ((request: Request) => Response | Promise<Response>)

export interface TestApp {
  readonly container: HTMLElement
  readonly queryClient: QueryClient
  /** Every request, as `METHOD /path`, in order. */
  readonly calls: string[]
  /** The JSON body of each request that had one, in order. */
  readonly bodies: unknown[]
  readonly sources: FakeSource[]
  /** The first button named `name` by its text or its `aria-label`,
   * anywhere in the page (menus and popups render outside the app's root). */
  button(name: string): HTMLButtonElement | undefined
  /** Every button so named, in document order. */
  buttons(name: string): HTMLButtonElement[]
  /** The checkbox whose label reads `name`. */
  checkbox(name: string): HTMLInputElement | undefined
  /** All the page's text, popups included. */
  text(): string
  /** The URL path the router is at. */
  path(): string
  /** Delivers one SSE frame, `data` as JSON, on the latest stream. */
  pushFrame(name: string, data: unknown): void
  unmount(): void
}

function buttonsNamed(name: string): HTMLButtonElement[] {
  return [...document.querySelectorAll('button')].filter(
    (b) => b.textContent?.trim() === name || b.getAttribute('aria-label') === name,
  )
}

/** The open menu's item that reads `name`, or starts with it (an item may
 * carry a note after its name, such as "coming"). */
export function menuItem(name: string): HTMLElement | undefined {
  const items = [...document.querySelectorAll<HTMLElement>('[role^="menuitem"]')]
  return (
    items.find((i) => i.textContent?.trim() === name) ??
    items.find((i) => i.textContent?.trim().startsWith(name))
  )
}

/** Opens the menu whose trigger reads `trigger` and picks `item` from it. */
export async function choose(app: TestApp, trigger: string, item: string): Promise<void> {
  const button = app.button(trigger)
  if (button === undefined) throw new Error(`no menu button reads "${trigger}"`)
  act(() => button.click())
  await until(() => menuItem(item) !== undefined)
  act(() => menuItem(item)?.click())
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

  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const root: Root = createRoot(container)
  const unmount = () => {
    act(() => root.unmount())
    container.remove()
    vi.unstubAllGlobals()
    vi.resetModules()
  }
  try {
    // happy-dom never finishes an animation, and a badge crossfade waits for
    // one. Set on the module instance the app is about to import: the app is
    // imported fresh (see `vi.resetModules` in `unmount`) so its router reads `url`.
    const { MotionGlobalConfig } = await import('motion/react')
    MotionGlobalConfig.skipAnimations = true
    const { router } = await import('@/router')
    const { RouterProvider } = await import('@tanstack/react-router')
    await act(async () => {
      root.render(
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>,
      )
    })
    // Pages load lazily (router.tsx): let the first one arrive before a test reads it.
    await act(async () => {
      await router.load()
    })
  } catch (error) {
    // The caller never receives an app to unmount, so a failed start cleans
    // up here: otherwise its stubs and mounted tree leak into the next test.
    unmount()
    throw error
  }

  return {
    container,
    queryClient,
    calls,
    bodies,
    sources,
    button: (name) => buttonsNamed(name)[0],
    buttons: buttonsNamed,
    checkbox: (name) =>
      [...document.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')].find(
        (i) => i.closest('label')?.textContent?.trim() === name,
      ),
    text: () => document.body.textContent ?? '',
    path: () => window.location.pathname,
    pushFrame: (name, data) => {
      const source = sources.findLast((s) => new URL(s.url).pathname === '/api/subscribe')
      if (source === undefined) throw new Error('no stream was opened')
      source.emit(name, JSON.stringify(data))
    },
    unmount,
  }
}

/** Lets pending fetches and renders settle until `ready` holds, for up to
 * three seconds. */
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
