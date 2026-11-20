// The project's state page: its tabs over one project, read only.
import { Link, useParams, useSearch } from '@tanstack/react-router'
import { cn } from 'cn'
import { Activity } from 'lucide-react'
import { Panel } from '@/components/ui/panel'
import { ScreensView } from './screens-view'
import { StateView } from './state-view'
import { TablesView } from './tables-view'

const tab = (active: boolean) =>
  cn(
    'rounded-lg px-3 py-1.5 transition-all',
    active ? 'pill-active' : 'text-muted-foreground hover:bg-accent-line/10 hover:text-foreground',
  )

export function ProjectStatePage() {
  const { projectId } = useParams({ from: '/projects/$projectId/state' })
  const { view, part, element, removed } = useSearch({ from: '/projects/$projectId/state' })
  const activeView = view ?? 'state'
  return (
    <main className="flex-1 overflow-y-auto bg-background p-6">
      <div className="mx-auto max-w-6xl space-y-6">
        <Panel className="flex flex-wrap items-center justify-between gap-4 px-5 py-4">
          <div className="flex items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-xl border border-accent-line/35 bg-accent-line/15">
              <Activity aria-hidden className="size-4 text-accent-line" />
            </div>
            <h1 className="text-lg font-semibold tracking-tight text-foreground">Project state</h1>
          </div>
          <nav
            className="flex flex-wrap items-center gap-1.5 rounded-xl border border-border bg-input-background p-1 text-xs font-medium"
            aria-label="Project state sections"
          >
            <Link to="/projects/$projectId/workspace" params={{ projectId }} className={tab(false)}>
              Workspace
            </Link>
            <Link
              to="/projects/$projectId/state"
              params={{ projectId }}
              search={{ view: 'state' }}
              className={tab(activeView === 'state')}
            >
              State
            </Link>
            <Link
              to="/projects/$projectId/state"
              params={{ projectId }}
              search={{ view: 'tables' }}
              className={tab(activeView === 'tables')}
            >
              Tables
            </Link>
            <Link
              to="/projects/$projectId/state"
              params={{ projectId }}
              search={{ view: 'screens' }}
              className={tab(activeView === 'screens')}
            >
              Screens
            </Link>
          </nav>
        </Panel>
        {view === 'tables' ? (
          <TablesView key={projectId} projectId={projectId} element={element} removed={removed === true} />
        ) : view === 'screens' ? (
          <ScreensView key={projectId} projectId={projectId} element={element} removed={removed === true} />
        ) : (
          <StateView key={projectId} projectId={projectId} part={part} removed={removed === true} />
        )}
      </div>
    </main>
  )
}
