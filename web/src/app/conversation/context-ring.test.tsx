// @vitest-environment happy-dom
//

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { type Root, createRoot } from 'react-dom/client'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { completedOperation, invocationFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import type { Limits } from '@/stream'
import { startApp, until } from '@/test/test-app'
import { ContextRing } from './context-ring'
import type { ContextFigures } from './usage'

// The clock the summary test reads, fixed: reset times count from the moment
// the ring opens, so a real clock that ticks a second between here and the
// click reads 4h17m.
const nowSec = Date.parse('2026-09-24T03:00:00Z') / 1000

const mounted: (() => void)[] = []
afterEach(() => {
  for (const unmount of mounted.splice(0)) unmount()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

function renderRing({
  usage,
  limits,
}: {
  usage: ContextFigures | null
  limits: Limits | null
}) {
  const calls: string[] = []
  vi.stubGlobal('fetch', async (r: Request) => {
    const key = `${r.method} ${new URL(r.url).pathname}`
    calls.push(key)
    return new Response(null, { status: 404 })
  })
  const container = document.createElement('div')
  document.body.append(container)
  const root: Root = createRoot(container)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  act(() =>
    root.render(
      <QueryClientProvider client={client}>
        <ContextRing threadId="t1" usage={usage} limits={limits} />
      </QueryClientProvider>,
    ),
  )
  mounted.push(() => {
    act(() => root.unmount())
    container.remove()
  })
  const trigger = () => {
    const button = container.querySelector<HTMLButtonElement>('button[aria-label="Context and limits"]')
    if (button === null) throw new Error('no ring')
    return button
  }
  return {
    container,
    calls,
    trigger,
    text: () => document.body.textContent ?? '',
    button: (name: string) =>
      [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === name),
  }
}

describe('the context ring', () => {
  it('opens at once with no figures and never shows a spinner', () => {
    const r = renderRing({ usage: null, limits: null })
    act(() => r.trigger().click())
    expect(r.text()).toContain('No figures yet')
    expect(r.container.querySelector('[role="progressbar"][aria-busy="true"]')).toBeNull()
    expect(document.querySelector('[aria-busy="true"]')).toBeNull()
    expect(r.calls).toEqual([])
  })

  it('shows the summary level: context, five-hour and weekly with resets, and when observed', () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(nowSec * 1000)
    const r = renderRing({
      usage: { contextUsed: 126800, contextWindow: 1_000_000 },
      limits: {
        fiveHour: { utilization: 0.16, resetsAt: nowSec + 4 * 3600 + 18 * 60 },
        sevenDay: { utilization: 0.96, resetsAt: nowSec + 86400 },
        observedAt: '2026-09-24T02:49:00Z',
      },
    })
    act(() => r.trigger().click())
    for (const t of ['126.8k / 1M (13%)', '5-hour limit', 'Resets in 4h18m', '16%', 'Weekly', '96%', 'updated']) {
      expect(r.text()).toContain(t)
    }
  })

  it('opens with no daemon request and points to /context for the full breakdown', () => {
    const r = renderRing({ usage: { contextUsed: 126800, contextWindow: 1_000_000 }, limits: null })
    act(() => r.trigger().click())
    expect(r.text()).toContain('Type /context in the conversation for the full breakdown.')
    expect(r.button('Details')).toBeUndefined()
    expect(r.calls).toEqual([])
  })
})

describe('the answering model', () => {
  it('a reply whose observed model differs from the requested one says both', async () => {
    const app = await startApp(
      '/projects/p1/threads/t1',
      answers({
        operations: [
          completedOperation({
            ...invocationFixture,
            requested_model: 'fake-small',
            observed_model: 'fake-large-answering',
          }),
        ],
      }),
    )
    mounted.push(app.unmount)
    await until(() => app.text().includes('Asked for fake-small · answered by fake-large-answering'))
  })

  it('a reply answered by the model asked for says nothing more', async () => {
    const app = await startApp(
      '/projects/p1/threads/t1',
      answers({ operations: [completedOperation(invocationFixture)] }),
    )
    mounted.push(app.unmount)
    await until(() => app.text().includes('hello'))
    expect(app.text()).not.toContain('Asked for')
  })
})
