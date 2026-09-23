// One job: the conversation's turn status as a badge — running, or how the
// latest turn ended.

import { LoaderCircle } from 'lucide-react'
import { AnimatePresence, motion } from 'motion/react'
import type { Turn } from './turn-state'

interface Look {
  readonly text: string
  readonly spinning: boolean
  readonly className: string
}

const RUNNING = 'bg-secondary text-secondary-foreground'
const QUIET = 'border border-border text-muted-foreground'

function look(running: Turn | null, latest: Turn | null): Look | null {
  if (running !== null) {
    if (running.stopRequested) return { text: 'Stopping', spinning: true, className: RUNNING }
    const text = running.status === 'Pending' ? 'Starting' : 'Running'
    return { text, spinning: true, className: RUNNING }
  }
  switch (latest?.status) {
    case 'Completed':
      return { text: 'Completed', spinning: false, className: QUIET }
    case 'Failed':
      return {
        text: 'Failed',
        spinning: false,
        className: 'border border-destructive-border text-destructive-foreground',
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
          className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs ${shown.className}`}
        >
          {shown.spinning && (
            <LoaderCircle className="size-3 animate-spin motion-reduce:animate-none" />
          )}
          {shown.text}
        </motion.span>
      )}
    </AnimatePresence>
  )
}
