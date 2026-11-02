import { Circle, FileCode2, FilePenLine, X } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Plan, PlanTask } from '@/api/workflows'
import { Button } from '@/components/ui/button'
import { Planned } from '@/components/ui/planned'
import { parentName } from './linked-layout'
import { BindingEditor } from '@/app/agreements'

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
    waits.filter((l) => l.waiting_items?.includes(item)).map((l) => parentName(l.after, plan.linked_tasks))

  return (
    <aside
      aria-label={`T${task.number}`}
      className="flex w-80 shrink-0 flex-col overflow-y-auto border-l border-border bg-sidebar/95 px-4 py-3 text-sm shadow-lg backdrop-blur-xl"
    >
      <header className="mb-3 flex items-start justify-between gap-2 border-b border-border/50 pb-2.5">
        <div className="min-w-0">
          <span dir="auto" className="inline-block rounded bg-accent-line/15 px-1.5 py-0.5 font-mono text-2xs font-semibold text-accent-line">
            T{task.number}
          </span>
          <h2 dir="auto" className="mt-1 font-semibold leading-snug">
            {task.title}
          </h2>
        </div>
        <Button variant="ghost" size="icon-sm" aria-label="Close" onClick={onClose} className="rounded-full hover:bg-sidebar-accent">
          <X className="size-4" />
        </Button>
      </header>
      <Section name="Goal">
        <p dir="auto" className="text-muted-foreground leading-relaxed">
          {task.goal}
        </p>
      </Section>
      <BindingEditor key={`${plan.id}:${task.number}`} plan={plan} task={task.number} />
      <Section name="Reads" badge={task.reads.length}>
        <Paths paths={task.reads} kind="read" />
      </Section>
      <Section name="Writes" badge={task.writes.length}>
        <Paths paths={task.writes} kind="write" />
      </Section>
      <Section name="Acceptance" badge={task.acceptance.length}>
        {task.acceptance.length > 0 && (
          <Planned
            title="Acceptance progress"
            reason="Waits for the shared request: saved acceptance ticks per task."
            className="mb-2.5"
          >
            <div className="h-1.5 w-full rounded-full bg-muted ring-1 ring-accent-line/20" />
          </Planned>
        )}
        <ol className="space-y-1.5">
          {task.acceptance.map((item) => {
            const after = waitingFor(item.number)
            return (
              <li key={item.number} className="flex gap-2.5 rounded-xl border border-accent-line/20 bg-card/60 p-2.5">
                <Circle aria-hidden className="mt-0.5 size-3.5 shrink-0 text-accent-line/60" />
                <span dir="auto" className="shrink-0 font-mono text-xs text-faint-foreground mt-0.5">
                  {item.number}.
                </span>
                <span className="min-w-0">
                  <span dir="auto" className="block text-xs leading-relaxed text-foreground">
                    {item.text}
                  </span>
                  {after.length > 0 && (
                    <span dir="auto" className="mt-1 block text-2xs text-warning italic">
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

function Section({ name, badge, children }: { name: string; badge?: number; children: ReactNode }) {
  return (
    <section className="mb-4">
      <div className="mb-1.5 flex items-center justify-between">
        <h3 className="text-2xs font-semibold tracking-wider text-faint-foreground uppercase">{name}</h3>
        {badge !== undefined && (
          <span className="rounded bg-accent-line/15 px-1.5 py-0.5 font-mono text-2xs text-accent-line">{badge}</span>
        )}
      </div>
      {children}
    </section>
  )
}

function Paths({ paths, kind }: { paths: string[]; kind: 'read' | 'write' }) {
  if (paths.length === 0) return <p className="text-xs text-faint-foreground">None</p>
  const Icon = kind === 'read' ? FileCode2 : FilePenLine
  return (
    <ul className="space-y-1.5">
      {paths.map((p) => (
        <li
          key={p}
          dir="auto"
          className="flex items-center gap-1.5 truncate rounded-lg border border-accent-line/25 bg-accent-line/10 px-2 py-1 font-mono text-xs text-secondary-foreground"
          title={p}
        >
          <Icon aria-hidden className="size-3 shrink-0 opacity-75" />
          <span className="truncate">{p}</span>
        </li>
      ))}
    </ul>
  )
}
