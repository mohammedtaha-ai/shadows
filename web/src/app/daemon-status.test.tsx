// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { projectsReachabilityQuery } from '@/api/projects'
import { FakeSource } from '@/test/fake-event-source'
import { until } from '@/test/test-app'
import { ConnectionIndicator } from './daemon-status'

it('shows the project stream connection and gives the project list its interval only with no project', async () => {
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const host = document.createElement('div')
  document.body.append(host)
  const root = createRoot(host)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  try {
    await act(async () => root.render(<QueryClientProvider client={client}><ConnectionIndicator projectId="p1" /></QueryClientProvider>))
    await until(() => sources.length === 1)
    expect(host.textContent).toContain('Connecting')
    act(() => sources[0].caughtUp(0))
    await until(() => host.textContent!.includes('Connected'))
    act(() => sources[0].fail())
    await until(() => !host.textContent!.includes('Connected'))
    expect(host.textContent).toMatch(/Connecting|Not reachable/)
    expect(client.isFetching()).toBe(0)
  } finally { act(() => root.unmount()); host.remove(); client.clear(); vi.unstubAllGlobals() }
})

it('labels a failed stream as not reachable', async () => {
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const host = document.createElement('div')
  const root = createRoot(host)
  const client = new QueryClient()
  try {
    await act(async () => root.render(<QueryClientProvider client={client}><ConnectionIndicator projectId="p2" /></QueryClientProvider>))
    for (let i = 0; i < 12 && !host.textContent!.includes('Not reachable'); i++) {
      await act(async () => { sources.at(-1)?.fail(); await new Promise(r => setTimeout(r, 0)) })
    }
    expect(host.textContent).toMatch(/Connecting|Not reachable/)
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})

it('polls the project list every 30 s only with no project open and after an answer', () => {
  const interval = (open: boolean, data: unknown) => {
    const fn = projectsReachabilityQuery(open).refetchInterval as (q: { state: { data: unknown } }) => unknown
    return fn({ state: { data } })
  }
  expect(interval(true, [])).toBe(false)
  expect(interval(false, [])).toBe(30_000)
  expect(interval(false, undefined)).toBe(false)
})
