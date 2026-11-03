import { useQuery } from '@tanstack/react-query'
import { Link, useNavigate } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { useState } from 'react'
import { agentsQuery } from '@/api/aylul-profiles'
import { groupsQuery } from '@/api/groups'
import { ColorDot } from '@/app/agents'
import { NewGroupDialog } from '@/app/groups'
import { Button } from '@/components/ui/button'
import { useProjectEvents } from '@/stream'
import { ErrorLine } from '../error-line'

export function GroupList({ filter }: { filter: string }) {
  const agents = useQuery(agentsQuery)
  const groups = useQuery({ ...groupsQuery, enabled: agents.isSuccess })
  const [open, setOpen] = useState(false)
  const navigate = useNavigate()
  useProjectEvents(agents.data?.project_id)
  const list = groups.data?.filter(group => group.name.toLowerCase().includes(filter.trim().toLowerCase()))
  const live = agents.data?.agents.filter(agent => !agent.deleted && agent.slug !== 'mohammed') ?? []
  return (
    <section aria-label="Agent groups" className="mb-4 space-y-1 border-b border-sidebar-border pb-3">
      <div className="flex items-center justify-between px-2">
        <h2 className="text-xs font-semibold text-muted-foreground">Groups</h2>
        <Button type="button" size="icon" variant="ghost" aria-label="New group"
          disabled={live.length === 0} onClick={() => setOpen(true)}><Plus aria-hidden /></Button>
      </div>
      {groups.error && <ErrorLine error={groups.error} />}
      {list?.map(group => (
        <Link key={group.id} to="/aylul/groups/$groupId" params={{ groupId: group.id }}
          className="flex items-center gap-2 rounded-lg px-2 py-1.5 text-sm hover:bg-sidebar-accent aria-[current=page]:bg-sidebar-accent">
          <span dir="auto" className="min-w-0 flex-1 truncate">{group.name}</span>
          {group.members.map(member => {
            const agent = live.find(item => item.slug === member.slug)
            return agent ? <ColorDot key={member.slug} id={agent.character_color} />
              : <span key={member.slug} aria-hidden className="size-2.5 rounded-full bg-muted-foreground" />
          })}
          {group.needs_you && <span role="img" aria-label="Needs you" className="size-2 shrink-0 rounded-full bg-warning" />}
        </Link>
      ))}
      {open && <NewGroupDialog agents={live} onClose={() => setOpen(false)} onCreated={groupId => {
        setOpen(false)
        void navigate({ to: '/aylul/groups/$groupId', params: { groupId } })
      }} />}
    </section>
  )
}
