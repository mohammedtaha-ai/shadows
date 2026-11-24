// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import type { LastRecheck } from '@/api/runs'
import { RecheckControl, RecheckLine } from './recheck-control'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

afterEach(() => {
  document.body.innerHTML = ''
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

const last = (counts: Partial<LastRecheck['counts']>, extra: Partial<LastRecheck> = {}): LastRecheck => ({
  recheck_id: 'r1', began: { kind: 'person' }, created_at: '2026-11-24T09:00:00Z', finished_at: '2026-11-24T09:05:00Z',
  counts: { passed: 0, failed: 0, timed_out: 0, inconclusive: 0, not_run: 0, unfinished: 0, ...counts }, ...extra,
})

async function draw(answer: LastRecheck | null) {
  const posts: unknown[] = []
  let gets = 0
  vi.stubGlobal('fetch', async (request: Request) => {
    if (request.method === 'POST') {
      posts.push(await request.json())
      return Response.json({ recheck_id: 'r2' }, { status: 202 })
    }
    gets += 1
    return Response.json(answer)
  })
  const queries = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const host = document.createElement('div')
  document.body.append(host)
  await act(async () => createRoot(host).render(<QueryClientProvider client={queries}>
    <RecheckLine workflowId="w1" /><RecheckControl workflowId="w1" />
  </QueryClientProvider>))
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 0)) })
  return { host, posts, gets: () => gets }
}

it('idle_with_a_last_result_shows_the_counts_and_the_button', async () => {
  const { host } = await draw(last({ passed: 4, failed: 2, not_run: 1 }))
  expect(host.textContent).toMatch(/Last project recheck \d\d:\d\d/)
  expect(host.textContent).toContain('4 passed')
  expect(host.textContent).toContain('2 failed')
  expect(host.textContent).toContain('1 not run')
  expect(host.querySelector('.text-destructive')?.textContent).toBe('2 failed')
  expect(host.querySelector('button')?.textContent).toBe('Recheck project')
})

it('no_failures_leaves_the_failed_count_out', async () => {
  const { host } = await draw(last({ passed: 3 }))
  expect(host.textContent).not.toContain('failed')
})

it('no_result_shows_the_button_and_no_line', async () => {
  const { host } = await draw(null)
  expect(host.textContent).not.toContain('Last project recheck')
  expect(host.querySelector('button')?.textContent).toBe('Recheck project')
})

it('running_replaces_the_button_and_reads_again_every_5_seconds', async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  const { host, gets } = await draw(last({ unfinished: 2 }, { finished_at: null }))
  expect(host.textContent).toContain('Rechecking…')
  expect(host.querySelector('button')).toBeNull()
  const before = gets()
  await act(async () => { await vi.advanceTimersByTimeAsync(5100) })
  expect(gets()).toBeGreaterThan(before)
})

it('a_click_posts_once_with_a_command_id', async () => {
  const { host, posts } = await draw(null)
  await act(async () => { host.querySelector('button')?.click() })
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 0)) })
  expect(posts).toHaveLength(1)
  expect((posts[0] as { command_id: string }).command_id).toMatch(/^[0-9a-f-]{36}$/)
})
