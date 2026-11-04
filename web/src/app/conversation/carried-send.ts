// Keep the failed first message and its command id together so a retry can replay it.

import { useEffect, useState, useSyncExternalStore } from 'react'
import type { PendingImage } from './upload-images'
import type { Attempt } from '@/api/command-id'
import { ApiError } from '@/api/error'
import type { ThreadPlanningAnchor } from '@/api/planning-context'

export interface PendingAnchor {
  readonly value: ThreadPlanningAnchor
  readonly attempt: Attempt
  readonly refused: boolean
}

export type AnchorRecovery =
  | { kind: 'set'; anchor: PendingAnchor }
  | { kind: 'clear'; attempt: Attempt; unsettled?: PendingAnchor }

export const anchorWriteKey = (threadId: string | null) => ['thread-anchor-write', threadId] as const

/** A transport or server failure can leave the detached write in flight. */
export function anchorWriteRefused(error: unknown): boolean {
  return error instanceof ApiError && error.problem.kind === 'daemon' &&
    error.problem.status >= 400 && error.problem.status < 500
}

export interface CarriedSend {
  readonly images?: PendingImage[]
  readonly text: string
  /** The failed turn's command, as `attemptFor` would have made it. */
  readonly attempt: Attempt
  readonly error: Error
  /** The selected plan remains part of a retry's command and prompt context. */
  readonly planId?: string
  readonly anchor?: AnchorRecovery
}

const carried = new Map<string, CarriedSend>()
const anchorListeners = new Set<() => void>()

function subscribeToAnchor(listener: () => void) {
  anchorListeners.add(listener)
  return () => { anchorListeners.delete(listener) }
}

export function carrySend(threadId: string, send: CarriedSend): void {
  carried.set(threadId, send)
  if (send.anchor !== undefined) anchorListeners.forEach(listener => listener())
}

export function readAnchorRecovery(threadId: string | null): AnchorRecovery | null {
  return threadId === null ? null : carried.get(threadId)?.anchor ?? null
}

export function saveAnchorRecovery(threadId: string, anchor: AnchorRecovery | null): void {
  const send = carried.get(threadId)
  if (send === undefined) return
  if (anchor === null) carried.delete(threadId)
  else carried.set(threadId, { ...send, anchor })
  anchorListeners.forEach(listener => listener())
}

export function useAnchorRecovery(threadId: string | null): AnchorRecovery | null {
  return useSyncExternalStore(subscribeToAnchor, () => readAnchorRecovery(threadId))
}

/** Keep the first draft available on a revisit until its pending anchor has settled. */
export function useCarriedSend(threadId: string): CarriedSend | null {
  const [send, setSend] = useState(() => carried.get(threadId) ?? null)
  useEffect(() => {
    if (carried.get(threadId)?.anchor === undefined) carried.delete(threadId)
    const receive = () => {
      const incoming = carried.get(threadId)
      if (incoming?.anchor !== undefined) {
        setSend(previous => previous?.attempt === incoming.attempt ? previous : incoming)
      }
    }
    const unsubscribe = subscribeToAnchor(receive)
    receive()
    return unsubscribe
  }, [threadId])
  return send
}
