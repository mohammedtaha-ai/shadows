// One job: the project's tables as an ER diagram, a list and one opened table.
import { ElementsView } from './elements-view'

export function TablesView({ projectId, element, removed }: { projectId: string; element?: string; removed: boolean }) {
  return <ElementsView kind="table" projectId={projectId} element={element} removed={removed} />
}
