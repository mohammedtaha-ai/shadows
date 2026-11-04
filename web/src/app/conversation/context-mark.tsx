import type { Operation } from '@/api/threads'

export type PlanningContextState = NonNullable<NonNullable<Operation['invocation']>['planning_context']>

export function ContextState({ state }: { state: PlanningContextState }) {
  return <>{state === 'heard' ? 'heard' : 'not confirmed yet'}</>
}

export function ContextMark({ state, onOpen }: { state: PlanningContextState; onOpen: () => void }) {
  return <button type="button" onClick={onOpen}
    className="rounded-md border border-border/70 px-2 py-1 text-xs text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring">
    Planning context · <ContextState state={state} />
  </button>
}
