// One job: the conversation's turn status as a badge — running, or how the
// latest turn ended.

import { LoaderCircle } from 'lucide-react'
import { AnimatePresence, motion } from 'motion/react'
import type { Turn } from './turn-state'

interface Look {
  readonly text: string
  readonly spinning: boolean
  readonly className: string
  readonly dot?: string
}

const RUNNING = 'border border-success/40 bg-success/15 text-success'
const QUIET = 'border border-border/80 bg-card/80 text-muted-foreground'

function look(running: Turn | null, latest: Turn | null): Look | null {
  if (running !== null) {
    if (running.stopRequested) {
      return {
        text: 'Stopping',
        spinning: true,
        className: 'border border-warning/40 bg-warning/15 text-warning',
        dot: 'bg-warning',
      }
    }
    const text = running.status === 'Pending' ? 'Starting' : 'Running'
    return { text, spinning: true, className: RUNNING, dot: 'bg-success' }
  }
  switch (latest?.status) {
    case 'Completed':
      return {
        text: 'Completed',
        spinning: false,
        className: 'border border-accent-line/35 bg-accent-softer/90 text-secondary-foreground',
        dot: 'bg-accent-line',
      }
    case 'Failed':
      return {
        text: 'Failed',
        spinning: false,
        className: 'border border-destructive-border bg-destructive/15 text-destructive-foreground',
        dot: 'bg-destructive',
      }
    case 'Cancelled':
      return { text: 'Stopped', spinning: false, className: QUIET }
    case 'Interrupted':
      return { text: 'Interrupted', spinning: false, className: QUIET }
    default:
      return null
  }
}

export function StatusBadge({ running, latest }: { running: Turn | null; latest: Turn | null }) {
  const shown = look(running, latest)
  return (
    <AnimatePresence mode="wait" initial={false}>
      {shown !== null && (
        <motion.span
          key={shown.text}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
          aria-live="polite"
          className={`inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium tracking-wide ${shown.className}`}
        >
          {shown.dot && !shown.spinning && (
            <span aria-hidden className={`size-1.5 rounded-full ${shown.dot}`} />
          )}
          {shown.spinning && (
            <LoaderCircle className="size-3.5 animate-spin motion-reduce:animate-none" />
          )}
          {shown.text}
        </motion.span>
      )}
    </AnimatePresence>
  )
}
