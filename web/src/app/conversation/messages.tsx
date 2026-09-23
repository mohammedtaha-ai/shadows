// One job: the conversation's messages — the durable entries in order, then
// whatever the running turn is saying.

import { AnimatePresence, motion } from 'motion/react'
import { LoaderCircle } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import type { ThreadEntry } from '@/api/client'
import { describeLabel } from './operational-label'
import type { ShownReply } from './reply'
import { ReplyText } from './reply-text'

const appear = {
  initial: { opacity: 0, y: 6 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: 0.18, ease: 'easeOut' },
} as const

export function Messages({
  entries,
  loading,
  reply,
  running,
  thinking,
  label,
}: {
  entries: readonly ThreadEntry[] | undefined
  /** The stream has not caught up yet: history is still arriving. */
  loading: boolean
  reply: ShownReply | null
  running: boolean
  /** A turn runs and has said nothing yet. */
  thinking: boolean
  label: string | undefined
}) {
  const scroller = useRef<HTMLDivElement>(null)
  const pinned = useRef(true)

  // Follow the bottom while the reader is there; leave them be once they
  // scroll up to read.
  useEffect(() => {
    const el = scroller.current
    if (el !== null && pinned.current) el.scrollTop = el.scrollHeight
  })

  // An entry that takes over from the streamed reply is already on screen as
  // that reply's text: it must not fade in again. Recorded while the reply
  // shows, well before the entry replaces it (state adjusted during render,
  // React's pattern for remembering what an earlier render saw).
  const [handedOver, setHandedOver] = useState<ReadonlySet<number>>(() => new Set())
  const newlyHidden = [...(reply?.hidden ?? [])].filter((ordinal) => !handedOver.has(ordinal))
  if (newlyHidden.length > 0) setHandedOver(new Set([...handedOver, ...newlyHidden]))

  const shown = entries?.filter((entry) => !reply?.hidden.has(entry.ordinal)) ?? []
  const operational = label === undefined ? null : describeLabel(label)

  return (
    <div
      ref={scroller}
      onScroll={(e) => {
        const el = e.currentTarget
        pinned.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48
      }}
      className="min-h-0 flex-1 overflow-y-auto"
    >
      <ol className="mx-auto flex max-w-3xl flex-col gap-6 px-6 py-8">
        {loading && (
          <li className="flex items-center gap-2 text-xs text-faint-foreground">
            <LoaderCircle className="size-3.5 animate-spin motion-reduce:animate-none" />
            Loading history
          </li>
        )}
        {!loading && shown.length === 0 && reply === null && !running && (
          <li className="py-16 text-center text-sm text-faint-foreground">
            Ask the Planner something about this project.
          </li>
        )}
        <AnimatePresence initial={false}>
          {shown.map((entry) => (
            <motion.li
              key={entry.id}
              {...appear}
              initial={handedOver.has(entry.ordinal) ? false : appear.initial}
            >
              <Entry entry={entry} />
            </motion.li>
          ))}
          {reply !== null && (
            <motion.li key="reply" {...appear}>
              <ReplyText text={reply.text} live={reply.live} />
            </motion.li>
          )}
          {thinking && (
            <motion.li key="waiting" {...appear} className="flex items-center gap-2">
              <span className="size-2 animate-pulse rounded-full bg-accent-line motion-reduce:animate-none" />
              <span className="text-xs text-faint-foreground">Thinking</span>
            </motion.li>
          )}
          {running && operational !== null && (
            <motion.li
              key="label"
              {...appear}
              className="flex items-center gap-2 text-xs text-faint-foreground"
            >
              <operational.icon className="size-3.5" />
              {operational.text}
            </motion.li>
          )}
        </AnimatePresence>
      </ol>
    </div>
  )
}

function Entry({ entry }: { entry: ThreadEntry }) {
  if (entry.kind === 'UserMessage') {
    return (
      <div className="flex justify-end">
        <p className="max-w-[80%] rounded-2xl rounded-br-md border border-border bg-card px-4 py-2.5 text-sm whitespace-pre-wrap text-card-foreground">
          {entry.body}
        </p>
      </div>
    )
  }
  if (entry.kind === 'AgentMessage') return <ReplyText text={entry.body} />
  return <p className="text-xs text-faint-foreground">{entry.body}</p>
}
