// One job: the project's tables or screens as a diagram, a list and one opened element.
import { useQuery } from '@tanstack/react-query'
import { Link, useNavigate } from '@tanstack/react-router'
import { useMemo } from 'react'
import { elementsQuery } from '@/api/project-state'
import { ErrorLine } from '@/app/error-line'
import { Button } from '@/components/ui/button'
import { SkeletonContent } from '@/components/ui/skeleton'
import { screenDiagram, tableDiagram } from './diagram-layout'
import { ElementCanvas } from './element-canvas'
import { ElementPanel } from './element-panel'
import { StateBadge } from './state-badge'

const KINDS = {
  table: { view: 'tables', diagram: tableDiagram, canvas: 'ER diagram', list: 'Tables list', loading: 'Loading tables…', noun: 'table', plural: 'tables' },
  screen: { view: 'screens', diagram: screenDiagram, canvas: 'Screen flow', list: 'Screens list', loading: 'Loading screens…', noun: 'screen', plural: 'screens' },
} as const

export function ElementsView({ kind, projectId, element, removed }: {
  kind: 'table' | 'screen'; projectId: string; element?: string; removed: boolean
}) {
  const config = KINDS[kind]
  const elements = useQuery(elementsQuery(projectId))
  const navigate = useNavigate()
  const all = elements.data
  const ofKind = useMemo(() => (all ?? []).filter(view => view.element.content.kind === kind), [all, kind])
  const shown = useMemo(
    () => ofKind.filter(view => removed || view.state.state !== 'removed'),
    [ofKind, removed],
  )
  const diagram = useMemo(() => config.diagram(shown), [config, shown])
  if (elements.isPending) return <SkeletonContent label={config.loading} layout="graph" />
  if (elements.isError) return <ErrorLine error={elements.error} />
  if (ofKind.length === 0) {
    return <p className="text-sm text-muted-foreground">No {config.plural} yet. The Planner writes them when a plan needs them.</p>
  }
  const names = (id: string) => all?.find(view => view.element.id === id)?.element.content.name
  const open = (id?: string) => void navigate({
    to: '/projects/$projectId/state', params: { projectId },
    search: { view: config.view, element: id, removed: removed ? true : undefined },
  })
  const opened = element === undefined ? undefined : ofKind.find(view => view.element.id === element)
  const hasRemoved = ofKind.some(view => view.state.state === 'removed')
  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
      <div className="min-w-0 space-y-3">
        <ElementCanvas label={config.canvas} nodes={diagram.nodes} edges={diagram.edges} selected={element}
          onOpen={id => open(id)} />
        <nav aria-label={config.list}>
          <ul className="space-y-1 text-sm">
            {shown.map(({ element: item, state }) => (
              <li key={item.id} className="flex flex-wrap items-center gap-2">
                <StateBadge state={state.state} />
                <Link to="/projects/$projectId/state" params={{ projectId }}
                  search={{ view: config.view, element: item.id, removed: removed ? true : undefined }}
                  aria-current={item.id === element ? 'page' : undefined} className="text-accent-line hover:underline">
                  {item.id} <span dir="auto">{item.content.name}</span>
                </Link>
              </li>
            ))}
          </ul>
        </nav>
        {hasRemoved && (
          <Button variant="outline" size="sm" onClick={() => void navigate({
            to: '/projects/$projectId/state', params: { projectId },
            search: { view: config.view, element, removed: removed ? undefined : true },
          })}>
            {removed ? 'Hide removed' : 'Show removed'}
          </Button>
        )}
      </div>
      <div className="min-w-0">
        {opened !== undefined ? (
          <ElementPanel key={opened.element.id} projectId={projectId} view={opened} names={names} onClose={() => open()} />
        ) : element !== undefined ? (
          <p className="text-sm text-muted-foreground">No {config.noun} {element} in this project.</p>
        ) : null}
      </div>
    </div>
  )
}
