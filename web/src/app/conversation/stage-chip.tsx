// One job: the project's stage (§23.4) as one line in the conversation header.
import { useQuery } from '@tanstack/react-query'
import { stageQuery } from '@/api/queries'

export function StageChip({ projectId }: { projectId: string }) {
  const stage = useQuery(stageQuery(projectId))
  if (stage.isPending || stage.isError || stage.data === undefined) return null
  return <p className="text-xs text-muted-foreground">
    Stage: {stage.data.stage}{stage.data.missing.length > 0 && <> · missing <bdi dir="auto">{stage.data.missing.join(', ')}</bdi></>}
  </p>
}
