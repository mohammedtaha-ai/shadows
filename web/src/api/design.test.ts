// The request each design call sends, read off a stubbed `fetch`: the route,
// and the fact that it went through client.ts's one client.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { getOutcome, getPart, getVision } from './design'

afterEach(() => vi.unstubAllGlobals())

/** Stubs `fetch` to answer `answer`, recording each request's full URL. */
function record(answer: () => Response) {
  const calls: string[] = []
  vi.stubGlobal('fetch', async (r: Request) => {
    calls.push(r.url)
    return answer()
  })
  return { calls }
}

describe('design', () => {
  it('reads each route through the shared client', async () => {
    const { calls } = record(() =>
      Response.json({
        revision: 0,
        content: {},
        id: 'x',
        parent: null,
        ordinal: 0,
        ancestors: [],
        plans: [],
        parts: [],
      }),
    )
    await getVision('p1')
    await getPart('p1', 'part-1')
    await getOutcome('p1', 'out-1')
    expect(calls).toEqual([
      'http://127.0.0.1:4318/api/projects/p1/design/vision',
      'http://127.0.0.1:4318/api/projects/p1/design/parts/part-1',
      'http://127.0.0.1:4318/api/projects/p1/design/outcomes/out-1',
    ])
  })
})