import type { ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { DialogClose, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ErrorLine } from './error-line'

type Presentation =
  | { kind: 'dialog'; className?: string }
  | { kind: 'inline'; ariaLabel: string; className: string; heading?: boolean }

export function ConfirmPanel({
  question, description, error, confirmLabel, pending, pendingLabel = confirmLabel,
  cancelDisabledWhilePending = false, onCancel, onConfirm, presentation, confirmType,
}: {
  question: ReactNode
  description?: ReactNode
  error: Error | null
  confirmLabel: string
  pending: boolean
  pendingLabel?: string
  cancelDisabledWhilePending?: boolean
  onCancel?: () => void
  onConfirm: () => void
  presentation: Presentation
  confirmType?: 'button'
}) {
  const cancel = onCancel
    ? <Button type="button" variant="outline" disabled={cancelDisabledWhilePending && pending} onClick={onCancel}>Cancel</Button>
    : <DialogClose render={<Button type="button" variant="outline" disabled={cancelDisabledWhilePending && pending} />}>Cancel</DialogClose>
  const buttons = <>
    {cancel}
    <Button type={confirmType} variant="destructive" onClick={onConfirm} disabled={pending}>
      {pending ? pendingLabel : confirmLabel}
    </Button>
  </>
  const content = <>
    {presentation.kind === 'dialog'
      ? <DialogHeader><DialogTitle>{question}</DialogTitle><DialogDescription>{description}</DialogDescription></DialogHeader>
      : presentation.heading
        ? <><h3 className="font-medium">{question}</h3><p className="text-sm text-muted-foreground">{description}</p></>
        : <p>{question}</p>}
    {error !== null && <ErrorLine error={error} />}
    {presentation.kind === 'dialog' || presentation.heading
      ? <DialogFooter>{buttons}</DialogFooter>
      : <div className="flex gap-3">{buttons}</div>}
  </>
  if (presentation.kind === 'inline') {
    return <div className={presentation.className} role="group" aria-label={presentation.ariaLabel}>{content}</div>
  }
  return presentation.className ? <div className={presentation.className}>{content}</div> : content
}
