import { afterEach, expect, it, vi } from 'vitest'
import { codeWhereQuery } from './code-record'
import { retryLost } from './error'

afterEach(() => vi.unstubAllGlobals())

it('the_topic_is_sent_as_typed', async () => {
  const fetch = vi.fn(async (_request: Request) => Response.json({}))
  vi.stubGlobal('fetch', fetch)
  await (codeWhereQuery('p1', '  login ').queryFn as () => Promise<unknown>)()
  const url = new URL(fetch.mock.calls[0][0].url)
  expect(url.pathname).toBe('/api/projects/p1/code-where')
  expect(url.searchParams.get('topic')).toBe('  login ')
  expect(url.search).toMatch(/^\?topic=(%20|\+){2}login(%20|\+)$/)
})

it('the_question_retries_only_a_lost_call', () => {
  expect(codeWhereQuery('p1', 'login').retry).toBe(retryLost)
})
