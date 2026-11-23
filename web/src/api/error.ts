// Public failure types let callers interpret transport errors.
import { ApiError } from './transport'

export { ApiError }

/** Retries a lost call up to three times; Core's own answer is final. */
export function retryLost(failures: number, error: Error): boolean {
  return failures < 3 && error instanceof ApiError && error.problem.kind === 'unreachable'
}
