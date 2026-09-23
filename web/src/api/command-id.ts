// One job: which idempotency key (spec §3.2) a command carries on each attempt
// a person makes to send it.
//
// A retry of the same request reuses the key, so an attempt whose answer was
// lost (the daemon applied it, the browser never heard) replays the first
// result instead of creating a second project. A changed request is a new
// command with a fresh key: reusing one would be refused as COMMAND_CONFLICT.

/** The command last sent: its request, canonically serialized, and its key. */
export interface Attempt {
  readonly fingerprint: string
  readonly commandId: string
}

/** The attempt for `request`, given the previous unfinished one (`null` once
 * one succeeded, or when the form was opened afresh). */
export function attemptFor(
  previous: Attempt | null,
  request: unknown,
  fresh: () => string = () => crypto.randomUUID(),
): Attempt {
  const fingerprint = canonical(request)
  if (previous !== null && previous.fingerprint === fingerprint) return previous
  return { fingerprint, commandId: fresh() }
}

/** JSON with object keys sorted, so the same request always reads the same. */
function canonical(value: unknown): string {
  return JSON.stringify(value, (_key, v: unknown) =>
    typeof v === 'object' && v !== null && !Array.isArray(v)
      ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)))
      : v,
  )
}
