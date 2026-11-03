// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import type { WaitingPermission } from '@/api/permissions'
import { until } from '@/test/test-app'
import { PermissionAnswer } from './permission-answer'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

const request: WaitingPermission = {
  thread_id: 'hidden1', request_id: 'r1', agent: 'alpha', title: 'Run a command', asked_at: 'now',
  options: [{ option_id: 'allow', name: 'Allow once', kind: 'allow_once' }, { option_id: 'reject', name: 'Reject', kind: 'reject_once' }],
}
let root: Root | undefined
afterEach(() => { act(() => root?.unmount()); root = undefined; vi.unstubAllGlobals() })

function render(answer: (call: number) => Response) {
  const posts: { path: string; body: unknown }[] = []
  vi.stubGlobal('fetch', vi.fn(async (input: Request) => {
    if (input.method !== 'POST') return Response.json([])
    posts.push({ path: new URL(input.url).pathname, body: await input.json() })
    return answer(posts.length)
  }))
  const host = document.createElement('div')
  document.body.append(host)
  root = createRoot(host)
  act(() => root!.render(
    <QueryClientProvider client={new QueryClient()}><PermissionAnswer request={request} /></QueryClientProvider>,
  ))
  const button = (name: string) => [...host.querySelectorAll('button')].find(item => item.textContent === name)
  return { posts, button }
}

it('permission answer answers a waiting request', async () => {
  const view = render(() => Response.json({ request_id: 'r1', option_id: 'allow' }))
  expect(view.button('Reject')).toBeDefined()
  act(() => view.button('Allow once')!.click())
  await until(() => view.button('Allow once') === undefined)
  expect(view.posts).toEqual([{
    path: '/api/threads/hidden1/session/permissions/r1',
    body: { command_id: expect.any(String), option_id: 'allow' },
  }])
})

it('a retry after a lost answer reuses the command id', async () => {
  const view = render(call => {
    if (call === 1) throw new TypeError('Failed to fetch')
    return Response.json({ request_id: 'r1', option_id: 'allow' })
  })
  act(() => view.button('Allow once')!.click())
  await until(() => view.posts.length === 1 && view.button('Allow once')?.disabled === false)
  act(() => view.button('Allow once')!.click())
  await until(() => view.button('Allow once') === undefined)
  expect(view.posts[1].body).toEqual(view.posts[0].body)
})
