// One job: an agent's task list drawn as a checklist.

import { Circle, CircleCheck, LoaderCircle } from 'lucide-react'
import type { TodoItem } from '@/stream'

const MARK = {
  completed: { icon: CircleCheck, mark: 'text-accent-line', text: 'text-muted-foreground' },
  in_progress: { icon: LoaderCircle, mark: 'animate-spin motion-reduce:animate-none', text: 'text-foreground' },
  pending: { icon: Circle, mark: 'text-faint-foreground', text: 'text-foreground' },
} as const

export function TodoCard({ items }: { items: readonly TodoItem[] }) {
  const done = items.filter((item) => item.status === 'completed').length
  return (
    <section data-todo-card className="max-w-xl rounded-lg border border-border bg-card px-3 py-2.5">
      <h3 className="text-xs font-medium text-muted-foreground">
        Tasks · {done} of {items.length} done
      </h3>
      <ul className="mt-2 space-y-1.5">
        {items.map((item, index) => {
          const look = MARK[item.status]
          return (
            <li key={index} data-status={item.status} className="flex items-start gap-2 text-sm">
              <look.icon aria-hidden className={`mt-0.5 size-4 shrink-0 ${look.mark}`} />
              <span dir="auto" className={look.text}>{item.content}</span>
            </li>
          )
        })}
      </ul>
    </section>
  )
}
