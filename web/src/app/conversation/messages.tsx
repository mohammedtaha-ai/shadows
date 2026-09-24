// One job: the conversation's messages — the durable entries in order, then
// whatever the running turn is saying.

import { AnimatePresence, motion } from 'motion/react'
import { LoaderCircle } from 'lucide-react'
import { Suspense, lazy, useEffect, useRef, useState } from 'react'
import type { Choice, Operation, ThreadEntry } from '@/api/client'
import { describeLabel } from './operational-label'
import type { ShownReply } from './reply'
import { labelOf } from './turn-settings'

// Streamdown and its code highlighting are most of this app's weight, so they
// are a chunk of their own, and not in the one every screen waits for. The
// load starts when this module does, so by the time a reply arrives it has
// usually landed; until it has, the reply shows as plain text.
const loadReplyText = () => import('./reply-text')
const LazyReplyText = lazy(async () => ({ default: (await loadReplyText()).ReplyText }))
void loadReplyText()

function ReplyText({ text, live = false }: { text: string; live?: boolean }) {
  return (
    <Suspense
      fallback={
        <p className="font-serif text-[15px] leading-7 whitespace-pre-wrap text-foreground">
          {text}
        </p>
      }
    >
      <LazyReplyText text={text} live={live} />
    </Suspense>
  )
}

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
  operations,
  models,
}: {
  entries: readonly ThreadEntry[] | undefined
  /** The thread's turns, for what each asked for and was answered by. */
  operations: readonly Operation[] | undefined
  /** The session's models, for their labels. */
  models: readonly Choice[]
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
  const answered = answeredNotes(shown, operations ?? [], models)

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
              {answered.has(entry.id) && (
                <p className="mt-2 text-xs text-faint-foreground">{answered.get(entry.id)}</p>
              )}
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

/** For each turn whose answering model is not the one asked for, its note,
 * by the id of the turn's last entry shown (spec §12.11). */
function answeredNotes(
  entries: readonly ThreadEntry[],
  operations: readonly Operation[],
  models: readonly Choice[],
): Map<string, string> {
  const last = new Map<string, string>()
  for (const entry of entries) {
    if (entry.operation_id !== null) last.set(entry.operation_id, entry.id)
  }
  const notes = new Map<string, string>()
  for (const operation of operations) {
    const entryId = last.get(operation.id)
    const invocation = operation.invocation
    if (entryId === undefined || invocation == null) continue
    const { requested_model: requested, observed_model: observed } = invocation
    // The request names the harness's option ("sonnet"), the answer a model
    // id ("claude-sonnet-5"): an id that contains the option is the same one.
    if (observed === null || observed.includes(requested)) continue
    notes.set(entryId, `Asked for ${labelOf(models, requested)} · answered by ${observed}`)
  }
  return notes
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
