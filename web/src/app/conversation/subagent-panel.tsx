// One job: what one subagent did, in a panel beside the conversation (spec
// §22.4) — what it was asked, its steps in order, and its report.

import { Wrench, X } from 'lucide-react'
import type { SubagentCard } from '@/stream/frames'
import { ReplyText } from './entry'
import { modelName, numbers } from './subagent-card'

export function SubagentPanel({ card, onClose }: { card: SubagentCard; onClose: () => void }) {
  const said = numbers(card)
  return (
    <aside
      aria-label="Subagent beside the conversation"
      className="flex w-[min(40rem,45%)] min-w-80 shrink-0 flex-col border-l border-border bg-card"
    >
      <header className="flex items-start justify-between gap-2 border-b border-border px-4 py-3">
        <div className="min-w-0">
          <h2 dir="auto" className="text-start text-sm font-medium">
            {card.title || 'Subagent'}
          </h2>
          <p className="text-xs text-faint-foreground">
            {[card.agentType, card.model === null ? null : modelName(card.model), card.status, said]
              .filter((part) => part !== null && part !== '')
              .join(' · ')}
          </p>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close subagent"
          className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        >
          <X className="size-3.5" />
        </button>
      </header>
      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-4 py-4">
        {card.prompt !== null && (
          <section aria-label="Asked">
            <h3 className="mb-1 text-xs font-medium text-muted-foreground">Asked</h3>
            <p
              dir="auto"
              className="rounded-md bg-muted px-3 py-2 text-start text-xs whitespace-pre-wrap [unicode-bidi:plaintext]"
            >
              {card.prompt}
            </p>
          </section>
        )}
        <section aria-label="Steps">
          <h3 className="mb-1 text-xs font-medium text-muted-foreground">Steps</h3>
          {card.steps.length === 0 ? (
            <p className="text-xs text-faint-foreground">
              {card.status === 'running' ? 'No step has ended yet.' : 'No steps.'}
            </p>
          ) : (
            <ol className="space-y-1">
              {card.steps.map((step, i) => (
                <li key={i} className="flex items-start gap-2 text-xs text-muted-foreground">
                  <Wrench aria-hidden className="mt-px size-3.5 shrink-0" />
                  <code dir="auto" className="font-mono break-all whitespace-pre-wrap">
                    {step}
                  </code>
                </li>
              ))}
            </ol>
          )}
        </section>
        {card.report !== null && (
          <section aria-label="Report">
            <h3 className="mb-1 text-xs font-medium text-muted-foreground">Report</h3>
            <ReplyText text={card.report} />
          </section>
        )}
      </div>
    </aside>
  )
}
