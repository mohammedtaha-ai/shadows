// One job: one line saying why a call to the daemon failed, inline where it
// was made.

import { CircleAlert } from 'lucide-react'
import { ApiError } from '@/api/error'

export function ErrorLine({ error }: { error: Error }) {
  const code =
    error instanceof ApiError && error.problem.kind === 'daemon' ? error.problem.code : null
  const message =
    error instanceof ApiError && error.problem.kind === 'unreachable'
      ? 'The daemon is not reachable.'
      : error.message
  return (
    <p role="alert" className="flex items-start gap-2 text-xs text-destructive-foreground">
      <CircleAlert aria-hidden className="mt-px size-3.5 shrink-0" />
      <span>
        {code !== null && <code className="mr-1.5 font-mono">{code}</code>}
        {message}
      </span>
    </p>
  )
}
