// One job: the conversation's messages — the durable entries in order, then
// whatever the running turn is saying.

import { AnimatePresence, motion } from 'motion/react'
import { CircleCheck, LoaderCircle, ShieldX, Wrench } from 'lucide-react'
import { Suspense, lazy, useEffect, useRef, useState } from 'react'
import type { Choice, Operation, Plan, PlanTask, ThreadEntry } from '@/api/client'
import { type HarnessPolicy, modeLabel, policyOf } from '../mode-policy'
import { MessageActions } from './message-actions'
import { describeLabel } from './operational-label'
import { PlanCard } from './plan-card'
import type { ShownReply } from './reply'
import { toolText, toolTitle } from './tool-text'
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
  harness,
  forkFrom,
  projectId,
  onPointAt,
}: {
  entries: readonly ThreadEntry[] | undefined
  projectId: string
  /** A task was clicked in a plan card. */
  onPointAt: (plan: Plan, task: PlanTask) => void
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
  const lastId = entries?.at(-1)?.id
  const policy = policyOf(harness)
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
              />
              {answered.has(entry.id) && (
                <p className="text-xs text-faint-foreground">{answered.get(entry.id)}</p>
              )}
              <div className={entry.kind === 'UserMessage' ? 'flex justify-end' : undefined}>
                <MessageActions
                  text={copyText(entry)}
                  forkPoint={
                    forkFrom !== null && entry.id === lastId
                      ? { ...forkFrom, entryId: entry.id }
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

const isTool = (entry: ThreadEntry) =>
  entry.kind === 'AgentMessage' && toolTitle(entry.body) !== null

/** An entry that adds no line: a tool call whose result is another entry. */
function silent(entry: ThreadEntry): boolean {
  return isTool(entry) && toolText(toolTitle(entry.body) ?? '') === null
}

/** What Copy puts on the clipboard: the text as it reads, not its wrapping. */
function copyText(entry: ThreadEntry): string {
  const tool = toolTitle(entry.body)
  return tool === null ? entry.body : (toolText(tool) ?? tool)
}

function Entry({
  entry,
  policy,
  requestedMode,
  projectId,
  onPointAt,
}: {
  entry: ThreadEntry
  policy: HarnessPolicy
  /** The mode the entry's turn asked for, when known. */
  requestedMode: string | null
  projectId: string
  onPointAt: (plan: Plan, task: PlanTask) => void
}) {
  if (entry.kind === 'UserMessage') {
    return (
      <div className="flex justify-end">
        <p
          dir="auto"
          className="max-w-[80%] rounded-2xl rounded-br-md border border-border bg-card px-4 py-2.5 text-sm whitespace-pre-wrap text-card-foreground"
        >
          {entry.body}
        </p>
      </div>
    )
  }
  if (entry.kind === 'AgentMessage') {
    const tool = toolTitle(entry.body)
    if (tool === null) return <ReplyText text={entry.body} />
    // Shadows' own tools read as sentences; any other shows its title.
    const text = toolText(tool) ?? tool
    return (
      <p className="flex items-start gap-2 text-xs text-muted-foreground">
        <Wrench aria-label="Tool" className="mt-px size-3.5 shrink-0" />
        {text === tool ? (
          <code dir="auto" className="font-mono break-all whitespace-pre-wrap">
            {tool}
          </code>
        ) : (
          <span dir="auto">{text}</span>
        )}
      </p>
    )
  }
  if (entry.kind === 'PlanView') {
    return <PlanCard entry={entry} projectId={projectId} onPointAt={onPointAt} />
  }
  if (entry.kind === 'PlanApproved') {
    return (
      <p className="flex items-center gap-2 text-xs text-faint-foreground">
        <CircleCheck aria-hidden className="size-3.5 shrink-0 text-accent-line" />
        <span dir="auto">{entry.body}</span>
      </p>
    )
  }
  if (entry.kind === 'PermissionRefused') {
    // Refused in the mode the turn ran in; only a mode that asks refuses,
    // so without a record it is the policy's first. The unattended mode
    // does not ask (spec §12.2).
    const refusedIn = requestedMode ?? policy.initial
    return (
      <p className="flex items-start gap-2 text-xs text-muted-foreground">
        <ShieldX aria-label="Permission refused" className="mt-px size-3.5 shrink-0" />
        <span>
          <code className="font-mono break-all whitespace-pre-wrap">{entry.body}</code>
          {refusedIn !== null && <> — refused in {modeLabel(policy, refusedIn)}</>}
          {policy.unattended !== null && refusedIn !== policy.unattended && (
            <>. {modeLabel(policy, policy.unattended)} would allow it.</>
          )}
        </span>
      </p>
    )
  }
  return (
    <p dir="auto" className="text-xs text-faint-foreground">
      {entry.body}
    </p>
  )
}
