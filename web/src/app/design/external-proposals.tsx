import { useQuery } from '@tanstack/react-query'
import {
  mapProposalsQuery, roadmapProposalsQuery, visionProposalsQuery,
  type MapProposal, type RoadmapProposal, type VisionProposal,
} from '@/api/design'
import { connectionsQuery } from '@/api/external-connections'
import { ErrorLine } from '../error-line'
import { MapProposalCard } from './map-proposal-card'
import { RoadmapProposalCard } from './roadmap-proposal-card'
import { VisionProposalCard } from './vision-proposal-card'

type Kind = 'vision' | 'map' | 'roadmap'

// Core's list holds each proposal once; this section shows the ones no conversation owns.
function useExternal(projectId: string, kind: Kind) {
  const vision = useQuery({ ...visionProposalsQuery(projectId), enabled: kind === 'vision' })
  const map = useQuery({ ...mapProposalsQuery(projectId), enabled: kind === 'map' })
  const roadmap = useQuery({ ...roadmapProposalsQuery(projectId), enabled: kind === 'roadmap' })
  const read = kind === 'vision' ? vision : kind === 'map' ? map : roadmap
  const rows: (VisionProposal | MapProposal | RoadmapProposal)[] = read.data ?? []
  return { error: read.error, rows: rows.filter(p => p.thread_id === null && p.status === 'Pending' && !p.outdated) }
}

export function ExternalProposals({ projectId, kind }: { projectId: string; kind: Kind }) {
  const { error, rows } = useExternal(projectId, kind)
  const connections = useQuery(connectionsQuery)
  if (rows.length === 0) return error ? <ErrorLine error={error} /> : null
  return <section aria-label="From external agents" className="space-y-3">
    <h3 className="text-sm font-medium">From external agents</h3>
    {rows.map(p => {
      const name = connections.data?.find(c => c.id === p.connection_id)?.name
      return <div key={p.id} className="space-y-2">
        <p className="text-sm text-muted-foreground">From <bdi>{name ?? 'an external agent'}</bdi></p>
        {kind === 'vision' && <VisionProposalCard proposal={p as VisionProposal} projectId={projectId} />}
        {kind === 'map' && <MapProposalCard proposal={p as MapProposal} projectId={projectId} />}
        {kind === 'roadmap' && <RoadmapProposalCard proposal={p as RoadmapProposal} projectId={projectId} />}
      </div>
    })}
  </section>
}
