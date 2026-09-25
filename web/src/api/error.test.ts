import createClient from 'openapi-fetch'
import { describe, expect, it } from 'vitest'
import { ApiError, unwrap } from './error'
import type { paths } from './schema'

/** A client whose every request gets `answer` — through openapi-fetch's own
 * parsing, so the test covers what the app actually receives. */
function clientAnswering(answer: () => Promise<Response>) {
  return createClient<paths>({ baseUrl: 'http://daemon', fetch: answer })
}

async function failureOf(pending: Promise<unknown>): Promise<ApiError> {
  try {
    await pending
  } catch (error) {
    if (error instanceof ApiError) return error
    throw error
  }
  throw new Error('the call succeeded')
}

describe('unwrap', () => {
  it("maps the daemon's ErrorBody to a daemon problem carrying its code", async () => {
    const client = clientAnswering(async () =>
      Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'database is locked' }, { status: 500 }),
    )

    const error = await failureOf(unwrap(client.GET('/api/projects')))

    expect(error.problem).toEqual({ kind: 'daemon', status: 500, code: 'STORAGE_UNAVAILABLE' })
    expect(error.message).toBe('database is locked')
  })

  it('keeps the current revision and the problems an ErrorBody carries', async () => {
    const conflict = clientAnswering(async () =>
      Response.json(
        { code: 'REVISION_CONFLICT', message: 'the plan changed', current_revision: 4 },
        { status: 409 },
      ),
    )
    const invalid = clientAnswering(async () =>
      Response.json(
        { code: 'WORKFLOW_VALIDATION_FAILED', message: 'T1 has no goal', problems: ['T1 has no goal'] },
        { status: 422 },
      ),
    )

    expect((await failureOf(unwrap(conflict.GET('/api/projects')))).problem).toStrictEqual({
      kind: 'daemon',
      status: 409,
      code: 'REVISION_CONFLICT',
      currentRevision: 4,
    })
    expect((await failureOf(unwrap(invalid.GET('/api/projects')))).problem).toStrictEqual({
      kind: 'daemon',
      status: 422,
      code: 'WORKFLOW_VALIDATION_FAILED',
      problems: ['T1 has no goal'],
    })
  })

  it("maps axum's plain-text rejection to an http problem", async () => {
    const client = clientAnswering(
      async () => new Response('Failed to deserialize query string', { status: 400 }),
    )

    const error = await failureOf(unwrap(client.GET('/api/projects')))

    expect(error.problem).toEqual({
      kind: 'http',
      status: 400,
      body: 'Failed to deserialize query string',
    })
  })

  it('maps a failure with an empty body to an http problem', async () => {
    const client = clientAnswering(
      async () => new Response(null, { status: 503, headers: { 'Content-Length': '0' } }),
    )

    const error = await failureOf(unwrap(client.GET('/api/projects')))

    expect(error.problem).toEqual({ kind: 'http', status: 503, body: '' })
  })

  it('maps no answer at all to unreachable', async () => {
    const client = clientAnswering(async () => {
      throw new TypeError('Failed to fetch')
    })

    const error = await failureOf(unwrap(client.GET('/api/projects')))

    expect(error.problem).toEqual({ kind: 'unreachable' })
  })

  it('returns the body of a success', async () => {
    const client = clientAnswering(async () => Response.json([]))

    await expect(unwrap(client.GET('/api/projects'))).resolves.toEqual([])
  })
})
