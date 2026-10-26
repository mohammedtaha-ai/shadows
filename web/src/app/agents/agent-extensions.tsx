import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { agentExtensionsQuery, setAgentExtension, saveAgentExtensionAnswer, type AgentItem, type ExtensionKind } from '@/api/extensions'
import { attemptFor, type Attempt } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { useOpenSettings } from '@/app/navigation'

export function AgentExtensions({ slug }: { slug: string }) {
  const state = useQuery(agentExtensionsQuery(slug))
  const open = useOpenSettings()
  return <section role="group" aria-label="Plugins & Skills" className="space-y-3">
    <h2 className="font-medium">Plugins & Skills</h2>
    {state.isPending ? <p role="status">Loading plugins and skills…</p> : state.error ? <div className="space-y-3"><ErrorLine error={state.error} /><Button type="button" variant="outline" disabled={state.isFetching} onClick={() => { void state.refetch() }}>Retry plugins and skills</Button></div> :
      state.data.plugins.length + state.data.skills.length === 0 ? <div><p>No plugins or skills yet.</p><Button type="button" variant="link" onClick={() => open('plugins')}>Open Plugins & Skills</Button></div> : <>
        <p className="text-xs text-muted-foreground">Changes apply when the next conversation session opens.</p>
        {(['plugin', 'skill'] as const).map(kind => <div key={kind} className="space-y-3">{(kind === 'plugin' ? state.data.plugins : state.data.skills).map(item => <ExtensionSwitch key={`${slug}:${kind}:${item.id}`} slug={slug} kind={kind} item={item} />)}</div>)}
      </>}
  </section>
}

function ExtensionSwitch({ slug, kind, item }: { slug: string; kind: ExtensionKind; item: AgentItem }) {
  const client = useQueryClient()
  const queryKey = agentExtensionsQuery(slug).queryKey
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const [optimistic, setOptimistic] = useState<boolean | null>(null)
  const toggle = useMutation({ retry: false,
    mutationFn: (enabled: boolean) => {
      attempt.current = attemptFor(attempt.current, { slug, kind, id: item.id, enabled })
      return setAgentExtension(slug, kind, item.id, enabled, attempt.current.commandId)
    },
    onSuccess: async saved => {
      attempt.current = null
      await saveAgentExtensionAnswer(client, slug, kind, saved)
      await client.invalidateQueries({ queryKey, exact: true })
    },
    onSettled: () => { locked.current = false; setOptimistic(null) },
  })
  return <div className="space-y-2 rounded border border-border p-3">
    <label className="flex items-center justify-between gap-3 text-sm"><span>{item.name}</span><input type="checkbox" role="switch" aria-label={item.name} checked={optimistic ?? item.enabled} disabled={toggle.isPending} onChange={event => {
      if (locked.current) return
      locked.current = true; setOptimistic(event.target.checked); toggle.mutate(event.target.checked)
    }} /></label>
    <p className="text-xs text-muted-foreground">{item.description}</p>
    {toggle.error && <ErrorLine error={toggle.error} />}
  </div>
}
