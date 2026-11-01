import { afterEach, expect, it, vi } from 'vitest'
import { waitingPermissionsQuery, answerPermission } from './permissions'
afterEach(() => vi.unstubAllGlobals())
it('permission queries and answers use the generated routes and the caller command', async () => {
  const calls: Request[] = []
  vi.stubGlobal('fetch', async (request: Request) => {
    calls.push(request)
    return Response.json(request.method === 'GET' ? [] : { request_id: 'r1', option_id: 'allow' })
  })
  expect(await waitingPermissionsQuery.queryFn!({} as never)).toEqual([])
  await answerPermission('t1', 'r1', 'allow', 'cmd1')
  expect(new URL(calls[0].url).pathname).toBe('/api/aylul/permissions')
  expect(new URL(calls[1].url).pathname).toBe('/api/threads/t1/session/permissions/r1')
  expect(calls[1].method).toBe('POST')
  expect(await calls[1].json()).toEqual({ command_id: 'cmd1', option_id: 'allow' })
})
