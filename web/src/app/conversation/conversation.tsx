// One job: the open conversation's pane — header, messages, composer.

import { useQuery } from '@tanstack/react-query'
import { getRouteApi } from '@tanstack/react-router'
import type { Project } from '@/api/client'
import { projectsQuery, threadsQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { Composer } from './composer'
import { Messages } from './messages'
import { StatusBadge } from './status-badge'
import { StreamBanner } from './stream-banner'
import { useConversation } from './use-conversation'

const route = getRouteApi('/projects/$projectId/threads/$threadId')

/** The route's component. Keyed by thread, so switching conversations starts
 * the next one's state from nothing rather than from the last one's. */
export function ConversationRoute() {
  const { projectId, threadId } = route.useParams()
  const project = useQuery(projectsQuery).data?.find((p) => p.id === projectId)
  const thread = useQuery(threadsQuery(projectId)).data?.find((t) => t.id === threadId)
  return (
    <Conversation
      key={threadId}
      threadId={threadId}
      title={thread?.title ?? 'Conversation'}
      project={project}
    />
  )
}

function Conversation({
  threadId,
  title,
  project,
}: {
  threadId: string
  title: string
  project: Project | undefined
}) {
  const c = useConversation(threadId)

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex items-center justify-between gap-4 border-b border-border px-6 py-3">
        <div className="min-w-0">
          <h1 className="truncate text-sm font-medium">{title}</h1>
          <p className="truncate text-xs text-faint-foreground">
            {project?.name ?? '…'} · Planner
          </p>
        </div>
        <StatusBadge running={c.running} latest={c.latest} />
      </header>
      <StreamBanner
        connection={c.stream.connection}
        problem={c.stream.problem}
        retry={c.stream.retry}
      />
      {c.entries.isError && (
        <div className="px-6 pt-3">
          <ErrorLine error={c.entries.error} />
        </div>
      )}
      <Messages
        entries={c.entries.data}
        loading={!c.stream.caughtUp}
        reply={c.reply}
        running={c.running !== null}
        thinking={c.thinking}
        label={c.label}
      />
      <Composer
        threadId={threadId}
        running={c.running}
        directory={project?.directory}
        known={c.known}
        settings={null}
        onStarted={c.started}
      />
    </div>
  )
}
