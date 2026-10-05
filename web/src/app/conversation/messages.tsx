// One job: the conversation's messages — the durable entries in order, then
// whatever the running turn is saying.

import { AnimatePresence, motion } from 'motion/react'
import { LoaderCircle } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import type { Choice, Operation, Plan, PlanTask, ThreadEntry } from '@/api/client'
import { policyOf } from '../mode-policy'
import { Entry, ReplyText } from './entry'
import { MessageActions } from './message-actions'
import { describeLabel } from './operational-label'
import type { SubagentCard } from '@/stream/frames'
import type { ShownReply } from './reply'
import { SubagentCardView } from './subagent-card'
import { copyText, isTool, silent, subagentOf } from './tool-text'
import { labelOf } from './turn-settings'

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
  harness,
  forkFrom,
  projectId,
  onPointAt,
  subagents,
  onOpenSubagent,
}: {
  entries: readonly ThreadEntry[] | undefined
  projectId: string
  /** A task was clicked in a plan card. */
  onPointAt: (plan: Plan, task: PlanTask) => void
  /** The running turns' subagent cards as the stream last sent them. */
  subagents: readonly SubagentCard[]
  /** A subagent's card was clicked (§22.4). */
  onOpenSubagent: (id: string) => void
  /** The thread's turns, for what each asked for and was answered by. */
  operations: readonly Operation[] | undefined
  /** The session's models, for their labels. */
  models: readonly Choice[]
  /** The thread's harness kind, whose policy names a refused permission's modes. */
  harness: string
  /** Where a fork from the last message goes; `null` unless the thread is
   * known to be idle. */
  forkFrom: { projectId: string; threadId: string } | null
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

  const shown =
    entries?.filter((entry) => !reply?.hidden.has(entry.ordinal) && !silent(entry)) ?? []
  const operational = label === undefined ? null : describeLabel(label)
  const answered = answeredNotes(shown, operations ?? [], models)
  const forkAt = forkAnchor(entries ?? [], shown)
  const policy = policyOf(harness)
  const turnCards = runningCards(entries ?? [], reply, subagents)
  const modeOf = (entry: ThreadEntry) =>
    operations?.find((op) => op.id === entry.operation_id)?.invocation?.requested_mode ?? null

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
              data-entry-kind={isTool(entry) ? 'tool' : entry.kind}
              className="group space-y-1"
            >
              <Entry
                entry={entry}
                policy={policy}
                requestedMode={modeOf(entry)}
                projectId={projectId}
                onPointAt={onPointAt}
                onOpenSubagent={onOpenSubagent}
              />
              {answered.has(entry.id) && (
                <p className="text-xs text-faint-foreground">{answered.get(entry.id)}</p>
              )}
              <div className={entry.kind === 'UserMessage' ? 'flex justify-end' : undefined}>
                <MessageActions
                  text={copyText(entry)}
                  forkPoint={
                    forkFrom !== null && forkAt !== null && entry.id === forkAt.shownId
                      ? { ...forkFrom, entryId: forkAt.entryId }
                      : undefined
                  }
                />
              </div>
            </motion.li>
          ))}
          {reply !== null && (
            <motion.li key="reply" {...appear}>
              <ReplyText text={reply.text} live={reply.live} />
            </motion.li>
          )}
          {turnCards.map((card) => (
            <motion.li key={`subagent-${card.id}`} {...appear} data-entry-kind="subagent">
              <SubagentCardView card={card} onOpen={onOpenSubagent} />
            </motion.li>
          ))}
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

/** The cards drawn after the running turn's reply (§22.4): a subagent's
 * entry the reply hides (it hides each entry of its turn until the turn's
 * text is in the list), then each live card no entry holds yet. */
function runningCards(
  entries: readonly ThreadEntry[],
  reply: ShownReply | null,
  live: readonly SubagentCard[],
): SubagentCard[] {
  const written = new Set<string>()
  const hidden: SubagentCard[] = []
  for (const entry of entries) {
    const card = subagentOf(entry.body)
    if (card === null) continue
    written.add(card.id)
    if (reply?.hidden.has(entry.ordinal) === true) hidden.push(card)
  }
  return [...hidden, ...live.filter((card) => !written.has(card.id))]
}

/** Where Fork shows, and the entry it forks from. The daemon forks only from
 * the thread's last entry (spec §12.9), which may be a line that shows nothing
 * (the tool call behind a plan card): Fork then sits on the last entry shown
 * and forks from the real last one, which holds the same conversation. */
function forkAnchor(
  entries: readonly ThreadEntry[],
  shown: readonly ThreadEntry[],
): { shownId: string; entryId: string } | null {
  const last = entries.at(-1)
  const lastShown = shown.at(-1)
  if (last === undefined || lastShown === undefined) return null
  const after = entries.slice(entries.findIndex((e) => e.id === lastShown.id) + 1)
  return after.every(silent) ? { shownId: lastShown.id, entryId: last.id } : null
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
