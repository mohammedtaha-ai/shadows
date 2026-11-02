// One job: the conversation's messages — the durable entries in order, then
// whatever the running turn is saying.

import { AnimatePresence, motion } from 'motion/react'
import { LoaderCircle, Sparkles } from 'lucide-react'
import { memo, useEffect, useMemo, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useLocation } from '@tanstack/react-router'
import type { Plan, PlanTask } from '@/api/workflows'
import type { Choice } from '@/api/sessions'
import type { Operation, ThreadEntry } from '@/api/threads'
import { type HarnessPolicy, harnessesQuery, policyOf } from '@/api/harnesses'
import { AYLUL_HARNESS } from './cli-picker'
import { Entry, ReplyText } from './entry'
import { MessageActions } from './message-actions'
import { describeLabel } from './operational-label'
import type { SubagentCard, TodoItem, ToolFrame } from '@/stream'
import { TodoCard } from './todo-card'
import { type Row, activityRows, liveSteps, withLiveSteps } from './activity-rows'
import { ToolGroup } from './tool-group'
import type { Cut, ShownReply } from './reply'
import { SubagentCardView } from './subagent-card'
import { copyText, silent, subagentOf } from './tool-text'
import { labelOf } from './turn-settings'

const NO_ENTRIES: readonly ThreadEntry[] = []
const NO_CUTS: readonly Cut[] = []

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
  tools,
  todos,
}: {
  entries: readonly ThreadEntry[] | undefined
  projectId: string
  /** A task was clicked in a plan card. */
  onPointAt: (plan: Plan, task: PlanTask) => void
  /** The running turns' subagent cards as the stream last sent them. */
  subagents: readonly SubagentCard[]
  onOpenSubagent: (id: string) => void
  /** The running turn's tool calls as the stream last sent them, by call id. */
  tools: Readonly<Record<string, ToolFrame>>
  /** The running turn's task lists as the stream last sent them, by operation. */
  todos: Readonly<Record<string, readonly TodoItem[]>>
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
  const location = useLocation()
  const followed = useRef<string | null>(null)

  // Follow the bottom while the reader is there; leave them be once they
  // scroll up to read.
  useEffect(() => {
    const el = scroller.current
    if (el !== null && pinned.current) el.scrollTop = el.scrollHeight
  })

  useEffect(() => {
    const hash = location.hash.replace(/^#/, '')
    if (!hash) { followed.current = null; return }
    const key = `${location.pathname}#${hash}`
    if (followed.current === key) return
    const target = document.getElementById(hash)
    if (target && scroller.current?.contains(target)) {
      pinned.current = false
      target.scrollIntoView({ block: 'center' })
      followed.current = key
    }
  }, [entries, location.hash, location.pathname])

  // An entry that takes over from the streamed reply is already on screen as
  // that reply's text: it must not fade in again. Recorded while the reply
  // shows, well before the entry replaces it (state adjusted during render,
  // React's pattern for remembering what an earlier render saw).
  const [handedOver, setHandedOver] = useState<ReadonlySet<number>>(() => new Set())
  const newlyHidden = [...(reply?.hidden ?? [])].filter((ordinal) => !handedOver.has(ordinal))
  if (newlyHidden.length > 0) setHandedOver(new Set([...handedOver, ...newlyHidden]))

  // The hidden ordinals as a string, so a delta that changes nothing here
  // leaves the shown entries, and so the rows, the same objects.
  const hiddenKey = [...(reply?.hidden ?? [])].join(',')
  const hidden = useMemo(
    () => new Set(hiddenKey === '' ? [] : hiddenKey.split(',').map(Number)),
    [hiddenKey],
  )
  const shown = useMemo(
    () => (entries ?? NO_ENTRIES).filter((entry) => !hidden.has(entry.ordinal) && !silent(entry)),
    [entries, hidden],
  )
  // The reply's text sits between the entries where it was cut, so a turn's
  // calls stay between the paragraphs they ran between, as after it ends.
  const cuts = reply === null || reply.cuts.length === 0 ? NO_CUTS : reply.cuts
  const written = useMemo(() => activityRows(shown, cuts), [shown, cuts])
  const frames = useMemo(() => Object.values(tools), [tools])
  const turnOp = frames.at(-1)?.op
  const live = useMemo(
    () => (turnOp === undefined ? [] : liveSteps(frames, entries ?? NO_ENTRIES, turnOp, !running)),
    [frames, entries, turnOp, running],
  )
  // The turn's unwritten calls join the written rows.
  const rows = useMemo(() => withLiveSteps(written, live, shown, turnOp), [written, live, shown, turnOp])
  // A list the entries already hold draws as that entry, never twice.
  const liveLists = Object.entries(todos).filter(
    ([op]) => !(entries ?? NO_ENTRIES).some((e) => e.kind === 'TodoList' && e.operation_id === op),
  )
  const operational = label === undefined ? null : describeLabel(label)
  const answered = answeredNotes(shown, operations ?? [], models)
  const forkAt = forkAnchor(entries ?? [], shown)
  const policy = policyOf(useQuery(harnessesQuery).data, harness)
  const turnCards = runningCards(entries ?? [], subagents)
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
      <ol className="mx-auto flex max-w-4xl flex-col gap-6 px-6 py-8">
        {loading && (
          <li className="flex items-center gap-2 rounded-xl border border-accent-line/20 bg-accent/10 px-3.5 py-2 text-xs text-accent-foreground">
            <LoaderCircle className="size-3.5 animate-spin text-accent-line motion-reduce:animate-none" />
            Loading history
          </li>
        )}
        {!loading && shown.length === 0 && reply === null && !running && (
          <li className="surface-panel my-12 flex flex-col items-center justify-center gap-3 rounded-2xl px-6 py-14 text-center">
            <div className="flex size-12 items-center justify-center rounded-2xl border border-accent-line/35 bg-accent/15">
              <Sparkles aria-hidden className="size-5 text-accent-line" />
            </div>
            <p className="text-sm font-medium text-muted-foreground">
              {harness === AYLUL_HARNESS
                ? 'Ask AYLUL anything.'
                : 'Ask the Planner something about this project.'}
            </p>
          </li>
        )}
        <AnimatePresence initial={false}>
          {rows.map((row) => row.kind === 'text' ? (
            <motion.li key={row.key} {...appear}>
              <ReplyText text={row.text} />
            </motion.li>
          ) : row.kind === 'tools' ? (
            <ToolRow
              key={row.key}
              row={row}
              answered={answered.get(row.steps.at(-1)?.key ?? row.key)}
              {...forkProps(row, forkFrom, forkAt)}
            />
          ) : (
            <EntryRow
              key={row.entry.id}
              entry={row.entry}
              handedOver={handedOver.has(row.entry.ordinal)}
              policy={policy}
              requestedMode={modeOf(row.entry)}
              projectId={projectId}
              onPointAt={onPointAt}
              onOpenSubagent={onOpenSubagent}
              answered={answered.get(row.entry.id)}
              {...(forkFrom !== null && forkAt !== null && row.entry.id === forkAt.shownId
                ? { forkProjectId: forkFrom.projectId, forkThreadId: forkFrom.threadId, forkEntryId: forkAt.entryId }
                : {})}
            />
          ))}
          {reply !== null && reply.tail !== '' && (
            <motion.li key="reply" {...appear}>
              <ReplyText text={reply.tail} live={reply.live} />
            </motion.li>
          )}
          {liveLists.map(([op, items]) => (
            <motion.li key={`todos-${op}`} {...appear} data-entry-kind='todos'>
              <TodoCard items={items} />
            </motion.li>
          ))}
          {turnCards.map((card) => (
            <motion.li key={`subagent-${card.id}`} {...appear} data-entry-kind="subagent">
              <SubagentCardView card={card} onOpen={onOpenSubagent} />
            </motion.li>
          ))}
          {thinking && (
            <motion.li key="waiting" {...appear} className="inline-flex w-fit items-center gap-2.5 rounded-full border border-accent-line/30 bg-accent/10 px-3.5 py-1.5">
              <span className="size-2 animate-pulse rounded-full bg-accent-line motion-reduce:animate-none" />
              <span className="text-xs font-medium text-accent-foreground">Thinking</span>
            </motion.li>
          )}
          {running && operational !== null && (
            <motion.li
              key="label"
              {...appear}
              className="inline-flex w-fit items-center gap-2 rounded-full border border-accent-line/20 bg-card px-3 py-1 text-xs text-accent-foreground"
            >
              <operational.icon className="size-3.5 text-accent-line" />
              {operational.text}
            </motion.li>
          )}
        </AnimatePresence>
      </ol>
    </div>
  )
}

/** A run of tool calls with its note. */
function ToolRow({
  row,
  answered,
  forkProjectId,
  forkThreadId,
  forkEntryId,
}: {
  row: Extract<Row, { kind: 'tools' }>
  answered: string | undefined
  forkProjectId?: string
  forkThreadId?: string
  forkEntryId?: string
}) {
  return (
    <motion.li {...appear} data-entry-kind='tool' className='group space-y-1.5'>
      <ToolGroup
        steps={row.steps}
        ordinals={row.steps.flatMap((step) => (step.ordinal === null ? [] : [step.ordinal])).join(',')}
        {...(forkProjectId !== undefined && forkThreadId !== undefined && forkEntryId !== undefined
          ? { forkProjectId, forkThreadId, forkEntryId }
          : {})}
      />
      {answered !== undefined && <p className='text-xs text-faint-foreground'>{answered}</p>}
    </motion.li>
  )
}

/** The fork point a row carries when its last step is the fork anchor. */
function forkProps(
  row: Extract<Row, { kind: 'tools' }>,
  forkFrom: { projectId: string; threadId: string } | null,
  forkAt: { shownId: string; entryId: string } | null,
) {
  return forkFrom !== null && forkAt !== null && row.steps.at(-1)?.key === forkAt.shownId
    ? { forkProjectId: forkFrom.projectId, forkThreadId: forkFrom.threadId, forkEntryId: forkAt.entryId }
    : {}
}

/** One durable entry with its note and actions. Memoised: a streamed delta
 * re-renders the list, and a long thread's entries (markdown, a plan's graph)
 * must not be drawn again for each one. */
const EntryRow = memo(function EntryRow({
  entry,
  handedOver,
  policy,
  requestedMode,
  projectId,
  onPointAt,
  onOpenSubagent,
  answered,
  forkProjectId,
  forkThreadId,
  forkEntryId,
}: {
  entry: ThreadEntry
  /** Already on screen as the streamed reply: it does not fade in again. */
  handedOver: boolean
  policy: HarnessPolicy
  requestedMode: string | null
  projectId: string
  onPointAt: (plan: Plan, task: PlanTask) => void
  onOpenSubagent: (id: string) => void
  answered: string | undefined
  /** Where a fork from this row goes; set on the fork anchor only, as plain
   * values so that a new object each render does not redraw the row. */
  forkProjectId?: string
  forkThreadId?: string
  forkEntryId?: string
}) {
  const forkPoint =
    forkProjectId === undefined || forkThreadId === undefined || forkEntryId === undefined
      ? undefined
      : { projectId: forkProjectId, threadId: forkThreadId, entryId: forkEntryId }
  return (
    <motion.li
      id={`entry-${entry.ordinal}`}
      {...appear}
      initial={handedOver ? false : appear.initial}
      data-entry-kind={entry.kind}
      className="group space-y-1.5"
    >
      <Entry
        entry={entry}
        policy={policy}
        requestedMode={requestedMode}
        projectId={projectId}
        onPointAt={onPointAt}
        onOpenSubagent={onOpenSubagent}
      />
      {answered !== undefined && <p className="text-xs text-faint-foreground">{answered}</p>}
      <div className={entry.kind === 'UserMessage' ? 'flex justify-end' : undefined}>
        <MessageActions text={copyText(entry)} forkPoint={forkPoint} />
      </div>
    </motion.li>
  )
})

/** The running turn's subagents that no entry holds yet; a written one draws
 * as its entry, in place. */
function runningCards(entries: readonly ThreadEntry[], live: readonly SubagentCard[]): SubagentCard[] {
  const written = new Set<string>()
  for (const entry of entries) {
    const card = subagentOf(entry)
    if (card !== null) written.add(card.id)
  }
  return live.filter((card) => !written.has(card.id))
}

/** The daemon requires the real last entry, even if hidden; show Fork on the last visible entry. */
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

/** Attach a model substitution note to the turn's last visible entry so it appears only once. */
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
