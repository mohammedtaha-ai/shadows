import { afterEach, expect, it, vi } from 'vitest'
import { carriedContextQuery, resendContext, setAnchor } from './planning-context'
import { QueryClient } from '@tanstack/react-query'

afterEach(() => vi.unstubAllGlobals())

it('sends_the_callers_command_id', async () => {
  const fetch = vi.fn(async (request: Request) => {
    const body = await request.json()
    expect(body.command_id).toBe('caller-id')
    if (request.method === 'PUT') {
      expect(new URL(request.url).pathname).toBe('/api/threads/t1/anchor')
      expect(body.anchor).toEqual({ kind: 'part', id: 'part-1' })
      return Response.json({ anchor: body.anchor })
    }
    expect(request.method).toBe('POST')
    expect(new URL(request.url).pathname).toBe('/api/threads/t1/context/resend')
    return Response.json({ owed: true })
  })
  vi.stubGlobal('fetch', fetch)

  await expect(setAnchor('t1', 'caller-id', { kind: 'part', id: 'part-1' }))
    .resolves.toEqual({ anchor: { kind: 'part', id: 'part-1' } })
  await expect(resendContext('t1', 'caller-id')).resolves.toEqual({ owed: true })
  expect(fetch).toHaveBeenCalledTimes(2)
})

it('a_missing_record_is_null', async () => {
  const fetch = vi.fn()
    .mockResolvedValueOnce(Response.json({ code: 'CONTEXT_NOT_CARRIED', message: 'No record.' }, { status: 404 }))
    .mockResolvedValueOnce(new Response(null, { status: 404 }))
  vi.stubGlobal('fetch', fetch)
  const client = new QueryClient()
  await expect(client.fetchQuery(carriedContextQuery('o1'))).resolves.toBeNull()
  await expect(client.fetchQuery(carriedContextQuery('o2'))).rejects.toMatchObject({ problem: { kind: 'http', status: 404 } })
})
