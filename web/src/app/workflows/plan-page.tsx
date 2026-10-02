// One job: the Workflows page — one plan version reviewed as its graph
// (§13.11, §16.8).

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, getRouteApi } from '@tanstack/react-router'
import { ChevronDown, Lock } from 'lucide-react'
import { useState } from 'react'
import { type Plan, archivePlan, unarchivePlan } from '@/api/client'
import { planVersionsQuery, workflowsKey } from '@/api/queries'
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
    <div className="flex h-full min-h-0 flex-col">
      <Header plan={p} projectId={projectId} />
      {p.plan_state === 'Archived' ? (
        <p className="flex items-center gap-2 border-b border-border bg-accent-softer px-6 py-2 text-xs text-secondary-foreground">
          Archived · read only
        </p>
      ) : p.state === 'Frozen' ? (
        <p className="flex items-center gap-2 border-b border-border bg-accent-softer px-6 py-2 text-xs text-secondary-foreground">
          <Lock className="size-3.5" aria-hidden />
          {/* Once the next version exists, editing no longer creates it. */}
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
    const commandId = crypto.randomUUID()
    if (isArchived) {
      unarchive.mutate(commandId)
    } else {
      archive.mutate(commandId)
    }
  }

  return (
    <header className="flex items-center justify-between gap-4 border-b border-border px-6 py-3">
      <div className="min-w-0 space-y-1">
        <div className="flex items-center gap-2">
          <h1 dir="auto" className="truncate text-sm font-medium">
            {plan.title}
          </h1>
          <span
            className={`rounded px-1.5 py-px text-xs ${
              plan.state === 'Frozen'
                ? 'bg-secondary text-secondary-foreground'
                : 'border border-border text-muted-foreground'
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
      </div>
      <div className="flex items-center gap-2">
        <nav aria-label="Versions" className="flex items-center gap-1 text-xs">
          <DropdownMenu>
            <DropdownMenuTrigger
              render={<Button variant="outline" size="sm" className="gap-1 text-xs" />}
            >
              Versions
              <ChevronDown className="size-3" aria-hidden />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
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
          className={buttonVariants({ variant: 'outline', size: 'sm', className: 'text-xs' })}
        >
          Continue this plan
        </Link>
        <Button
          variant="outline"
          size="sm"
          onClick={toggleArchive}
          disabled={archive.isPending || unarchive.isPending}
          className="text-xs"
        >
          {isArchived ? 'Unarchive' : 'Archive'}
        </Button>
      </div>
    </header>
  )
}
