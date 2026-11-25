import { SkeletonContent } from '@/components/ui/skeleton'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { agentExtensionsQuery, setAgentExtension, saveAgentExtensionAnswer } from '@/api/extensions'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { SavingSwitch } from '../saving-switch'
import { useOpenSettings } from '@/app/navigation'
import { ToolRules } from './tool-rules'

export function AgentExtensions({ slug }: { slug: string }) {
  const state = useQuery(agentExtensionsQuery(slug))
  const open = useOpenSettings()
  const client = useQueryClient()
  return (
    <section role="group" aria-label="Capabilities" className="space-y-3">
      <h2 className="font-medium">Capabilities</h2>
      {state.isPending ? (
        <SkeletonContent label="Loading capabilities…" />
      ) : state.error ? (
        <div className="space-y-3">
          <ErrorLine error={state.error} />
          <Button
            type="button"
            variant="outline"
            disabled={state.isFetching}
            onClick={() => { void state.refetch() }}
          >
            Retry capabilities
          </Button>
        </div>
      ) : state.data.plugins.length + state.data.skills.length + state.data.mcp.length === 0 ? (
        <p>No capabilities yet.</p>
      ) : (
        <>
          <p className="text-xs text-muted-foreground">Changes apply when the next conversation session opens.</p>
          {(['skill', 'plugin', 'mcp'] as const).map(kind => (
            <div key={kind} className="space-y-3">
              <h3 className="text-sm text-muted-foreground">
                {kind === 'skill' ? 'Skills' : kind === 'plugin' ? 'Plugins' : 'MCP tools'}
              </h3>
              {(kind === 'plugin' ? state.data.plugins : kind === 'skill' ? state.data.skills : state.data.mcp).map(item => (
                <SavingSwitch
                  key={`${slug}:${kind}:${item.id}`}
                  name={item.name}
                  description={item.description}
                  enabled={item.enabled}
                  request={{ slug, kind, id: item.id }}
                  send={(enabled, commandId) => setAgentExtension(slug, kind, item.id, enabled, commandId)}
                  onSaved={async saved => {
                    await saveAgentExtensionAnswer(client, slug, kind, saved)
                    await client.invalidateQueries({ queryKey: agentExtensionsQuery(slug).queryKey, exact: true })
                  }}
                >
                  {kind === 'mcp' && item.enabled && <ToolRules slug={slug} id={item.id} />}
                </SavingSwitch>
              ))}
            </div>
          ))}
        </>
      )}
      <Button type="button" variant="link" onClick={() => open('plugins')}>Add to library</Button>
    </section>
  )
}
