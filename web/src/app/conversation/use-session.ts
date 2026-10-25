// One job: the open conversation's harness session as its screen sees it —
// connecting, ready with what it offers, or failed with why.
//
// Opening is `POST /api/threads/{id}/session` (spec §12.2): idempotent, so it
// is held as a query, asked once when the conversation opens and again only
// when the harness changes (`reopenSession` in `api/sessions.ts`) or the person
// presses Retry. An `options` frame replaces what it holds (`replaceChoices`).

import { useQuery } from '@tanstack/react-query'
import type { SessionChoices } from '@/api/client'
import { sessionQuery } from '@/api/sessions'

export type SessionView =
  | { state: 'connecting' }
  | { state: 'ready'; choices: SessionChoices }
  | { state: 'failed'; error: Error; retry: () => void }

export function useSession(threadId: string): SessionView {
  const session = useQuery(sessionQuery(threadId))
  if (session.data !== undefined) return { state: 'ready', choices: session.data }
  if (session.isError && !session.isFetching) {
    return { state: 'failed', error: session.error, retry: () => void session.refetch() }
  }
  return { state: 'connecting' }
}
