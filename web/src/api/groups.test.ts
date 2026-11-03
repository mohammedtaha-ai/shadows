import { afterEach, expect, it, vi } from 'vitest'
import { createGroup, listGroupMessages, sendToGroup } from './groups'

afterEach(() => vi.unstubAllGlobals())

it('sends the caller’s command id with each group write', async () => {
  const fetch = vi.fn(async (_request: Request) => Response.json({}))
  vi.stubGlobal('fetch', fetch)
  await createGroup({ name: 'Group', members: ['alpha'], command_id: 'create' })
  await sendToGroup('g1', 'Hello', 'send')
  const [create, send] = fetch.mock.calls.map(([request]) => request)
  expect(new URL(create.url).pathname).toBe('/api/groups')
  expect(await create.json()).toEqual({ name: 'Group', members: ['alpha'], command_id: 'create' })
  expect(new URL(send.url).pathname).toBe('/api/groups/g1/messages')
  expect(await send.json()).toEqual({ text: 'Hello', command_id: 'send' })
})

it('reads messages after a sequence', async () => {
  const fetch = vi.fn(async (_request: Request) => Response.json([]))
  vi.stubGlobal('fetch', fetch)
  await expect(listGroupMessages('g1', 4)).resolves.toEqual([])
  const url = new URL(fetch.mock.calls[0][0].url)
  expect(url.pathname).toBe('/api/groups/g1/messages')
  expect(url.searchParams.get('after')).toBe('4')
})
