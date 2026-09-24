// One job: the open conversation's harness session as its screen sees it —
// connecting, ready with what it offers, or failed with why.
//
// Opening is `POST /api/threads/{id}/session` (spec §12.2): idempotent, so it
// is held as a query, asked once when the conversation opens and again only
// when the harness changes (`reopenSession`) or the person presses Retry. An
// `options` frame replaces what it holds (`replaceChoices`).

import { type QueryClient, useQuery } from '@tanstack/react-query'
import { type SessionChoices, openSession } from '@/api/client'

export type SessionView =
  | { state: 'connecting' }
  | { state: 'ready'; choices: SessionChoices }
  | { state: 'failed'; error: Error; retry: () => void }

export function sessionKey(threadId: string) {
  return ['threads', threadId, 'session'] as const
}

export function useSession(threadId: string): SessionView {
  const session = useQuery({
    queryKey: sessionKey(threadId),
    queryFn: () => openSession(threadId),
    // Never on a timer or on focus: an open session answers what it holds,
    // and the stream says when that changes.
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    // A failure is shown with a Retry at once, never behind silent retries.
    retry: false,
  })
  if (session.data !== undefined) return { state: 'ready', choices: session.data }
  if (session.isError && !session.isFetching) {
    return { state: 'failed', error: session.error, retry: () => void session.refetch() }
  }
  return { state: 'connecting' }
}

/** The session's choices changed (an `options` frame). Ignored while the
 * opening is in flight: opening sets mode, model, then effort, each step's
 * frame arrives before the opening answers, and the answer is the result.
 * Taking the first step as the session's state kept its effort for good. */
export function replaceChoices(queryClient: QueryClient, threadId: string, choices: SessionChoices) {
  const state = queryClient.getQueryState(sessionKey(threadId))
  if (state?.data === undefined || state.fetchStatus === 'fetching') return
  queryClient.setQueryData(sessionKey(threadId), choices)
}

/** The thread's harness changed: forget the old session's choices and open
 * the new one's. */
export function reopenSession(queryClient: QueryClient, threadId: string) {
  void queryClient.resetQueries({ queryKey: sessionKey(threadId) })
}
