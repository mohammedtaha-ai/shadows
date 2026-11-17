import type { PlanTask } from '@/api/workflows'
import type { ReactNode } from 'react'

function Heading({ name }: { name: string }) {
  return <h3 className="mb-1 text-2xs font-semibold tracking-wider text-faint-foreground uppercase">{name}</h3>
}

function Paths({ name, paths }: { name: string; paths: string[] }) {
  return (
    <section className="mb-4">
      <Heading name={name} />
      {paths.length === 0 ? <p className="text-xs text-faint-foreground">None</p> : (
        <ul className="space-y-1">
          {paths.map((p) => <li key={p} className="font-mono text-xs">{p}</li>)}
        </ul>
      )}
    </section>
  )
}

export function DetailsTab({ task, executor }: {
  task: PlanTask
  executor: ReactNode
}) {
  return (
    <div className="max-w-2xl px-6 py-4 text-sm">
      <section className="mb-4">
        <Heading name="Goal" />
        <p dir="auto" className="text-muted-foreground">{task.goal}</p>
      </section>
      <Paths name="Writes" paths={task.writes} />
      <Paths name="Reads" paths={task.reads} />
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
