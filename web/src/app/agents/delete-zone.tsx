// One job: the danger zone at the foot of a provider or agent form — what
// deleting does, and a dialog that confirms it.

import { Trash2 } from 'lucide-react'
import { type ReactNode, useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
} from '@/components/ui/dialog'
import { ErrorLine } from '../error-line'

export function DeleteZone({ title, summary, consequence, action, pending, error, onConfirm, inline = false, compact = false }: {
  /** "Delete provider". */
  title: string
  /** One line under the title, before anything is clicked. */
  summary: ReactNode
  /** What the dialog says will happen. */
  consequence: ReactNode
  /** The dialog's question, such as "Delete MiniMax?". */
  action: string
  pending: boolean
  error: Error | null
  inline?: boolean
  compact?: boolean
  onConfirm: () => void
}) {
  const [open, setOpen] = useState(false)
  const confirmation = <div className="grid gap-4" role={inline ? 'group' : undefined} aria-label={inline ? action : undefined}>
    {inline ? <><h3 className="font-medium">{action}</h3><p className="text-sm text-muted-foreground">{consequence}</p></> :
      <DialogHeader><DialogTitle>{action}</DialogTitle><DialogDescription>{consequence}</DialogDescription></DialogHeader>}
    {error !== null && <ErrorLine error={error} />}
    <DialogFooter>
      <Button type="button" variant="outline" onClick={() => setOpen(false)}>Cancel</Button>
      <Button type="button" variant="destructive" onClick={onConfirm} disabled={pending}>{pending ? 'Deleting...' : title}</Button>
    </DialogFooter>
  </div>
  return (
    <section className={compact ? 'space-y-3' : 'space-y-3 rounded-md border border-destructive-border p-4'}>
      <div className="flex items-start justify-between gap-4">
        {!compact && <div className="space-y-1">
          <h2 className="text-sm font-medium text-destructive-foreground">{title}</h2>
          <p className="text-xs text-muted-foreground">{summary}</p>
        </div>}
        <Button size="sm" variant="destructive" onClick={() => setOpen(true)}>
          <Trash2 />
          Delete
        </Button>
      </div>
      {inline ? open && confirmation : <Dialog open={open} onOpenChange={setOpen}><DialogContent>{confirmation}</DialogContent></Dialog>}

    </section>
  )
}
