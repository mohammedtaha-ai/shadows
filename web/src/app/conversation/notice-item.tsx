/* eslint-disable react/only-export-components -- The notice's name and title stay beside the item that draws them; the waiting list and the turn box read the same words. */
// One job: an executor's notice drawn as an item of the Planner's activity,
// the look of a tool group, with its report under a header that opens and closes.

import { cn } from 'cn'
import { Bell, ChevronRight } from 'lucide-react'
import { Suspense, lazy, useState } from 'react'
import type { AgentNotice, AgentReplyNotice, AttemptNotice, RecheckNotice } from '@/api/runs'
import type { ThreadEntry } from '@/api/threads'
import { ConflictNotice } from './conflict-notice'
import { RecheckNoticeItem } from './recheck-notice'

const LazyReplyText = lazy(async () => ({ default: (await import('./reply-text')).ReplyText }))

const LONG_LINES = 16
const LONG_CHARS = 1200

/** Who sent it: the executor, by the model it ran on. */
export function noticeName(card: AgentNotice | null): string {
  return card === null ? 'Executor' : 'message' in card ? 'Task agent' : `Executor (${card.model})`
}

/** The outcome as the header says it; a Stop is the person's own act. */
function outcomeWord(outcome: AttemptNotice['outcome']): string {
  return outcome === 'stopped' ? 'stopped by you' : outcome
}

/** The header's line, from the card alone. */
export function noticeTitle(card: AgentNotice | null): string {
  if (card === null) return 'Executor'
  if ('message' in card) return `${card.reason != null ? 'No reply' : 'Reply'} from task agent · T${card.task}`
  return `${noticeName(card)} · T${card.task} · attempt ${card.attempt} ${outcomeWord(card.outcome)}`
}

function AgentReplyItem({ card }: { card: AgentReplyNotice }) {
  const [open, setOpen] = useState(true)
  return <section aria-label="Task agent reply" className="surface-panel rounded-xl p-3">
    <button type="button" aria-expanded={open} onClick={() => setOpen(!open)}
      className="flex w-full cursor-pointer items-center gap-2 text-start text-xs font-medium">
      <ChevronRight aria-hidden className={cn('size-4 text-accent-line', open && 'rotate-90')} />
      {noticeTitle(card)}
      <span dir="auto" className="truncate font-normal text-muted-foreground">{card.title}</span>
    </button>
    {open && <div className="mt-2.5 space-y-2 border-t border-border/60 pt-2.5 text-sm">
      <p className="text-xs font-medium text-muted-foreground">Planner's question</p>
      <p dir="auto" className="whitespace-pre-wrap break-words">{card.message}</p>
      {card.reply != null && <><p className="text-xs font-medium text-muted-foreground">Agent's reply</p>
        <p dir="auto" className="whitespace-pre-wrap break-words">{card.reply}</p></>}
      {card.reason != null && <p dir="auto" className="whitespace-pre-wrap break-words text-destructive">{card.reason}</p>}
    </div>}
  </section>
}

/** The files an interrupted or stopped attempt left; Core's list, drawn as given. */
function LeftFiles({ files }: { files: string[] | null }) {
  return <div className="space-y-1.5 text-xs">
    {files === null ? <p className="text-muted-foreground">Uncommitted files: unknown</p>
      : files.length === 0 ? <p className="text-muted-foreground">No uncommitted files</p>
        : <>
          <p className="font-medium text-muted-foreground">Uncommitted files</p>
          <ul className="font-mono text-2xs text-foreground">
            {files.map(file => <li key={file} dir="auto" className="break-all">{file}</li>)}
          </ul>
        </>}
    <p className="text-muted-foreground">A new Start continues from these files.</p>
  </div>
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

/** A notice from an executor, or from Shadows itself. `body` is shown only when the card is missing or unreadable. */
export function NoticeItem({ body, card, recheck = null, projectId = '', sender = 'Executor' }: {
  body: string
  card: AgentNotice | null
  recheck?: RecheckNotice | null
  projectId?: string
  sender?: 'Executor' | 'Shadows'
}) {
  const [open, setOpen] = useState(true)
  if (recheck !== null) return <RecheckNoticeItem recheck={recheck} projectId={projectId} body={body} />
  if (card !== null && 'message' in card) return <AgentReplyItem card={card} />
  const conflict = card?.conflict ?? null
  const left = card !== null && (card.outcome === 'interrupted' || card.outcome === 'stopped')
  const hasBody = card === null || left || card.report != null || card.reason != null || conflict !== null
  const header = (
    <>
      <Bell aria-hidden className="size-3.5 shrink-0 text-accent-line/85" />
      <span dir="auto" className={cn('font-medium text-foreground')}>
        {card === null ? sender : (
          <>
            {noticeName(card)} · T{card.task} · attempt {card.attempt}{' '}
            <span className={cn(card.outcome === 'failed' && 'text-destructive', card.outcome === 'interrupted' && 'text-warning',
              card.outcome === 'stopped' && 'text-muted-foreground')}>{outcomeWord(card.outcome)}</span>
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
              {left && <LeftFiles files={card.left_files ?? null} />}
              {conflict !== null && <ConflictNotice conflict={conflict} attemptId={card.attempt_id} />}
            </>
          )}
        </div>
      )}
    </div>
  )
}

/** The paths the Planner changed in its turn, with its reason, or that it gave none. */
export function ProjectChangeLine({ entry }: { entry: ThreadEntry }) {
  const paths = Array.isArray(entry.card?.paths)
    ? entry.card.paths.filter((path): path is string => typeof path === 'string')
    : []
  const reason = typeof entry.card?.reason === 'string' && entry.card.reason !== '' ? entry.card.reason : null
  return (
    <section aria-label="Project change" className="surface-panel space-y-1.5 rounded-xl p-3 text-xs">
      <p className="font-medium text-secondary-foreground">Changed project files</p>
      <ul className="space-y-0.5 font-mono text-2xs text-muted-foreground">
        {paths.map(path => <li key={path} className="break-all">{path}</li>)}
      </ul>
      <p dir="auto" className={cn('text-foreground', reason === null && 'text-faint-foreground')}>
        {reason ?? 'no reason given'}
      </p>
    </section>
  )
}
