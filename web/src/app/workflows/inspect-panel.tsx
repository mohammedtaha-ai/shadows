// One job: one task opened beside the graph — its goal, reads, writes and
// acceptance items, each waiting item saying for which task (§13.11).

import { X } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Plan, PlanTask } from '@/api/client'
import { Button } from '@/components/ui/button'

export function InspectPanel({
  plan,
  task,
  onClose,
}: {
  plan: Plan
  task: PlanTask
  onClose: () => void
}) {
  const waits = plan.links.filter((l) => l.task === task.number && l.kind === 'completes_after')
  const waitingFor = (item: number) =>
    waits.filter((l) => l.waiting_items?.includes(item)).map((l) => `T${l.after}`)

  return (
    <aside
      aria-label={`T${task.number}`}
      className="flex w-80 shrink-0 flex-col overflow-y-auto border-l border-border bg-sidebar px-4 py-3 text-sm"
    >
      <header className="mb-3 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p dir="auto" className="font-mono text-xs text-faint-foreground">
            T{task.number}
          </p>
          <h2 dir="auto" className="font-medium">
            {task.title}
          </h2>
        </div>
        <Button variant="ghost" size="icon-sm" aria-label="Close" onClick={onClose}>
          <X />
        </Button>
      </header>
      <Section name="Goal">
        <p dir="auto" className="text-muted-foreground">
          {task.goal}
        </p>
      </Section>
      <Section name="Reads">
        <Paths paths={task.reads} />
      </Section>
      <Section name="Writes">
        <Paths paths={task.writes} />
      </Section>
      <Section name="Acceptance">
        <ol className="space-y-1.5">
          {task.acceptance.map((item) => {
            const after = waitingFor(item.number)
            return (
              <li key={item.number} className="flex gap-2">
                <span dir="auto" className="font-mono text-xs text-faint-foreground">
                  {item.number}.
                </span>
                <span className="min-w-0">
                  <span dir="auto" className="block">
                    {item.text}
                  </span>
                  {after.length > 0 && (
                    <span dir="auto" className="block text-xs text-muted-foreground italic">
                      waits for {after.join(', ')}
                    </span>
                  )}
                </span>
              </li>
            )
          })}
        </ol>
      </Section>
    </aside>
  )
}

function Section({ name, children }: { name: string; children: ReactNode }) {
  return (
    <section className="mb-4">
      <h3 className="mb-1 text-[11px] tracking-wide text-faint-foreground uppercase">{name}</h3>
      {children}
    </section>
  )
}

function Paths({ paths }: { paths: string[] }) {
  if (paths.length === 0) return <p className="text-xs text-faint-foreground">None</p>
  return (
    <ul className="space-y-0.5">
      {paths.map((p) => (
        <li key={p} dir="auto" className="truncate font-mono text-xs text-muted-foreground" title={p}>
          {p}
        </li>
      ))}
    </ul>
  )
}
