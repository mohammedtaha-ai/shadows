// One job: reading the `data:` of each `/api/subscribe` frame into a typed
// value. The frame shapes are the daemon's (`src/protocol/sse.rs`, described in
// the OpenAPI document's `/api/subscribe` entry); OpenAPI cannot type them, so
// they are checked here, and a frame that does not match is a protocol error.

/** One journal event. `operationId` and `threadId` are the ids the event
 * names, `null` where it names none; `payload` is the event's JSON payload. */
export interface DurableEvent {
  seq: number
  kind: string
  operationId: string | null
  threadId: string | null
  payload: unknown
}

/** Streamed text of a running turn. Transient. */
export interface Delta {
  op: string
  text: string
}

/** The harness finished a turn. Transient. */
export interface TurnEnd {
  op: string
  subtype: string
  stopReason: string | null
}

/** Any other harness line, by label. Transient. */
export interface Meta {
  op: string
  label: string
}

export class FrameError extends Error {
  constructor(event: string, data: string) {
    super(`malformed \`${event}\` frame: ${data.slice(0, 200)}`)
    this.name = 'FrameError'
  }
}

export function parseDurable(data: string): DurableEvent {
  const frame = object('durable', data)
  const { seq, kind, payload } = frame
  const operationId = frame.operation_id
  const threadId = frame.thread_id
  if (
    !isSeq(seq) ||
    typeof kind !== 'string' ||
    !('payload' in frame) ||
    !isIdOrNull(operationId) ||
    !isIdOrNull(threadId)
  ) {
    throw new FrameError('durable', data)
  }
  return { seq, kind, operationId, threadId, payload }
}

/** `caught-up` carries the last replayed seq. */
export function parseCaughtUp(data: string): number {
  const { seq } = object('caught-up', data)
  if (!isSeq(seq)) {
    throw new FrameError('caught-up', data)
  }
  return seq
}

export function parseDelta(data: string): Delta {
  const frame = object('delta', data)
  const { op, text } = frame
  if (typeof op !== 'string' || typeof text !== 'string') {
    throw new FrameError('delta', data)
  }
  return { op, text }
}

export function parseTurnEnd(data: string): TurnEnd {
  const frame = object('turn-end', data)
  const { op, subtype } = frame
  const stopReason = frame.stop_reason ?? null
  if (
    typeof op !== 'string' ||
    typeof subtype !== 'string' ||
    (stopReason !== null && typeof stopReason !== 'string')
  ) {
    throw new FrameError('turn-end', data)
  }
  return { op, subtype, stopReason }
}

export function parseMeta(data: string): Meta {
  const frame = object('meta', data)
  const { op, label } = frame
  if (typeof op !== 'string' || typeof label !== 'string') {
    throw new FrameError('meta', data)
  }
  return { op, label }
}

function isSeq(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
}

/** Present, and either an id or `null`. A missing field is a daemon that
 * does not send it, which this client cannot follow. */
function isIdOrNull(value: unknown): value is string | null {
  return value === null || typeof value === 'string'
}

function json(event: string, data: string): unknown {
  try {
    return JSON.parse(data)
  } catch {
    throw new FrameError(event, data)
  }
}

function object(event: string, data: string): Record<string, unknown> {
  const value = json(event, data)
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new FrameError(event, data)
  }
  return Object.fromEntries(Object.entries(value))
}
