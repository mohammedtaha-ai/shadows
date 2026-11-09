import type { PlanTask } from '@/api/workflows'

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

/** The task as planned, and the model Start will use. The critic row waits for slice 2. */
export function DetailsTab({ task, model, onModel }: {
  task: PlanTask
  model: string
  onModel: (model: string) => void
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
      <label className="block text-xs">
        <Heading name="Executor model" />
        <input value={model} onChange={(e) => { onModel(e.target.value) }} aria-label="Executor model"
          className="w-72 rounded border border-border bg-transparent px-2 py-1 font-mono text-xs" />
      </label>
    </div>
  )
}
