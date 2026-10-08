// One job: how one durable thread entry reads in the conversation — a
// message, a tool call, a subagent's card, a plan card, or a system line.

import { CircleCheck, ShieldX, Wrench } from 'lucide-react'
import { Suspense, lazy } from 'react'
import type { Plan, PlanTask, ThreadEntry } from '@/api/client'
import { type HarnessPolicy, modeLabel } from '../mode-policy'
import { PlanCard } from './plan-card'
import { SubagentCardView } from './subagent-card'
import { subagentOf, toolText } from './tool-text'

// Streamdown and its code highlighting are most of this app's weight, so they
// are a chunk of their own, and not in the one every screen waits for. The
// load starts when this module does, so by the time a reply arrives it has
// usually landed; until it has, the reply shows as plain text.
const loadReplyText = () => import('./reply-text')
const LazyReplyText = lazy(async () => ({ default: (await loadReplyText()).ReplyText }))
void loadReplyText()

/** A reply's markdown, as plain text until its renderer's chunk lands. */
export function ReplyText({ text, live = false }: { text: string; live?: boolean }) {
  return (
    <Suspense
      fallback={
        <p
          dir="auto"
          className="font-serif text-[15px] leading-7 text-start whitespace-pre-wrap text-foreground [unicode-bidi:plaintext]"
        >
          {text}
        </p>
      }
    >
      <LazyReplyText text={text} live={live} />
    </Suspense>
  )
}

export function Entry({
  entry,
  policy,
  requestedMode,
  projectId,
  onPointAt,
  onOpenSubagent,
}: {
  entry: ThreadEntry
  policy: HarnessPolicy
  /** The mode the entry's turn asked for, when known. */
  requestedMode: string | null
  projectId: string
  onPointAt: (plan: Plan, task: PlanTask) => void
  /** A subagent's card was clicked (§22.4). */
  onOpenSubagent: (id: string) => void
}) {
  if (entry.kind === 'UserMessage') {
    return (
      <div className="flex justify-end">
        <p
          dir="auto"
          className="max-w-[80%] rounded-2xl rounded-br-md border border-border bg-card px-4 py-2.5 text-start text-sm whitespace-pre-wrap text-card-foreground [unicode-bidi:plaintext]"
        >
          {entry.body}
        </p>
      </div>
    )
  }
  if (entry.kind === 'AgentMessage') return <ReplyText text={entry.body} />
  if (entry.kind === 'Subagent') {
    const card = subagentOf(entry)
    return card === null ? null : <SubagentCardView card={card} onOpen={onOpenSubagent} />
  }
  if (entry.kind === 'ToolCall') {
    const tool = entry.body
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
