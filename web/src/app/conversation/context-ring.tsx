// One job: the context ring at the end of the composer bar and the two levels
// it opens (spec §12.8): the summary from figures the client already holds,
// and the breakdown, read from the daemon only when asked for.
//
// It never waits: it opens at once, on hover or click, with the last figures
// and when they were observed, or says there are none. The breakdown is the
// one thing fetched, only when Details is pressed, never on a timer, and
// while it is on its way the level says "Reading…" — a word, not a spinner.

import { useMutation } from '@tanstack/react-query'
import { useState } from 'react'
import { readContext } from '@/api/client'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import type { LimitWindow, Limits } from '@/stream/frames'
import { ErrorLine } from '../error-line'
import { type ContextFigures, contextShown, formatTokens, resetIn } from './usage'

export function ContextRing({
  threadId,
  usage,
  limits,
}: {
  threadId: string
  usage: ContextFigures | null
  limits: Limits | null
}) {
  const shown = usage === null ? null : contextShown(usage.contextUsed, usage.contextWindow)
  const breakdown = useMutation({ mutationFn: () => readContext(threadId) })
  // Reset times count from the moment the ring opened (Unix seconds).
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000))

  return (
    <Popover
      onOpenChange={(open) => {
        if (open) setNow(Math.floor(Date.now() / 1000))
      }}
    >
      <PopoverTrigger
        openOnHover
        delay={300}
        render={<Button variant="ghost" size="icon-xs" aria-label="Context and limits" />}
      >
        <Ring percent={shown?.percent ?? null} />
      </PopoverTrigger>
      <PopoverContent side="top" align="end" className="space-y-3 text-xs">
        <section className="space-y-1">
          <h2 className="font-medium text-foreground">Context</h2>
          <p className="text-muted-foreground">
            {shown === null
              ? 'No figures yet'
              : `${formatTokens(shown.used)} / ${formatTokens(shown.window)} (${shown.percent}%)`}
          </p>
        </section>
        {limits === null ? (
          <p className="text-muted-foreground">Limits: no figures yet</p>
        ) : (
          <section className="space-y-2">
            <Bar name="5-hour limit" window={limits.fiveHour} now={now} />
            <Bar name="Weekly" window={limits.sevenDay} now={now} />
            <p className="text-faint-foreground">updated {clock(limits.observedAt)}</p>
          </section>
        )}
        <div className="border-t border-border pt-2">
          <Button variant="ghost" size="xs" onClick={() => breakdown.mutate()}>
            Details
          </Button>
          {breakdown.isPending && <p className="px-2 text-muted-foreground">Reading…</p>}
          {breakdown.error !== null && <ErrorLine error={breakdown.error} />}
          {breakdown.data?.categories != null && (
            <ul className="space-y-0.5 px-2 pt-1">
              {breakdown.data.categories.map((c) => (
                <li key={c.name} className="flex justify-between gap-3">
                  <span>{c.name}</span>
                  <span className="text-muted-foreground tabular-nums">
                    {formatTokens(c.tokens)} · {c.percent}%
                  </span>
                </li>
              ))}
            </ul>
          )}
          {breakdown.data !== undefined && breakdown.data.categories === null && (
            <p className="px-2 pt-1 text-muted-foreground">
              {breakdown.data.reason ?? 'No breakdown.'}
            </p>
          )}
        </div>
      </PopoverContent>
    </Popover>
  )
}

/** One limit window: its use as a bar, and when it resets. */
function Bar({ name, window, now }: { name: string; window: LimitWindow | null; now: number }) {
  if (window === null) {
    return <p className="text-muted-foreground">{name}: not reported</p>
  }
  const percent = Math.round(window.utilization * 100)
  return (
    <div className="space-y-1">
      <div className="flex justify-between gap-3">
        <span className="text-foreground">{name}</span>
        <span className="text-muted-foreground tabular-nums">{percent}%</span>
      </div>
      <div
        role="progressbar"
        aria-label={name}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        className="h-1.5 overflow-hidden rounded-full bg-muted"
      >
        <div
          className={percent >= 90 ? 'h-full bg-destructive' : 'h-full bg-accent-line'}
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>
      <p className="text-faint-foreground">
        Resets in {resetIn(window.resetsAt, now)}
      </p>
    </div>
  )
}

/** A ring filled to `percent`, or an empty dashed one when there is no figure. */
function Ring({ percent }: { percent: number | null }) {
  const r = 6
  const around = 2 * Math.PI * r
  return (
    <svg viewBox="0 0 16 16" className="size-4 -rotate-90" aria-hidden>
      <circle
        cx="8"
        cy="8"
        r={r}
        fill="none"
        strokeWidth="2"
        className="stroke-muted"
        strokeDasharray={percent === null ? '2 2' : undefined}
      />
      {percent !== null && (
        <circle
          cx="8"
          cy="8"
          r={r}
          fill="none"
          strokeWidth="2"
          className="stroke-accent-line"
          strokeDasharray={`${(Math.min(percent, 100) / 100) * around} ${around}`}
        />
      )}
    </svg>
  )
}

function clock(iso: string): string {
  const at = new Date(iso)
  return Number.isNaN(at.getTime())
    ? iso
    : at.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}
