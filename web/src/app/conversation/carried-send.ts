// Keep the failed first message and its command id together so a retry can replay it.

import { useEffect, useState } from 'react'
import type { PendingImage } from './upload-images'
import type { Attempt } from '@/api/command-id'

export interface CarriedSend {
  readonly images?: PendingImage[]
  readonly text: string
  /** The failed turn's command, as `attemptFor` would have made it. */
  readonly attempt: Attempt
  readonly error: Error
  /** The selected plan remains part of a retry's command and prompt context. */
  readonly planId?: string
}

const carried = new Map<string, CarriedSend>()

export function carrySend(threadId: string, send: CarriedSend): void {
  carried.set(threadId, send)
}

/** What was carried to `threadId`, read once: taken on the first render and
 * dropped after it, so a later visit starts empty. */
export function useCarriedSend(threadId: string): CarriedSend | null {
  const [send] = useState(() => carried.get(threadId) ?? null)
  useEffect(() => {
    carried.delete(threadId)
  }, [threadId])
  return send
}
