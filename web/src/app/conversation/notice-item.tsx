/* eslint-disable react/only-export-components -- The notice's name and title stay beside the item that draws them; the waiting list and the turn box read the same words. */
// One job: an executor's notice drawn as an item of the Planner's activity,
// the look of a tool group, with its report under a header that opens and closes.

import { cn } from 'cn'
import { Bell, ChevronRight } from 'lucide-react'
import { Suspense, lazy, useState } from 'react'
import type { AttemptNotice } from '@/api/runs'

const LazyReplyText = lazy(async () => ({ default: (await import('./reply-text')).ReplyText }))

const LONG_LINES = 16
const LONG_CHARS = 1200

/** Who sent it: the executor, by the model it ran on. */
export function noticeName(card: AttemptNotice | null): string {
  return card === null ? 'Executor' : `Executor (${card.model})`
}

/** The header's line, from the card alone. */
export function noticeTitle(card: AttemptNotice | null): string {
  if (card === null) return 'Executor'
  return `${noticeName(card)} · T${card.task} · attempt ${card.attempt} ${card.outcome}`
}

function Report({ text }: { text: string }) {
  const long = text.split('\n').length > LONG_LINES || text.length > LONG_CHARS
  const [all, setAll] = useState(false)
  return (
    <div>
      <div className={cn(long && !all && 'max-h-80 overflow-hidden')}>
        <Suspense
          fallback={<p dir="auto" className="text-sm leading-6 whitespace-pre-wrap text-foreground">{text}</p>}
        >
          <LazyReplyText text={text} compact />
        </Suspense>
      </div>
      {long && (
        <button
          type="button"
          onClick={() => setAll(!all)}
          className="mt-1 text-xs font-medium text-accent-line underline underline-offset-2"
        >
          {all ? 'Show less' : 'Show more'}
        </button>
      )}
    </div>
  )
}

/** An executor's notice. `body` is shown only when the card is missing or unreadable. */
export function NoticeItem({ body, card }: { body: string; card: AttemptNotice | null }) {
  const [open, setOpen] = useState(true)
  const hasBody = card === null || card.report != null || card.reason != null
  const header = (
    <>
      <Bell aria-hidden className="size-3.5 shrink-0 text-accent-line/85" />
      <span dir="auto" className={cn('font-medium text-foreground')}>
        {card === null ? 'Executor' : (
          <>
            {noticeName(card)} · T{card.task} · attempt {card.attempt}{' '}
            <span className={cn(card.outcome === 'failed' && 'text-destructive')}>{card.outcome}</span>
          </>
        )}
      </span>
      {card !== null && (
        <>
          <span dir="auto" className="min-w-0 truncate font-normal text-muted-foreground">{card.title}</span>
          {card.last_addition && (
            <span className="shrink-0 rounded-full border border-warning/30 bg-warning/10 px-2 py-0.5 text-2xs text-warning">
              last addition
            </span>
          )}
          {card.commit != null && card.commit !== '' && (
            <span className="ms-auto shrink-0 font-mono text-2xs text-muted-foreground">{card.commit.slice(0, 7)}</span>
          )}
        </>
      )}
    </>
  )
  const lineClass = 'flex w-full items-center gap-2.5 text-start text-xs'
  return (
    <div className="surface-panel rounded-xl p-3">
      {hasBody ? (
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
          className={cn(lineClass, 'text-secondary-foreground transition-colors hover:text-foreground')}
        >
          <ChevronRight
            className={cn('size-4 shrink-0 text-accent-line transition-transform duration-150', open && 'rotate-90')}
          />
          {header}
        </button>
      ) : (
        <div className={lineClass}>{header}</div>
      )}
      {hasBody && open && (
        <div className="mt-2.5 space-y-2 border-t border-border/60 pt-2.5">
          {card === null ? (
            <p dir="auto" className="text-sm whitespace-pre-wrap text-foreground">{body}</p>
          ) : (
            <>
              {card.reason != null && <p dir="auto" className="text-xs text-destructive">{card.reason}</p>}
              {card.report != null && <Report text={card.report} />}
            </>
          )}
        </div>
      )}
    </div>
  )
}
