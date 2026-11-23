import { afterEach, expect, it, vi } from 'vitest'
import { ApiError, retryLost } from './error'
import { elementRevisionsQuery, elementsQuery, projectStateQuery } from './project-state'

afterEach(() => vi.unstubAllGlobals())

it('the_state_read_sends_part_id_only_when_asked', async () => {
  const fetch = vi.fn(async (_request: Request) => Response.json({}))
  vi.stubGlobal('fetch', fetch)
  const run = (query: ReturnType<typeof projectStateQuery>) =>
    (query.queryFn as () => Promise<unknown>)()
  await run(projectStateQuery('p1'))
  await run(projectStateQuery('p1', 'web'))
  const urls = fetch.mock.calls.map(([request]) => {
    const url = new URL(request.url)
    return url.pathname + url.search
  })
  expect(urls).toEqual(['/api/projects/p1/state', '/api/projects/p1/state?part_id=web'])
})

it('the_reads_retry_only_a_lost_call', () => {
  expect(projectStateQuery('p1').retry).toBe(retryLost)
  expect(elementsQuery('p1').retry).toBe(retryLost)
  expect(elementRevisionsQuery('p1', 'E1').retry).toBe(retryLost)
  const refused = new ApiError({ kind: 'daemon', status: 422, code: 'INVALID_COMMAND' }, 'no part')
  const lost = new ApiError({ kind: 'unreachable' }, 'lost')
  expect(retryLost(0, refused)).toBe(false)
  expect(retryLost(2, lost)).toBe(true)
  expect(retryLost(3, lost)).toBe(false)
})
