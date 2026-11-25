import { useMutation } from '@tanstack/react-query'
import { useRef, useState, type ReactNode } from 'react'
import { attemptFor, type Attempt } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { ErrorLine } from './error-line'

export interface SavingSwitchProps<T> {
  /** The switch's label, also its accessible name. */
  name: string
  /** The line under the switch. */
  description: string
  /** Core's saved value; shown whenever no change is in flight. */
  enabled: boolean
  /** What identifies this switch's change, without `enabled`. With `enabled` it is the
   *  fingerprint `attemptFor` keeps a command id for. */
  request: Record<string, unknown>
  /** Sends one change; answers Core's saved value. */
  send: (enabled: boolean, commandId: string) => Promise<T>
  /** Writes the answer into the cache. Awaited before the optimistic state clears. */
  onSaved: (saved: T) => Promise<void>
  /** Called with the failure, after the switch has gone back. */
  onFailed?: (error: Error) => void
  /** Drawn under the error line, inside the row. */
  children?: ReactNode
}

export function SavingSwitch<T>({ name, description, enabled, request, send, onSaved, onFailed, children }: SavingSwitchProps<T>) {
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const [optimistic, setOptimistic] = useState<boolean | null>(null)
  const toggle = useMutation({ retry: false,
    mutationFn: (next: boolean) => {
      attempt.current = attemptFor(attempt.current, { ...request, enabled: next })
      return send(next, attempt.current.commandId)
    },
    onSuccess: async saved => {
      attempt.current = null
      await onSaved(saved)
    },
    onError: error => {
      if (error instanceof ApiError && error.problem.kind !== 'unreachable' && error.problem.status === 409) {
        attempt.current = null
      }
      onFailed?.(error)
    },
    onSettled: () => { locked.current = false; setOptimistic(null) },
  })
  return (
    <div className="surface-panel space-y-2 rounded-lg p-3">
      <label className="flex items-center justify-between gap-3 text-sm">
        <span>{name}</span>
        <input
          type="checkbox"
          role="switch"
          aria-label={name}
          checked={optimistic ?? enabled}
          disabled={toggle.isPending}
          onChange={event => {
            if (locked.current) return
            locked.current = true; setOptimistic(event.target.checked); toggle.mutate(event.target.checked)
          }}
        />
      </label>
      <p className="text-xs text-muted-foreground">{description}</p>
      {toggle.error && <ErrorLine error={toggle.error} />}
      {children}
    </div>
  )
}
