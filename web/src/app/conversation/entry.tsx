// One job: how one durable thread entry reads in the conversation — a
// message, a subagent's card, a plan card, or a system line. Tool calls are
// drawn as groups by `ToolGroup`.

import { CircleCheck, ShieldX } from 'lucide-react'
import { Suspense, lazy } from 'react'
import { type Plan, type PlanTask } from '@/api/workflows'
import { attachmentUrl, type ThreadEntry } from '@/api/threads'
import { type HarnessPolicy, modeLabel } from '@/api/harnesses'
import { PlanCard } from './plan-card'
import { SubagentCardView } from './subagent-card'
import { VisionProposalCard } from './vision-proposal-card'
import { MapProposalCard } from './map-proposal-card'
import { RoadmapProposalCard } from './roadmap-proposal-card'
import { subagentOf } from './tool-text'

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
  onOpenSubagent: (id: string) => void
}) {
  if (entry.kind === 'UserMessage') {
    return (
      <div className="flex justify-end">
        <div
          dir="auto"
          className="max-w-[80%] rounded-2xl rounded-br-md border border-border bg-card px-4 py-2.5 text-start text-sm whitespace-pre-wrap text-card-foreground [unicode-bidi:plaintext]"
        >
          {entry.attachments.length > 0 && <div className="flex flex-wrap gap-2">
            {entry.attachments.map(image => <a key={image.id} href={attachmentUrl(entry.thread_id, image.id)} target="_blank" rel="noreferrer">
              <img src={attachmentUrl(entry.thread_id, image.id)} alt={image.name ?? 'Attached image'} loading="lazy" className="max-h-48 max-w-64 rounded object-contain" />
            </a>)}
          </div>}
          <p>{entry.body}</p>
        </div>
      </div>
    )
  }
  if (entry.kind === 'AgentMessage') return <ReplyText text={entry.body} />
  if (entry.kind === 'VisionProposal') return <VisionProposalCard entry={entry} projectId={projectId} />
  if (entry.kind === 'MapProposal') return <MapProposalCard entry={entry} projectId={projectId} />
  if (entry.kind === 'RoadmapProposal') return <RoadmapProposalCard entry={entry} projectId={projectId} />
  if (entry.kind === 'Subagent') {
    const card = subagentOf(entry)
    return card === null ? null : <SubagentCardView card={card} onOpen={onOpenSubagent} />
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
    // so without a record it is the initial mode. The unattended mode
    // does not ask.
    const refusedIn = requestedMode ?? policy.initial_mode
    return (
      <p className="flex items-start gap-2 text-xs text-muted-foreground">
        <ShieldX aria-label="Permission refused" className="mt-px size-3.5 shrink-0" />
        <span>
          <code className="font-mono break-all whitespace-pre-wrap">{entry.body}</code>
          {refusedIn !== null && <> — refused in {modeLabel(policy, refusedIn)}</>}
          {policy.unattended_mode !== null && refusedIn !== policy.unattended_mode && (
            <>. {modeLabel(policy, policy.unattended_mode)} would allow it.</>
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
