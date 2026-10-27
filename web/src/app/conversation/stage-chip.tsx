import { useQuery } from '@tanstack/react-query'
import { stageQuery } from '@/api/design'
import { projectsQuery } from '@/api/projects'

export function StageChip({ projectId }: { projectId: string }) {
  const assistant = useQuery(projectsQuery).data?.find((p) => p.id === projectId)?.assistant
  const stage = useQuery({ ...stageQuery(projectId), enabled: assistant !== true })
  if (assistant === true) return null
  if (stage.isPending || stage.isError || stage.data === undefined) return null
  return <p className="text-xs text-muted-foreground">
    Stage: {stage.data.stage}{stage.data.missing.length > 0 && <> · missing <bdi dir="auto">{stage.data.missing.join(', ')}</bdi></>}
  </p>
}
