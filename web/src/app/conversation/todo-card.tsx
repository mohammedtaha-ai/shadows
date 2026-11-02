// One job: an agent's task list drawn as a checklist.

import { Circle, CircleCheck, LoaderCircle } from 'lucide-react'
import type { TodoItem } from '@/stream'

const MARK = {
  completed: {
    icon: CircleCheck,
    mark: 'text-success',
    text: 'text-muted-foreground line-through decoration-muted-foreground/40',
    row: 'border-transparent bg-background/30',
  },
  in_progress: {
    icon: LoaderCircle,
    mark: 'animate-spin text-accent-line motion-reduce:animate-none',
    text: 'font-medium text-foreground',
    row: 'border-accent-line/45 bg-accent/20',
  },
  pending: {
    icon: Circle,
    mark: 'text-faint-foreground',
    text: 'text-foreground/90',
    row: 'border-transparent bg-background/20',
  },
} as const

export function TodoCard({ items }: { items: readonly TodoItem[] }) {
  const done = items.filter((item) => item.status === 'completed').length
  const percent = items.length > 0 ? Math.round((done / items.length) * 100) : 0
  return (
    <section data-todo-card className="surface-glow max-w-xl rounded-xl px-4 py-3.5">
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-xs font-semibold tracking-wide text-secondary-foreground">
          Tasks · {done} of {items.length} done
        </h3>
        <span className="font-mono text-xs font-semibold text-accent-line tabular-nums">
          {percent}%
        </span>
      </div>
      <div aria-hidden className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-background/70">
        <div
          className="h-full rounded-full bg-gradient-to-r from-accent to-accent-line transition-all duration-300"
          style={{ width: `${percent}%` }}
        />
      </div>
      <ul className="mt-3 space-y-1.5">
        {items.map((item, index) => {
          const look = MARK[item.status]
          return (
            <li
              key={index}
              data-status={item.status}
              className={`flex items-start gap-2.5 rounded-lg border px-2.5 py-1.5 text-sm transition-colors ${look.row}`}
            >
              <look.icon aria-hidden className={`mt-0.5 size-4 shrink-0 ${look.mark}`} />
              <span dir="auto" className={look.text}>{item.content}</span>
            </li>
          )
        })}
      </ul>
    </section>
  )
}
