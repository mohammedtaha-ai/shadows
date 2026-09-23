// One job: every way a call to the daemon can fail, as one typed error.

import type { components } from './schema'

export type ErrorCode = components['schemas']['ErrorCode']
type ErrorBody = components['schemas']['ErrorBody']

/** Why a call failed, in the terms a screen needs to say what to do next. */
export type Problem =
  /** No HTTP answer at all: the daemon is not running, or the browser refused
   * the answer because this page's origin is not in `--allow-origin`. A
   * browser reports both identically, so the client cannot tell them apart. */
  | { kind: 'unreachable' }
  /** The daemon answered with its own `ErrorBody` (spec §3.4). Match on `code`. */
  | { kind: 'daemon'; status: number; code: ErrorCode }
  /** An HTTP failure that is not an `ErrorBody` — axum rejecting a request
   * before a handler ran answers in plain text. */
  | { kind: 'http'; status: number; body: string }

export class ApiError extends Error {
  readonly problem: Problem

  constructor(problem: Problem, message: string, options?: { cause?: unknown }) {
    super(message, options)
    this.name = 'ApiError'
    this.problem = problem
  }
}

/** The failure a non-2xx answer stands for. `body` is what openapi-fetch
 * parsed: JSON when it could, the raw text otherwise, `undefined` when empty. */
export function toApiError(response: Response, body: unknown): ApiError {
  if (isErrorBody(body)) {
    return new ApiError(
      { kind: 'daemon', status: response.status, code: body.code },
      body.message,
    )
  }
  const text = typeof body === 'string' ? body : body === undefined ? '' : JSON.stringify(body)
  return new ApiError(
    { kind: 'http', status: response.status, body: text },
    text === '' ? `HTTP ${response.status}` : `HTTP ${response.status}: ${text}`,
  )
}

/** What an openapi-fetch call resolves to, reduced to what `unwrap` reads. */
type Answer<T> = { data?: T; error?: unknown; response: Response }

/** The body of a successful call, or the `ApiError` its failure stands for.
 * For a route that answers with a body; none of the daemon's routes answers
 * a success without one. */
export async function unwrap<T>(pending: Promise<Answer<T>>): Promise<T> {
  let answer: Answer<T>
  try {
    answer = await pending
  } catch (cause) {
    // `fetch` rejects with a TypeError, and only a TypeError, when no HTTP
    // answer arrived. Anything else thrown here is a bug and stays loud.
    if (cause instanceof TypeError) {
      throw new ApiError({ kind: 'unreachable' }, 'The daemon is not reachable', { cause })
    }
    throw cause
  }
  // Read `ok`, not `error`: openapi-fetch leaves `error` undefined for a
  // failure with an empty body.
  if (!answer.response.ok) {
    throw toApiError(answer.response, answer.error)
  }
  if (answer.data === undefined) {
    throw toApiError(answer.response, 'a success with no body')
  }
  return answer.data
}

/** `code` is trusted to be an `ErrorCode`: the type is generated from this
 * daemon's own document, and CI fails when the two drift. */
function isErrorBody(body: unknown): body is ErrorBody {
  return (
    typeof body === 'object' &&
    body !== null &&
    'code' in body &&
    typeof body.code === 'string' &&
    'message' in body &&
    typeof body.message === 'string'
  )
}
