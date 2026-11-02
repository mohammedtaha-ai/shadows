// Opens from the figures already held, with no daemon request; the full breakdown is the /context slash command.

import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import type { LimitWindow, Limits } from '@/stream'
import { type ContextFigures, contextShown, formatTokens, resetIn } from './usage'

export function ContextRing({
  usage,
  limits,
}: {
  usage: ContextFigures | null
  limits: Limits | null
}) {
  const shown = usage === null ? null : contextShown(usage.contextUsed, usage.contextWindow)
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
        render={<Button variant="ghost" size="icon-xs" aria-label="Context and limits" className="hover:bg-accent/25" />}
      >
        <Ring percent={shown?.percent ?? null} />
      </PopoverTrigger>
      <PopoverContent
        side="top"
        align="end"
        className="surface-glow w-72 space-y-3.5 rounded-xl p-4 text-xs"
      >
        <section className="space-y-1.5">
          <h2 className="text-xs font-semibold tracking-wide text-foreground">Context</h2>
          <p className="font-mono text-muted-foreground tabular-nums">
            {shown === null
              ? 'No figures yet'
              : `${formatTokens(shown.used)} / ${formatTokens(shown.window)} (${shown.percent}%)`}
          </p>
          {shown !== null && (
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="h-full rounded-full bg-gradient-to-r from-accent to-accent-line"
                style={{ width: `${Math.min(shown.percent, 100)}%` }}
              />
            </div>
          )}
        </section>
        {limits === null ? (
          <p className="rounded-lg border border-border/50 bg-background/40 px-2.5 py-1.5 text-muted-foreground">
            Limits: no figures yet
          </p>
        ) : (
          <section className="space-y-2.5 rounded-lg border border-border/50 bg-background/40 p-2.5">
            <Bar name="5-hour limit" window={limits.fiveHour} now={now} />
            <Bar name="Weekly" window={limits.sevenDay} now={now} />
            <p className="text-faint-foreground">updated {clock(limits.observedAt)}</p>
          </section>
        )}
        <p className="border-t border-border/70 pt-2.5 text-2xs text-muted-foreground">
          Type /context in the conversation for the full breakdown.
        </p>
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
        <span className="font-medium text-foreground">{name}</span>
        <span className="font-mono text-muted-foreground tabular-nums">{percent}%</span>
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
          className={
            percent >= 90
              ? 'h-full bg-destructive'
              : 'h-full bg-gradient-to-r from-accent to-accent-line'
          }
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
