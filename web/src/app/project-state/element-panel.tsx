// One job: one table or screen opened in full: its state, its part, its tasks and its revisions.
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { elementRevisionsQuery, projectStateQuery, type ElementTask, type ElementView } from '@/api/project-state'
import { ElementContent } from '@/app/element-content'
import { ErrorLine } from '@/app/error-line'
import { Button } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { SkeletonContent } from '@/components/ui/skeleton'
import { StateBadge } from './state-badge'

const link = 'text-accent-line hover:underline'

function TaskLines({ projectId, tasks }: { projectId: string; tasks: ElementTask[] }) {
  if (tasks.length === 0) return <p className="text-muted-foreground">No task yet.</p>
  return (
    <ul className="space-y-0.5">
      {tasks.map((task, index) => (
        <li key={`${task.workflow_id}:${task.task}:${index}`} dir="auto">
          <Link to="/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task"
            params={{ projectId, workflowId: task.workflow_id, version: String(task.version), task: String(task.task) }}
            className={link}>
            T{task.task} · v{task.version} · r{task.revision} · {task.action} · {task.file}
            {task.symbol_id != null ? ` · ${task.symbol_id}` : ''}
            {task.symbol_state != null ? ` (${task.symbol_state})` : ''}
          </Link>
        </li>
      ))}
    </ul>
  )
}

export function ElementPanel({ projectId, view, names, onClose }: {
  projectId: string
  view: ElementView
  names: (id: string) => string | undefined
  onClose: () => void
}) {
  const { element, state } = view
  const parts = useQuery(projectStateQuery(projectId))
  const revisions = useQuery(elementRevisionsQuery(projectId, element.id))
  const partTitle = parts.data?.parts.find(part => part.part_id === element.part_id)?.title ?? element.part_id
  const all = revisions.data ?? []
  const built = state.built_revision === null ? undefined : all.find(item => item.revision === state.built_revision)
  const latest = all.find(item => item.revision === element.revision)
  return (
    <aside aria-label={`Element ${element.id}`}>
      <Panel className="space-y-3 p-4 text-xs">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 dir="auto" className="text-sm font-semibold">{element.id} · {element.content.kind} · {element.content.name}</h2>
          <StateBadge state={state.state} />
          <Button variant="outline" size="sm" onClick={onClose}>Close</Button>
        </div>
        {state.why != null && <p dir="auto" className="text-muted-foreground">{state.why}</p>}
        <p>
          Part:{' '}
          <Link to="/projects/$projectId/state" params={{ projectId }} search={{ view: 'state', part: element.part_id }}
            dir="auto" className={link}>
            {partTitle}
          </Link>
        </p>
        {element.outcomes.length > 0 && (
          <p className="flex flex-wrap gap-x-2">
            Outcomes:
            {element.outcomes.map(id => (
              <Link key={id} to="/projects/$projectId/workspace" params={{ projectId }}
                search={{ view: 'roadmap', outcome: id }} className={link}>
                {id}
              </Link>
            ))}
          </p>
        )}
        {state.confirmed === 'file_and_symbol' && <p>File and symbol confirmed</p>}
        {state.confirmed === 'file_only' && <p>File confirmed</p>}
        <div className="space-y-1">
          <h3 className="font-medium">Built by</h3>
          <TaskLines projectId={projectId} tasks={state.done} />
        </div>
        <div className="space-y-1">
          <h3 className="font-medium">Planned in</h3>
          <TaskLines projectId={projectId} tasks={state.planned} />
        </div>
        <div className="space-y-2">
          <h3 className="font-medium">Revisions</h3>
          {revisions.isPending && <SkeletonContent label="Loading revisions…" layout="detail" />}
          {revisions.isError && <ErrorLine error={revisions.error} />}
          {revisions.isSuccess && (
            <>
              {state.behind && built !== undefined && latest !== undefined && (
                <>
                  <section aria-label="Built revision" className="space-y-1 rounded-lg border border-border p-2">
                    <h4 className="font-medium">Built (r{built.revision})</h4>
                    <ElementContent element={built} names={names} />
                  </section>
                  <section aria-label="Planned change" className="space-y-1 rounded-lg border border-border p-2">
                    <h4 className="font-medium">Planned change (r{latest.revision})</h4>
                    <ElementContent element={latest} names={names} />
                  </section>
                </>
              )}
              {all.map(item => (
                <details key={item.revision} className="rounded-lg border border-border p-2">
                  <summary className="cursor-pointer">
                    r{item.revision} · {item.created_at}{item.deleted ? ' · deleted' : ''}
                  </summary>
                  <div className="mt-2"><ElementContent element={item} names={names} /></div>
                </details>
              ))}
            </>
          )}
        </div>
      </Panel>
    </aside>
  )
}
