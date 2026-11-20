// One job: the project's screens as a flow diagram, a list and one opened screen.
import { ElementsView } from './elements-view'

export function ScreensView({ projectId, element, removed }: { projectId: string; element?: string; removed: boolean }) {
  return <ElementsView kind="screen" projectId={projectId} element={element} removed={removed} />
}
