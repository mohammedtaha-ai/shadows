// One job: one version's writer and reason as one line (§16.3, §16.8).

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import type { HarnessInfo, WrittenBy } from '@/api/client'
import { harnessesQuery } from '@/api/queries'

function cliLabel(harness: string | null | undefined, harnesses?: HarnessInfo[]): string | null {
  if (!harness) return null
  const found = harnesses?.find((h) => h.kind === harness)
  if (found) return found.label
  if (harness === 'claude-code') return 'Claude Code'
  if (harness === 'codex') return 'Codex'
  return harness
}


function formatReason(version: number, changeReason: string | null | undefined): string | null {
  if (version <= 1) return null
  if (changeReason && changeReason.trim() !== '') return changeReason
  return 'Reason not recorded'
}

export function VersionLine({
  version,
  writtenBy,
  changeReason,
  projectId,
}: {
  version: number
  writtenBy: WrittenBy
  changeReason?: string | null
  projectId?: string
}) {
  const { data: harnesses } = useQuery(harnessesQuery)
  const reason = formatReason(version, changeReason)

  if (writtenBy.kind === 'external') {
    return (
      <div className="space-y-1">
        <p className="text-xs text-muted-foreground">v{version} · External agent</p>
        {reason !== null && (
          <p className="inline-block rounded-md border border-border/40 bg-accent-softer/60 px-2 py-0.5 text-xs text-faint-foreground">
            {reason}
          </p>
        )}
      </div>
    )
  }

  const conversationTitle = writtenBy.thread_removed
    ? `${writtenBy.thread_title} (deleted)`
    : writtenBy.thread_title

  const conversationNode = projectId ? (
    <Link
      to="/projects/$projectId/threads/$threadId"
      params={{ projectId, threadId: writtenBy.thread_id }}
      className="text-foreground/90 underline decoration-muted-foreground/40 underline-offset-2 transition-colors hover:text-foreground hover:decoration-foreground"
    >
      {conversationTitle}
    </Link>
  ) : (
    conversationTitle
  )

  const cli = cliLabel(writtenBy.harness, harnesses)

  return (
    <div className="space-y-1">
      <p className="text-xs text-muted-foreground">
        v{version} · from {conversationNode}
        {writtenBy.model ? ` · ${writtenBy.model}` : ''}
        {cli ? ` · ${cli}` : ''}
      </p>
      {reason !== null && (
        <p className="inline-block rounded-md border border-border/40 bg-accent-softer/60 px-2 py-0.5 text-xs text-faint-foreground">
          {reason}
        </p>
      )}
    </div>
  )
}
