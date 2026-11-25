import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  projectExtensionsQuery,
  saveProjectExtensionAnswer,
  setProjectExtension,
  type AgentItem,
  type PlannerExtensionKind,
} from '@/api/extensions'
import { saveProjectAnswer, setPlannerClaudeMd, type Project } from '@/api/projects'
import { ApiError } from '@/api/error'
import { useOpenSettings } from '@/app/navigation'
import { Button } from '@/components/ui/button'
import { SkeletonContent } from '@/components/ui/skeleton'
import { ErrorLine } from '../error-line'
import { SavingSwitch } from '../saving-switch'

const FIXED_LINE =
  'These switches apply to the Planner on Claude Code only. A skill or MCP server reaches the next conversation session that opens.'

const KINDS: { kind: PlannerExtensionKind; heading: string }[] = [
  { kind: 'skill', heading: 'Skills' },
  { kind: 'mcp', heading: 'MCP servers' },
]

export function PlannerSwitches({ project }: { project: Project }) {
  const client = useQueryClient()
  const open = useOpenSettings()
  const state = useQuery(projectExtensionsQuery(project.id))
  const viewKey = projectExtensionsQuery(project.id).queryKey
  const lists: Record<PlannerExtensionKind, AgentItem[]> | null = state.data
    ? { skill: state.data.skills, mcp: state.data.mcp }
    : null

  return (
    <section role="group" aria-label="What the Planner may use" className="space-y-3">
      <h2 className="text-sm font-medium">What the Planner may use</h2>
      <p className="text-xs text-faint-foreground">{FIXED_LINE}</p>
      <SavingSwitch
        name="Read the project's CLAUDE.md"
        description="Adds the CLAUDE.md in the project's folder to the Planner's instructions. Only a new conversation receives it."
        enabled={project.planner_reads_claude_md}
        request={{ project: project.id, setting: 'planner_reads_claude_md' }}
        send={(enabled, commandId) => setPlannerClaudeMd(project.id, commandId, enabled)}
        onSaved={saved => saveProjectAnswer(client, saved, 'planner_reads_claude_md')}
      />
      {state.isPending ? (
        <SkeletonContent label="Loading the library…" />
      ) : state.error ? (
        <div className="space-y-3">
          <ErrorLine error={state.error} />
          <Button type="button" variant="outline" disabled={state.isFetching} onClick={() => { void state.refetch() }}>
            Retry
          </Button>
        </div>
      ) : lists && lists.skill.length + lists.mcp.length === 0 ? (
        <p>No skills or MCP servers in the library.</p>
      ) : (
        lists &&
        KINDS.filter(({ kind }) => lists[kind].length > 0).map(({ kind, heading }) => (
          <div key={kind} className="space-y-3">
            <h3 className="text-sm text-muted-foreground">{heading}</h3>
            {lists[kind].map(item => (
              <SavingSwitch
                key={`${project.id}:${kind}:${item.id}`}
                name={item.name}
                description={item.description}
                enabled={item.enabled}
                request={{ project: project.id, kind, id: item.id }}
                send={(enabled, commandId) => setProjectExtension(project.id, kind, item.id, enabled, commandId)}
                onSaved={async saved => {
                  await saveProjectExtensionAnswer(client, project.id, kind, saved)
                  await client.invalidateQueries({ queryKey: viewKey, exact: true })
                }}
                onFailed={error => {
                  if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.status === 404) {
                    void client.invalidateQueries({ queryKey: viewKey, exact: true })
                  }
                }}
              />
            ))}
          </div>
        ))
      )}
      <Button type="button" variant="link" onClick={() => open('plugins')}>Add to library</Button>
    </section>
  )
}
