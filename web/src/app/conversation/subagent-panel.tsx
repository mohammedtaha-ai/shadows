
import { Bot, Wrench, X } from 'lucide-react'
import type { SubagentCard } from '@/stream'
import { ReplyText } from './entry'
import { modelName, numbers } from './subagent-text'

export function SubagentPanel({ card, onClose }: { card: SubagentCard; onClose: () => void }) {
  const said = numbers(card)
  return (
    <aside
      aria-label="Subagent beside the conversation"
      className="flex w-[min(42rem,46%)] min-w-80 shrink-0 flex-col border-l border-border/80 bg-card shadow-[-12px_0_32px_-12px_var(--palette-overlay)]"
    >
      <header className="flex items-start justify-between gap-3 border-b border-border/70 bg-background/40 px-5 py-3.5 backdrop-blur-md">
        <div className="flex min-w-0 items-start gap-2.5">
          <span className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg border border-accent-line/40 bg-accent/25 text-accent-line">
            <Bot aria-hidden className="size-4" />
          </span>
          <div className="min-w-0">
            <h2 dir="auto" className="text-start text-sm font-semibold text-foreground">
              {card.title || 'Subagent'}
            </h2>
            <p className="mt-0.5 font-mono text-xs text-muted-foreground">
              {[card.agentType, card.model === null ? null : modelName(card.model), card.status, said]
                .filter((part) => part !== null && part !== '')
                .join(' · ')}
            </p>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close subagent"
          className="rounded-lg border border-border/60 bg-card/60 p-1.5 text-muted-foreground transition-colors hover:border-accent-line/40 hover:bg-muted hover:text-foreground"
        >
          <X className="size-3.5" />
        </button>
      </header>
      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-5 py-4">
        {card.prompt !== null && (
          <section aria-label="Asked" className="surface-panel rounded-xl p-3.5">
            <h3 className="mb-1.5 text-xs font-semibold tracking-wide text-accent-line">Asked</h3>
            <p
              dir="auto"
              className="rounded-lg border border-border/60 bg-background/70 px-3 py-2.5 text-start text-xs leading-relaxed whitespace-pre-wrap text-foreground/90 [unicode-bidi:plaintext]"
            >
              {card.prompt}
            </p>
          </section>
        )}
        <section aria-label="Steps" className="surface-panel rounded-xl p-3.5">
          <h3 className="mb-2 text-xs font-semibold tracking-wide text-accent-line">Steps</h3>
          {card.steps.length === 0 ? (
            <p className="text-xs text-faint-foreground">
              {card.status === 'running' ? 'No step has ended yet.' : 'No steps.'}
            </p>
          ) : (
            <ol className="space-y-1.5">
              {card.steps.map((step, i) => (
                <li
                  key={i}
                  className="flex items-start gap-2.5 rounded-lg border border-border/40 bg-background/50 px-2.5 py-1.5 text-xs text-muted-foreground"
                >
                  <Wrench aria-hidden className="mt-0.5 size-3.5 shrink-0 text-accent-line" />
                  <code dir="auto" className="font-mono text-foreground/90 break-all whitespace-pre-wrap">
                    {step}
                  </code>
                </li>
              ))}
            </ol>
          )}
        </section>
        {card.report !== null && (
          <section aria-label="Report" className="surface-panel rounded-xl p-3.5">
            <h3 className="mb-2 text-xs font-semibold tracking-wide text-accent-line">Report</h3>
            <ReplyText text={card.report} />
          </section>
        )}
      </div>
    </aside>
  )
}
