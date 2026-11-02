// One job: a run of tool calls drawn as one line that opens into numbered
// steps; a lone call is drawn as its own line.

import { useLocation } from '@tanstack/react-router'
import { Ban, CheckCircle2, ChevronRight, LoaderCircle, Terminal, X } from 'lucide-react'
import { memo, useEffect, useState } from 'react'
import type { ToolStep } from './activity-rows'
import { groupSummary, stepLine } from './activity-text'
import { MessageActions } from './message-actions'
import { duration } from './subagent-text'
import { ToolDiff } from './tool-diff'

const running = (step: ToolStep) => step.status === 'pending' || step.status === 'in_progress'

/** The clock, ticking once a second while `active`. */
function useElapsed(active: boolean): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    if (!active) return
    const timer = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(timer)
  }, [active])
  return now
}

const at = (time: string | null) => {
  if (time === null) return null
  const ms = Date.parse(time)
  return Number.isNaN(ms) ? null : ms
}

/** How long a step took, or has taken; `null` without times. */
function stepTime(step: ToolStep, now: number): string | null {
  const start = at(step.startedAt)
  if (start === null) return null
  if (running(step)) return duration(Math.max(0, now - start))
  const end = at(step.finishedAt)
  return end === null ? null : duration(Math.max(0, end - start))
}

function groupTime(steps: readonly ToolStep[], busy: boolean, now: number): string | null {
  const first = at(steps[0]?.startedAt ?? null)
  if (first === null) return null
  if (busy) return duration(Math.max(0, now - first))
  const last = at(steps.at(-1)?.finishedAt ?? null)
  return last === null ? null : duration(Math.max(0, last - first))
}

const CLAMPED_LINES = 6
const CLAMPED_CHARS = 600

/** A failed step's error as Core sent it, shortened to six lines past that. */
function ErrorText({ text, indent }: { text: string; indent: string }) {
  const long = text.split('\n').length > CLAMPED_LINES || text.length > CLAMPED_CHARS
  const [all, setAll] = useState(false)
  return (
    <div className={`mt-1.5 ${indent}`}>
      <pre
        dir="auto"
        className={`rounded-lg border border-destructive-border/60 bg-destructive/10 px-3 py-2 font-mono text-xs break-words whitespace-pre-wrap text-destructive-foreground ${long && !all ? 'line-clamp-6' : ''}`}
      >
        {text}
      </pre>
      {long && (
        <button
          type="button"
          onClick={() => setAll(!all)}
          className="mt-1 text-xs font-medium text-accent-line underline underline-offset-2"
        >
          {all ? 'Show less' : 'Show all'}
        </button>
      )}
    </div>
  )
}

function StatusMark({ step }: { step: ToolStep }) {
  if (running(step)) {
    return (
      <LoaderCircle
        aria-label="Running"
        className="mt-px size-3.5 shrink-0 animate-spin text-accent-line motion-reduce:animate-none"
      />
    )
  }
  if (step.status === 'failed') {
    return <X aria-label="Failed" className="mt-px size-3.5 shrink-0 text-destructive" />
  }
  if (step.status === 'stopped') {
    return <Ban aria-label="Stopped" className="mt-px size-3.5 shrink-0 text-faint-foreground" />
  }
  return <CheckCircle2 aria-hidden className="mt-px size-3.5 shrink-0 text-success/90" />
}

function StepLine({
  step,
  number,
  now,
}: {
  step: ToolStep
  number: number | null
  now: number
}) {
  const line = stepLine(step)
  const time = stepTime(step, now)
  const [shown, setShown] = useState(false)
  const drawable =
    step.kind === 'edit' &&
    ((step.diff?.length ?? 0) > 0 || step.added !== null || step.removed !== null)
  const isBusy = running(step)
  return (
    <div
      id={step.ordinal === null || number === null ? undefined : `entry-${step.ordinal}`}
      className={`rounded-lg px-2 py-1 transition-colors ${
        isBusy ? 'border border-accent-line/35 bg-accent/15' : 'hover:bg-muted/40'
      }`}
    >
      <p className="flex items-start gap-2 text-xs text-muted-foreground">
        {/* Every numbered step keeps the chevron's column, so the numbers line up. */}
        {!drawable && number !== null && <span aria-hidden className="size-3.5 shrink-0" />}
        {drawable && (
          <button
            type="button"
            aria-expanded={shown}
            aria-label="Show diff"
            onClick={() => setShown(!shown)}
            className="shrink-0 rounded text-accent-line transition-colors hover:bg-accent/25 hover:text-foreground"
          >
            <ChevronRight className={`size-3.5 transition-transform ${shown ? 'rotate-90' : ''}`} />
          </button>
        )}
        {number !== null && (
          <span className="w-4 shrink-0 font-mono text-end text-faint-foreground tabular-nums">
            {number}.
          </span>
        )}
        <StatusMark step={step} />
        <span dir="auto" className="min-w-0 break-words text-foreground/90">
          {line.text}
          {line.counts !== null && (
            <span className="ms-2 rounded-md border border-accent-line/30 bg-accent-softer/90 px-1.5 py-0.5 font-mono text-2xs font-medium text-accent-line tabular-nums">
              {line.counts}
            </span>
          )}
          {step.status === 'failed' && (
            <span className="ms-1.5 font-medium text-destructive">failed</span>
          )}
        </span>
        {time !== null && (
          <span className="ms-auto shrink-0 font-mono text-2xs text-faint-foreground tabular-nums">
            {time}
          </span>
        )}
      </p>
      {step.status === 'failed' && step.error !== null && step.error !== '' && (
        <ErrorText
          text={step.error}
          indent={number !== null ? 'ps-16' : drawable ? 'ps-11' : 'ps-6'}
        />
      )}
      {drawable && shown && <ToolDiff diff={step.diff} />}
    </div>
  )
}

/** What Copy puts on the clipboard: each step's line, one per line. */
const copyOf = (steps: readonly ToolStep[]) => steps.map((step) => stepLine(step).text).join('\n')

export const ToolGroup = memo(function ToolGroup({
  steps,
  ordinals,
  forkProjectId,
  forkThreadId,
  forkEntryId,
}: {
  steps: readonly ToolStep[]
  /** The members' ordinals joined: a stable key for a link and for memo. */
  ordinals: string
  forkProjectId?: string
  forkThreadId?: string
  forkEntryId?: string
}) {
  const busy = steps.some(running)
  const now = useElapsed(busy)
  const [chosen, setChosen] = useState<boolean | null>(null)
  const open = chosen ?? busy
  const hash = useLocation().hash.replace(/^#/, '')

  // A link to a member's entry opens its group and lands on that step.
  const linked = ordinals.split(',').some((ordinal) => hash === `entry-${ordinal}`)
  const [seen, setSeen] = useState(false)
  if (linked !== seen) {
    setSeen(linked)
    if (linked) setChosen(true)
  }
  useEffect(() => {
    if (!linked || steps.length < 2) return
    const frame = requestAnimationFrame(() =>
      document.getElementById(hash)?.scrollIntoView({ block: 'center' }),
    )
    return () => cancelAnimationFrame(frame)
  }, [linked, hash, steps.length])

  const forkPoint =
    forkProjectId === undefined || forkThreadId === undefined || forkEntryId === undefined
      ? undefined
      : { projectId: forkProjectId, threadId: forkThreadId, entryId: forkEntryId }

  const only = steps.length === 1 ? steps[0] : undefined
  const failed = steps.filter((step) => step.status === 'failed').length
  const time = groupTime(steps, busy, now)

  return (
    <>
      {only !== undefined ? (
        <div
          id={only.ordinal === null ? undefined : `entry-${only.ordinal}`}
          className="surface-panel rounded-xl px-3 py-2"
        >
          <StepLine step={{ ...only, ordinal: null }} number={null} now={now} />
        </div>
      ) : (
        <div className="surface-panel rounded-xl p-3">
          <button
            type="button"
            aria-expanded={open}
            onClick={() => setChosen(!open)}
            className="flex w-full items-center gap-2.5 text-start text-xs font-medium text-secondary-foreground transition-colors hover:text-foreground"
          >
            <ChevronRight
              className={`size-4 shrink-0 text-accent-line transition-transform duration-150 ${open ? 'rotate-90' : ''}`}
            />
            {busy ? (
              <LoaderCircle
                aria-label="Running"
                className="size-3.5 shrink-0 animate-spin text-accent-line motion-reduce:animate-none"
              />
            ) : (
              <Terminal aria-hidden className="size-3.5 shrink-0 text-accent-line/85" />
            )}
            <span dir="auto" className="font-medium text-foreground">
              {groupSummary(steps)}
            </span>
            {failed > 0 && <span className="font-medium text-destructive">· {failed} failed</span>}
            {time !== null && (
              <span className="ms-auto rounded-full border border-border/70 bg-background/60 px-2 py-0.5 font-mono text-2xs text-muted-foreground tabular-nums">
                {time}
              </span>
            )}
          </button>
          {open && (
            <ol className="mt-2.5 space-y-1 border-t border-border/60 pt-2.5">
              {steps.map((step, index) => (
                <li key={step.key}>
                  <StepLine step={step} number={index + 1} now={now} />
                </li>
              ))}
            </ol>
          )}
        </div>
      )}
      {!busy && (
        <div>
          <MessageActions text={copyOf(steps)} forkPoint={forkPoint} />
        </div>
      )}
    </>
  )
})
