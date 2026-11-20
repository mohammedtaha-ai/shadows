// One job: the project's tables as an ER diagram, a list and one opened table.
import { useQuery } from '@tanstack/react-query'
import { Link, useNavigate } from '@tanstack/react-router'
import { useMemo } from 'react'
import { elementsQuery } from '@/api/project-state'
import { ErrorLine } from '@/app/error-line'
import { Button } from '@/components/ui/button'
import { SkeletonContent } from '@/components/ui/skeleton'
import { tableDiagram } from './diagram-layout'
import { ElementCanvas } from './element-canvas'
import { ElementPanel } from './element-panel'
import { StateBadge } from './state-badge'

export function TablesView({ projectId, element, removed }: { projectId: string; element?: string; removed: boolean }) {
  const elements = useQuery(elementsQuery(projectId))
  const navigate = useNavigate()
  const all = elements.data
  const tables = useMemo(() => (all ?? []).filter(view => view.element.content.kind === 'table'), [all])
  const shown = useMemo(
    () => tables.filter(view => removed || view.state.state !== 'removed'),
    [tables, removed],
  )
  const diagram = useMemo(() => tableDiagram(shown), [shown])
  if (elements.isPending) return <SkeletonContent label="Loading tables…" layout="graph" />
  if (elements.isError) return <ErrorLine error={elements.error} />
  if (tables.length === 0) {
    return <p className="text-sm text-muted-foreground">No tables yet. The Planner writes them when a plan needs them.</p>
  }
  const names = (id: string) => all?.find(view => view.element.id === id)?.element.content.name
  const open = (id?: string) => void navigate({
    to: '/projects/$projectId/state', params: { projectId },
    search: { view: 'tables', element: id, removed: removed ? true : undefined },
  })
  const opened = element === undefined ? undefined : tables.find(view => view.element.id === element)
  const hasRemoved = tables.some(view => view.state.state === 'removed')
  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
      <div className="min-w-0 space-y-3">
        <ElementCanvas label="ER diagram" nodes={diagram.nodes} edges={diagram.edges} selected={element}
          onOpen={id => open(id)} />
        <nav aria-label="Tables list">
          <ul className="space-y-1 text-sm">
            {shown.map(({ element: table, state }) => (
              <li key={table.id} className="flex flex-wrap items-center gap-2">
                <StateBadge state={state.state} />
                <Link to="/projects/$projectId/state" params={{ projectId }}
                  search={{ view: 'tables', element: table.id, removed: removed ? true : undefined }}
                  aria-current={table.id === element ? 'page' : undefined} className="text-accent-line hover:underline">
                  {table.id} <span dir="auto">{table.content.name}</span>
                </Link>
              </li>
            ))}
          </ul>
        </nav>
        {hasRemoved && (
          <Button variant="outline" size="sm" onClick={() => void navigate({
            to: '/projects/$projectId/state', params: { projectId },
            search: { view: 'tables', element, removed: removed ? undefined : true },
          })}>
            {removed ? 'Hide removed' : 'Show removed'}
          </Button>
        )}
      </div>
      <div className="min-w-0">
        {opened !== undefined ? (
          <ElementPanel key={opened.element.id} projectId={projectId} view={opened} names={names} onClose={() => open()} />
        ) : element !== undefined ? (
          <p className="text-sm text-muted-foreground">No table {element} in this project.</p>
        ) : null}
      </div>
    </div>
  )
}
