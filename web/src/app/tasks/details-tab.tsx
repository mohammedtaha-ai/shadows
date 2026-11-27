import { Link } from '@tanstack/react-router'
import type { PlanDelivery, PlanTask } from '@/api/workflows'
import type { ReactNode } from 'react'

type Scope = PlanDelivery['tasks'][number]['ownership']['reads'][number]

function Heading({ name }: { name: string }) {
  return <h3 className="mb-1 text-2xs font-semibold tracking-wider text-faint-foreground uppercase">{name}</h3>
}

/** One access mode's paths; each shows the parts Core says own it, or that none does. */
function Paths({ name, paths, scopes, projectId, titles }: {
  name: string; paths: string[]; scopes: Scope[] | undefined; projectId: string; titles: Map<string, string>
}) {
  return (
    <section className="mb-4">
      <Heading name={name} />
      {paths.length === 0 ? <p className="text-xs text-faint-foreground">None</p> : (
        <ul className="space-y-1">
          {paths.map((p) => {
            const owners = scopes?.find((s) => s.path === p)?.owners
            return (
              <li key={p} className="text-xs">
                <span className="font-mono">{p}</span>
                {owners !== undefined && (owners.length === 0
                  ? <span className="ml-2 text-faint-foreground">unmapped</span>
                  : owners.map((o) => (
                    <Link key={o.part_id} to="/projects/$projectId/workspace" params={{ projectId }}
                      search={{ view: 'map', part: o.part_id }} dir="auto" className="ml-2 underline">
                      {titles.get(o.part_id) ?? o.part_id}
                    </Link>
                  )))}
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}

export function DetailsTab({ task, executor, delivery, projectId }: {
  task: PlanTask
  executor: ReactNode
  delivery?: PlanDelivery | null
  projectId: string
}) {
  const owned = delivery?.tasks.find((t) => t.number === task.number)?.ownership
  const titles = new Map([...(delivery?.parts ?? []), ...(delivery?.task_parts ?? [])].map((p) => [p.part_id, p.title]))
  return (
    <div className="max-w-2xl px-6 py-4 text-sm">
      <section className="mb-4">
        <Heading name="Goal" />
        <p dir="auto" className="text-muted-foreground">{task.goal}</p>
      </section>
      {delivery != null && delivery.outcomes.length > 0 && (
        <section className="mb-4" aria-label="Outcomes served">
          <Heading name="Outcomes" />
          <ul className="space-y-1">
            {delivery.outcomes.map((o) => (
              <li key={o.outcome_id} className="text-xs">
                <Link to="/projects/$projectId/workspace" params={{ projectId }}
                  search={{ view: 'roadmap', outcome: o.outcome_id }} dir="auto" className="underline">{o.title}</Link>
              </li>
            ))}
          </ul>
        </section>
      )}
      <Paths name="Writes" paths={task.writes} scopes={owned?.writes} projectId={projectId} titles={titles} />
      <Paths name="Reads" paths={task.reads} scopes={owned?.reads} projectId={projectId} titles={titles} />
      <section className="mb-4">
        <Heading name="Acceptance" />
        <ol className="space-y-1">
          {task.acceptance.map((item) => <li key={item.number} dir="auto" className="text-xs">{item.number}. {item.text}</li>)}
        </ol>
      </section>
      {executor}
    </div>
  )
}
