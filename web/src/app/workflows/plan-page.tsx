import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, getRouteApi } from '@tanstack/react-router'
import { Archive, ChevronDown, Layers, Lock } from 'lucide-react'
import { useState } from 'react'
import { type Plan, archivePlan, unarchivePlan } from '@/api/workflows'
import { planVersionsQuery, workflowsKey } from '@/api/workflows'
import { Planned } from '@/components/ui/planned'
import { Button, buttonVariants } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { ErrorLine } from '../error-line'
import { ApproveBar } from './approve-bar'
import { InspectPanel } from './inspect-panel'
import { PlanGraph } from './plan-graph'
import { stateLabel } from './plan-state'
import { usePlan } from './use-plan'
import { VersionLine } from './version-line'

const route = getRouteApi('/projects/$projectId/workflows/$workflowId')

/** The route's component. Keyed by version, so moving to the next one
 * closes the task the last one had open. */
export function PlanPage() {
  const { projectId, workflowId } = route.useParams()
  return <PlanView key={workflowId} projectId={projectId} workflowId={workflowId} />
}

function PlanView({ projectId, workflowId }: { projectId: string; workflowId: string }) {
  const plan = usePlan(workflowId)
  const [inspected, setInspected] = useState<number | null>(null)

  if (plan.data === undefined) {
    return (
      <div className="p-6 text-sm text-faint-foreground">
        {plan.isError ? <ErrorLine error={plan.error} /> : 'Reading the plan…'}
      </div>
    )
  }
  const p = plan.data
  const task = p.tasks.find((t) => t.number === inspected)

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <Header plan={p} projectId={projectId} />
      {p.plan_state === 'Archived' ? (
        <p className="flex items-center gap-2 border-b border-accent-line/15 bg-accent-line/10 px-6 py-2 text-xs text-secondary-foreground">
          <Archive className="size-3.5 text-accent-line" aria-hidden />
          Archived · read only
        </p>
      ) : p.state === 'Frozen' ? (
        <p className="flex items-center gap-2 border-b border-border/80 bg-accent-softer/80 px-6 py-2 text-xs text-secondary-foreground">
          <Lock className="size-3.5 text-accent-line" aria-hidden />
          {/* A frozen version is not edited in place: the first edit makes the next draft, later ones reuse it. */}
          Approved v{p.version} ·{' '}
          {p.next == null
            ? `editing creates draft v${p.version + 1}`
            : `v${p.version + 1} is its next version`}
        </p>
      ) : (
        <ApproveBar plan={p} />
      )}
      {plan.isError && (
        <div className="px-6 pt-2">
          <ErrorLine error={plan.error} />
        </div>
      )}
      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
          {/* One canvas throughout: when Inspect opens or closes, the graph
              refits to the width left (see `plan-camera.ts`). */}
          <PlanGraph plan={p} onSelectTask={(t) => setInspected(t.number)} />
        </div>
        {task !== undefined && (
          <InspectPanel plan={p} task={task} onClose={() => setInspected(null)} />
        )}
      </div>
    </div>
  )
}

function Header({ plan, projectId }: { plan: Plan; projectId: string }) {
  const queryClient = useQueryClient()
  const { data: planVersions } = useQuery(planVersionsQuery(plan.plan_id))
  const isArchived = plan.plan_state === 'Archived' || planVersions?.state === 'Archived'

  const archive = useMutation({
    mutationFn: (commandId: string) => archivePlan(plan.plan_id, commandId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: workflowsKey })
    },
  })

  const unarchive = useMutation({
    mutationFn: (commandId: string) => unarchivePlan(plan.plan_id, commandId),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: workflowsKey })
    },
  })

  const toggleArchive = () => {
    archive.reset()
    unarchive.reset()
    const commandId = crypto.randomUUID()
    if (isArchived) {
      unarchive.mutate(commandId)
    } else {
      archive.mutate(commandId)
    }
  }
  const error = archive.error ?? unarchive.error

  return (
    <header className="flex flex-wrap items-center justify-between gap-4 border-b border-accent-line/15 bg-background/85 px-6 py-3 backdrop-blur-xl">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-accent-line/35 bg-accent-line/15">
          <Layers aria-hidden className="size-4 text-accent-line" />
        </div>
      <div className="min-w-0 space-y-1">
        <div className="flex items-center gap-2">
          <h1 dir="auto" className="truncate text-sm font-semibold">
            {plan.title}
          </h1>
          <span
            className={`rounded-md px-2 py-0.5 text-xs font-medium transition-colors ${
              plan.state === 'Frozen'
                ? 'border border-success/30 bg-success/10 text-success'
                : 'border border-accent-line/25 bg-accent-line/10 text-accent-line'
            }`}
          >
            {stateLabel(plan)}
          </span>
        </div>
        <VersionLine
          version={plan.version}
          writtenBy={plan.written_by}
          changeReason={plan.change_reason}
          projectId={projectId}
        />
        {error !== null && <ErrorLine error={error} />}
      </div>
      </div>
      <div className="flex flex-wrap items-center gap-4">
        <Planned
          title="Plan progress"
          reason="Waits for the shared request: task status on a plan task."
          className="min-w-52"
        >
          <p className="text-xs tabular-nums">{plan.tasks.length} {plan.tasks.length === 1 ? 'task' : 'tasks'} · — done</p>
          <div className="h-1.5 w-full rounded-full bg-muted ring-1 ring-accent-line/20" />
          <div className="flex flex-wrap gap-1">
            {plan.tasks.map((t) => (
              <button key={t.id} type="button" disabled className="rounded border border-border px-1.5 py-0.5 font-mono text-[10px]">
                T{t.number}
              </button>
            ))}
          </div>
        </Planned>
      <div className="flex items-center gap-2">
        <nav aria-label="Versions" className="flex items-center gap-1 text-xs">
          <DropdownMenu>
            <DropdownMenuTrigger
              render={<Button variant="outline" size="sm" className="gap-1 text-xs shadow-2xs transition-all hover:bg-accent/40" />}
            >
              Versions
              <ChevronDown className="size-3" aria-hidden />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48 shadow-lg">
              <DropdownMenuGroup>
                <DropdownMenuLabel>Versions</DropdownMenuLabel>
                {planVersions?.versions.map((v) => (
                  <DropdownMenuItem
                    key={v.workflow_id}
                    render={
                      <Link
                        to="/projects/$projectId/workflows/$workflowId"
                        params={{ projectId, workflowId: v.workflow_id }}
                      />
                    }
                    className={v.workflow_id === plan.id ? 'font-medium bg-accent' : ''}
                  >
                    <span>v{v.version} · {v.state === 'Frozen' ? 'Approved' : 'Draft'}</span>
                  </DropdownMenuItem>
                ))}
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        </nav>
        <Link
          to="/projects/$projectId/new"
          params={{ projectId }}
          search={{ plan: plan.plan_id }}
          className={buttonVariants({ variant: 'outline', size: 'sm', className: 'text-xs shadow-2xs transition-all hover:bg-accent/40' })}
        >
          Continue this plan
        </Link>
        <Button
          variant="outline"
          size="sm"
          onClick={toggleArchive}
          disabled={archive.isPending || unarchive.isPending}
          className="text-xs shadow-2xs transition-all hover:bg-accent/40"
        >
          {isArchived ? 'Unarchive' : 'Archive'}
        </Button>
      </div>
      </div>
    </header>
  )
}
