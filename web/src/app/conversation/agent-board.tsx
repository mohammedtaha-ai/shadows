// One job: Agent Mode's board. The prompt, steps, task list, subagents, model
// and context of the conversation, drawn from what the client already holds.

import { Activity, Bot, CheckCircle2, CircleDot, Code2, LoaderCircle, Sparkles } from 'lucide-react'
import { useMemo, useState } from 'react'
import type { ThreadEntry } from '@/api/threads'
import type { SubagentCard, TodoItem, ToolFrame } from '@/stream'
import { Planned } from '@/components/ui/planned'
import { AgentConsole } from './agent-console'
import { activityRows, currentTodos, liveSteps, stepsOf, withLiveSteps } from './activity-rows'
import { stepLine } from './activity-text'
import { StatusBadge } from './status-badge'
import { SubagentCardView } from './subagent-card'
import { duration } from './subagent-text'
import { TodoCard } from './todo-card'
import { silent, subagentOf } from './tool-text'
import type { Turn } from './turn-state'
import { type ContextFigures, contextShown, formatTokens } from './usage'

const STAGES = ['Plan', 'Analyze', 'Execute', 'Verify', 'Deliver'] as const
const TOOL_KINDS = ['Read', 'Edit', 'Search', 'Execute', 'Fetch'] as const

const NO_ENTRIES: readonly ThreadEntry[] = []

export function AgentBoard({
  title,
  projectName,
  agentLabel,
  modelLabel,
  entries,
  tools,
  todos,
  subagents,
  running,
  latest,
  thinking,
  label,
  context,
  onOpenSubagent,
}: {
  title: string
  projectName: string
  /** The agent or the CLI that runs the turns. */
  agentLabel: string
  modelLabel: string
  entries: readonly ThreadEntry[] | undefined
  tools: Readonly<Record<string, ToolFrame>>
  todos: Readonly<Record<string, readonly TodoItem[]>>
  /** The running turn's subagent cards as the stream last sent them. */
  subagents: readonly SubagentCard[]
  running: Turn | null
  latest: Turn | null
  thinking: boolean
  /** What the running turn says it is doing. */
  label: string | undefined
  context: ContextFigures | null
  onOpenSubagent: (id: string) => void
}) {
  const [picked, setPicked] = useState<string | null>(null)
  const all = entries ?? NO_ENTRIES
  const shown = useMemo(() => all.filter((entry) => !silent(entry)), [all])
  const frames = useMemo(() => Object.values(tools), [tools])
  const turnOp = frames.at(-1)?.op
  const steps = useMemo(() => {
    const live = turnOp === undefined ? [] : liveSteps(frames, all, turnOp, running === null)
    return stepsOf(withLiveSteps(activityRows(shown), live, shown, turnOp))
  }, [shown, frames, all, turnOp, running])
  const list = useMemo(() => currentTodos(todos, all), [todos, all])
  const cards = useMemo(() => {
    const byId = new Map<string, SubagentCard>()
    for (const entry of all) {
      const card = subagentOf(entry)
      if (card !== null) byId.set(card.id, card)
    }
    for (const card of subagents) if (!byId.has(card.id)) byId.set(card.id, card)
    return [...byId.values()]
  }, [all, subagents])
  const prompt = all.findLast((e) => e.kind === 'UserMessage' && e.body.trim() !== '')?.body ?? null

  const done = steps.filter((s) => s.status === 'completed').length
  const percent = steps.length === 0 ? null : Math.round((done / steps.length) * 100)
  const active =
    steps.find((s) => s.key === picked) ??
    steps.find((s) => s.status === 'in_progress' || s.status === 'pending') ??
    steps.at(-1) ??
    null
  const ctx = context === null ? null : contextShown(context.contextUsed, context.contextWindow)
  const operational = running !== null || thinking ? label : undefined

  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
      <div className="mx-auto flex max-w-6xl flex-col gap-5">
        <section aria-label="Task" className="surface-panel rounded-2xl p-5">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div className="flex min-w-0 items-start gap-4">
              <div className="flex size-12 shrink-0 items-center justify-center rounded-2xl border border-accent-line/40 bg-accent/15">
                <Code2 aria-hidden className="size-6 text-accent-line" />
              </div>
              <div className="min-w-0 space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <h2 dir="auto" className="truncate text-base font-semibold tracking-tight text-foreground">
                    {title}
                  </h2>
                  <span className="rounded-md border border-accent-line/25 bg-accent/10 px-2 py-0.5 text-[11px] font-medium text-accent-foreground">
                    {projectName}
                  </span>
                  <span className="rounded-md border border-border bg-card px-2 py-0.5 font-mono text-[11px] text-muted-foreground">
                    {agentLabel}
                  </span>
                </div>
                {prompt !== null && (
                  <p dir="auto" className="line-clamp-2 max-w-2xl text-xs leading-relaxed text-muted-foreground">
                    {prompt}
                  </p>
                )}
              </div>
            </div>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2 rounded-xl border border-accent-line/25 bg-card px-3 py-1.5 text-xs">
                <Sparkles aria-hidden className="size-3.5 text-warning" />
                <span className="text-muted-foreground">Model:</span>
                <span className="font-medium text-foreground">{modelLabel}</span>
              </div>
              <StatusBadge running={running} latest={latest} />
            </div>
          </div>

          <Planned
            className="mt-5"
            title="Run stages"
            reason="Waits for a shared request: the run stages of an agent turn."
          >
            <ol className="grid grid-cols-5 gap-2">
              {STAGES.map((stage, index) => (
                <li
                  key={stage}
                  className="flex items-center gap-2.5 rounded-xl border border-accent-line/15 bg-background/70 px-3 py-2 text-xs text-muted-foreground"
                >
                  <CircleDot aria-hidden className="size-4 shrink-0 text-accent-line/50" />
                  <span className="min-w-0">
                    <span className="block truncate font-semibold">{stage}</span>
                    <span className="block truncate text-[10px] opacity-75">Stage {index + 1}</span>
                  </span>
                </li>
              ))}
            </ol>
          </Planned>
        </section>

        <div className="grid grid-cols-1 gap-5 lg:grid-cols-12">
          <div className="flex flex-col gap-5 lg:col-span-7">
            <section aria-label="Agent execution" className="surface-panel rounded-2xl p-4">
              <div className="mb-3 flex items-center justify-between gap-2 border-b border-accent-line/15 pb-3">
                <div className="flex items-center gap-2">
                  <Activity aria-hidden className="size-4 text-accent-line" />
                  <h3 className="text-xs font-semibold tracking-wider text-foreground uppercase">
                    Agent Execution
                  </h3>
                </div>
                {percent !== null && (
                  <span className="rounded-md border border-accent-line/20 bg-accent/10 px-2 py-0.5 font-mono text-[11px] text-accent-foreground">
                    {done} / {steps.length} steps ({percent}%)
                  </span>
                )}
              </div>

              {steps.length === 0 ? (
                <p className="rounded-xl border border-border/50 bg-background/50 p-6 text-center text-xs text-muted-foreground">
                  {running !== null || thinking
                    ? 'The agent has not run a tool yet.'
                    : 'No tool steps in this conversation yet. Send a task below to start.'}
                </p>
              ) : (
                <div className="grid grid-cols-1 gap-4 md:grid-cols-12">
                  <ul className="max-h-64 space-y-1.5 overflow-y-auto pe-1 md:col-span-6">
                    {steps.map((step, i) => {
                      const ms =
                        step.startedAt !== null && step.finishedAt !== null
                          ? Date.parse(step.finishedAt) - Date.parse(step.startedAt)
                          : null
                      return (
                        <li key={step.key}>
                          <button
                            type="button"
                            aria-pressed={active?.key === step.key}
                            onClick={() => setPicked(step.key)}
                            className={`flex w-full items-center justify-between gap-2 rounded-xl border px-3 py-2 text-start text-xs transition-colors ${
                              active?.key === step.key
                                ? 'border-accent-line/45 bg-accent/15 text-foreground'
                                : 'border-border/50 bg-background/60 text-muted-foreground hover:border-accent-line/25 hover:text-foreground'
                            }`}
                          >
                            <span className="flex min-w-0 items-center gap-2">
                              {step.status === 'completed' ? (
                                <CheckCircle2 aria-hidden className="size-3.5 shrink-0 text-success" />
                              ) : step.status === 'in_progress' || step.status === 'pending' ? (
                                <LoaderCircle
                                  aria-hidden
                                  className="size-3.5 shrink-0 animate-spin text-accent-line motion-reduce:animate-none"
                                />
                              ) : (
                                <CircleDot aria-hidden className="size-3.5 shrink-0 text-warning" />
                              )}
                              <span dir="auto" className="truncate font-medium">
                                {i + 1}. {stepLine(step).text}
                              </span>
                            </span>
                            {ms !== null && ms >= 100 && (
                              <span className="shrink-0 font-mono text-[10px] text-faint-foreground">
                                {duration(ms)}
                              </span>
                            )}
                          </button>
                        </li>
                      )
                    })}
                  </ul>

                  <div className="rounded-xl border border-accent-line/25 bg-card/90 p-3.5 md:col-span-6">
                    {active !== null && (
                      <div className="space-y-2.5">
                        <span className="inline-flex items-center gap-1.5 rounded-md border border-accent-line/30 bg-accent/15 px-2 py-0.5 text-[11px] font-medium text-accent-foreground">
                          <Code2 aria-hidden className="size-3" />
                          Current Step
                        </span>
                        <p dir="auto" className="text-xs font-semibold text-foreground">
                          {stepLine(active).text}
                        </p>
                        {active.target !== null && (
                          <p className="truncate rounded-lg border border-accent-line/15 bg-background px-2.5 py-1.5 font-mono text-[11px] text-accent-foreground">
                            {active.target}
                          </p>
                        )}
                        {operational !== undefined && (
                          <p className="text-[11px] text-muted-foreground">Status: {operational}</p>
                        )}
                        {percent !== null && (
                          <div aria-hidden className="h-2 w-full overflow-hidden rounded-full bg-background p-0.5">
                            <div
                              className="h-full rounded-full bg-gradient-to-r from-accent to-accent-line transition-all duration-300"
                              style={{ width: `${percent}%` }}
                            />
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </section>

            <AgentConsole steps={steps} />
          </div>

          <div className="flex flex-col gap-5 lg:col-span-5">
            {list.length > 0 && <TodoCard items={list} />}

            <section aria-label="Subagents" className="surface-panel space-y-3 rounded-2xl p-4">
              <div className="flex items-center gap-2">
                <Bot aria-hidden className="size-4 text-accent-line" />
                <h3 className="text-xs font-semibold tracking-wider text-foreground uppercase">Subagents</h3>
              </div>
              {cards.length === 0 ? (
                <p className="text-xs text-muted-foreground">No subagents in this conversation.</p>
              ) : (
                cards.map((card) => <SubagentCardView key={card.id} card={card} onOpen={onOpenSubagent} />)
              )}
            </section>

            <section aria-label="Context" className="surface-panel space-y-3 rounded-2xl p-4">
              <h3 className="text-xs font-semibold tracking-wider text-foreground uppercase">Context</h3>
              {ctx === null ? (
                <p className="text-xs text-muted-foreground">No context figures reported yet.</p>
              ) : (
                <>
                  <p className="font-mono text-xs text-foreground tabular-nums">
                    {formatTokens(ctx.used)} / {formatTokens(ctx.window)} tokens ({ctx.percent}%)
                  </p>
                  <div aria-hidden className="h-2 w-full overflow-hidden rounded-full bg-background p-0.5">
                    <div
                      className="h-full rounded-full bg-gradient-to-r from-accent to-accent-line"
                      style={{ width: `${Math.min(ctx.percent, 100)}%` }}
                    />
                  </div>
                </>
              )}
              <Planned
                title="Context per operation"
                reason="Waits for a shared request: context use per operation."
              />
            </section>

            <Planned
              title="Active tools"
              reason="Waits for a shared request: the run stages of an agent turn, which name the tools in use."
            >
              <ul className="flex flex-wrap gap-2">
                {TOOL_KINDS.map((kind) => (
                  <li
                    key={kind}
                    className="flex items-center gap-1.5 rounded-lg border border-border/60 px-2.5 py-1 text-xs"
                  >
                    <CircleDot aria-hidden className="size-3" />
                    {kind}
                  </li>
                ))}
              </ul>
            </Planned>
          </div>
        </div>
      </div>
    </div>
  )
}
