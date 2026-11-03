import { useQueries, useQuery, useQueryClient } from '@tanstack/react-query'
import { useParams } from '@tanstack/react-router'
import { useEffect } from 'react'
import { agentsQuery } from '@/api/aylul-profiles'
import { groupMessagesQuery, groupQuery, type Group } from '@/api/groups'
import { operationsQuery } from '@/api/threads'
import { colorOf } from '@/app/agents'
import { holdThreadRefresh, useProjectEvents } from '@/stream'
import { ErrorLine } from '../error-line'
import { GroupComposer } from './group-composer'
import { GroupLog } from './group-log'

export function GroupPage() {
  const { groupId } = useParams({ from: '/aylul/groups/$groupId' })
  const group = useQuery(groupQuery(groupId))
  if (group.error) return <div className="p-6"><ErrorLine error={group.error} /></div>
  if (!group.data) return <p role="status" className="p-6">Loading group…</p>
  return <OpenGroup key={groupId} group={group.data} />
}

function OpenGroup({ group }: { group: Group }) {
  const agents = useQuery(agentsQuery)
  const log = useQuery(groupMessagesQuery(group.id))
  const client = useQueryClient()
  const projectId = agents.data?.project_id
  const operations = useQueries({ queries: group.members.map(member => operationsQuery(member.thread_id)) })
  useProjectEvents(projectId)
  const members = group.members
  useEffect(() => {
    const release = members.map(member => holdThreadRefresh(client, member.thread_id, { projectId }))
    return () => release.forEach(stop => stop())
  }, [client, members, projectId])
  const live = agents.data?.agents.filter(agent => !agent.deleted) ?? []
  const colors = new Map(live.map(agent => [agent.slug, colorOf(agent.character_color).swatch]))
  const mentionable = members.filter(member => colors.has(member.slug))
  const thinking = members.filter((_member, index) => {
    const status = operations[index].data?.[0]?.status_kind
    return status === 'Pending' || status === 'Running'
  })
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="border-b border-border px-5 py-4">
        <h1 dir="auto" className="text-lg font-semibold">{group.name}</h1>
        {group.needs_you && <p className="text-sm text-warning">Needs you</p>}
        {thinking.map(member => <p key={member.slug} role="status" className="text-sm text-muted-foreground">{member.name} is thinking</p>)}
      </header>
      {log.error && <div className="px-5 pt-3"><ErrorLine error={log.error} /></div>}
      <GroupLog messages={log.data ?? []} colors={colors} />
      <GroupComposer groupId={group.id} members={mentionable} />
    </div>
  )
}
