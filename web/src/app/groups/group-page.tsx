import { SkeletonContent } from '@/components/ui/skeleton'
import { useQueries, useQuery, useQueryClient } from '@tanstack/react-query'
import { useParams } from '@tanstack/react-router'
import { useEffect } from 'react'
import { agentsQuery } from '@/api/aylul-profiles'
import { groupMessagesQuery, groupQuery, type Group } from '@/api/groups'
import { operationsQuery, type Operation } from '@/api/threads'
import { waitingPermissionsQuery } from '@/api/permissions'
import { colorOf } from '@/app/agents'
import { initialTurnState, PermissionAnswer, runningTurn, turnReducer } from '@/app/conversation'
import { holdThreadRefresh, useProjectEvents } from '@/stream'
import { ErrorLine } from '../error-line'
import { GroupComposer } from './group-composer'
import { GroupLog } from './group-log'

export function GroupPage() {
  const { groupId } = useParams({ from: '/aylul/groups/$groupId' })
  const group = useQuery(groupQuery(groupId))
  if (group.error) return <div className="p-6"><ErrorLine error={group.error} /></div>
  if (!group.data) return <SkeletonContent label="Loading group…" layout="conversation" className="p-6" />
  return <OpenGroup key={groupId} group={group.data} />
}

/** A member thinks while its thread has a turn that has not ended, by the conversation's one rule. */
function running(operations: readonly Operation[] | undefined) {
  const snapshot = { type: 'snapshot', operations: operations ?? [], now: 0 } as const
  return runningTurn(turnReducer(initialTurnState, snapshot)) !== null
}

function OpenGroup({ group }: { group: Group }) {
  const agents = useQuery(agentsQuery)
  const log = useQuery(groupMessagesQuery(group.id))
  const permissions = useQuery(waitingPermissionsQuery)
  const client = useQueryClient()
  const projectId = agents.data?.project_id
  const operations = useQueries({ queries: group.members.map(member => operationsQuery(member.thread_id)) })
  useProjectEvents(projectId)
  const members = group.members
  // Each group refetch makes new member objects; the holds follow the thread ids only.
  const threadIds = JSON.stringify(members.map(member => member.thread_id))
  useEffect(() => {
    const release = (JSON.parse(threadIds) as string[]).map(threadId => holdThreadRefresh(client, threadId, { projectId }))
    return () => release.forEach(stop => stop())
  }, [client, threadIds, projectId])
  const live = agents.data?.agents.filter(agent => !agent.deleted) ?? []
  const colors = new Map(live.map(agent => [agent.slug, colorOf(agent.character_color).swatch]))
  const mentionable = members.filter(member => colors.has(member.slug))
  const thinking = members.filter((_member, index) => running(operations[index].data))
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="border-b border-border px-5 py-4">
        <h1 dir="auto" className="text-lg font-semibold">{group.name}</h1>
        {group.needs_you && <p className="text-sm text-warning">Needs you</p>}
        {thinking.map(member => <p key={member.slug} role="status" className="text-sm text-muted-foreground">{member.name} is thinking</p>)}
      </header>
      {log.error && <div className="px-5 pt-3"><ErrorLine error={log.error} /></div>}
      <GroupLog messages={log.data ?? []} colors={colors} />
      {permissions.error
        ? <div className="px-4"><ErrorLine error={permissions.error} /></div>
        : <section aria-label="Group permission requests" className="max-h-[35%] space-y-2 overflow-y-auto px-4">
          {permissions.data?.map(request => {
            const member = members.find(value => value.thread_id === request.thread_id)
            if (!member) return null
            return <article key={JSON.stringify([request.thread_id, request.request_id])} className="space-y-2 rounded-xl border border-warning/30 p-3">
              <h2 dir="auto" className="text-sm font-semibold">{member.name}</h2>
              <p dir="auto" className="whitespace-pre-wrap text-sm">{request.title}</p>
              <PermissionAnswer request={request} />
            </article>
          })}
        </section>}
      <GroupComposer groupId={group.id} members={mentionable} />
    </div>
  )
}
