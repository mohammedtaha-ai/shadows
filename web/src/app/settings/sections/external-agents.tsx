import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import {
  type ExternalConnection, type IssuedConnection, connectionsQuery, issueConnection,
  revokeConnection, saveRevokedConnection,
} from '@/api/external-connections'
import { type Project, projectsQuery } from '@/api/projects'
import { Button } from '@/components/ui/button'
import { CopyButton } from '../../copy-button'
import { ErrorLine } from '../../error-line'

export function ExternalAgentsSection() {
  const queryClient = useQueryClient()
  const projects = useQuery(projectsQuery)
  const connections = useQuery(connectionsQuery)
  const [selected, setSelected] = useState<string[]>([])
  const [issued, setIssued] = useState<IssuedConnection>()
  const pending = useRef<Attempt | null>(null)
  const available = projects.data?.filter(project => !project.assistant)
  const projectIds = selected.filter(id => available?.some(project => project.id === id)).sort()
  const connect = useMutation({
    mutationFn: ({ projectIds, commandId }: { projectIds: string[]; commandId: string }) =>
      issueConnection(projectIds, commandId),
    gcTime: 0,
    onSuccess: answer => {
      pending.current = null
      setIssued(answer)
      void queryClient.invalidateQueries(connectionsQuery)
    },
  })

  return (
    <div className="space-y-6">
      <form aria-label="Connect external agent" className="space-y-3" onSubmit={event => {
        event.preventDefault()
        if (!projectIds.length || connect.isPending) return
        pending.current = attemptFor(pending.current, { projectIds })
        connect.mutate({ projectIds, commandId: pending.current.commandId })
      }}>
        <p className="text-sm text-muted-foreground">
          Connect Codex or Claude Code over MCP. Each token can access the projects you select.
        </p>
        {projects.isError && <ErrorLine error={projects.error} />}
        <fieldset className="space-y-2" disabled={connect.isPending || issued !== undefined}>
          <legend className="mb-2 text-sm font-medium">Allowed projects</legend>
          {projects.isPending && <p role="status" className="text-sm text-muted-foreground">Loading projects…</p>}
          {available?.length === 0 && <p className="text-sm text-muted-foreground">No projects available.</p>}
          {available?.map(project => (
            <label key={project.id} className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={projectIds.includes(project.id)}
                onChange={event => {
                  const checked = event.currentTarget.checked
                  setSelected(value => checked ? [...value, project.id] : value.filter(id => id !== project.id))
                }} />
              <span dir="auto">{project.name}</span>
              <span className="text-xs text-muted-foreground">({project.slug})</span>
            </label>
          ))}
        </fieldset>
        <Button type="submit" size="sm" disabled={!projectIds.length || connect.isPending || issued !== undefined}>
          {connect.isPending ? 'Connecting…' : 'Connect'}
        </Button>
        {connect.isError && <ErrorLine error={connect.error} />}
      </form>
      {issued && <ConnectionSetup issued={issued} close={() => {
        setIssued(undefined)
        setSelected([])
        connect.reset()
      }} />}
      <section aria-label="Connections" className="space-y-2">
        <h3 className="text-sm font-medium">Connections</h3>
        {connections.isPending && <p role="status" className="text-sm text-muted-foreground">Loading connections…</p>}
        {connections.isError && <ErrorLine error={connections.error} />}
        {connections.data?.length === 0 && <p className="text-sm text-muted-foreground">No connections yet.</p>}
        <ul className="divide-y divide-border rounded-md border border-border empty:hidden">
          {connections.data?.map(connection => (
            <ConnectionRow key={connection.id} connection={connection} projects={available ?? []}
              onRevoked={() => {
                if (issued?.connection.id === connection.id) {
                  setIssued(undefined)
                  setSelected([])
                  connect.reset()
                }
              }} />
          ))}
        </ul>
      </section>
    </div>
  )
}

function ConnectionSetup({ issued, close }: { issued: IssuedConnection; close: () => void }) {
  const [cli, setCli] = useState('codex')
  const id = issued.connection.id.replaceAll('-', '')
  const name = `shadows-${id}`
  const variable = `SHADOWS_TOKEN_${id.toUpperCase()}`
  return (
    <section aria-label="Connection setup" className="space-y-3 rounded-lg border border-accent-line bg-accent-softer p-3">
      {issued.token ? <>
        <label className="flex items-center gap-2 text-sm">
          Client
          <select value={cli} onChange={event => setCli(event.currentTarget.value)}
            className="rounded border border-border bg-background px-2 py-1">
            <option value="codex">Codex</option>
            <option value="claude">Claude Code</option>
          </select>
        </label>
        {cli === 'codex' ? <>
          <p className="text-xs text-muted-foreground">Set <code>{variable}</code> in the environment that starts Codex, then register the MCP server.</p>
          <CopyLine label="Token" value={issued.token} />
          <CopyLine label="Codex command" value={`codex mcp add ${name} --url "${issued.mcp_url}" --bearer-token-env-var ${variable}`} />
        </> : <CopyLine label="Claude Code command"
          value={`claude mcp add --transport http --scope user ${name} "${issued.mcp_url}" --header "Authorization: Bearer ${issued.token}"`} />}
        <p className="text-xs text-muted-foreground">Copy the details now. The token is shown once.</p>
      </> : <p className="text-sm text-muted-foreground">
        This connection was already created. Revoke it and connect again if you need a new token.
      </p>}
      <Button type="button" variant="secondary" size="sm" onClick={close}>Done</Button>
    </section>
  )
}

function CopyLine({ label, value }: { label: string; value: string }) {
  return <div className="space-y-1">
    <p className="text-xs font-medium">{label}</p>
    <div className="flex items-start gap-2">
      <pre className="min-w-0 flex-1 overflow-auto rounded bg-background px-2 py-1.5 text-xs">{value}</pre>
      <CopyButton text={value} size="sm" />
    </div>
  </div>
}

function ConnectionRow({ connection, projects, onRevoked }: {
  connection: ExternalConnection
  projects: Project[]
  onRevoked: () => void
}) {
  const queryClient = useQueryClient()
  const pending = useRef<Attempt | null>(null)
  const revoke = useMutation({
    mutationFn: (commandId: string) => revokeConnection(connection.id, commandId),
    onSuccess: async revoked => {
      pending.current = null
      await saveRevokedConnection(queryClient, revoked)
      onRevoked()
    },
  })
  const connected = new Date(connection.created_at).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' })
  return <li className="space-y-1 px-3 py-2">
    <div className="flex items-center gap-3 text-sm">
      <div className="min-w-0 flex-1">
        <p>Created {connected}</p>
        <p className="break-all font-mono text-xs text-muted-foreground">shadows-{connection.id.replaceAll('-', '')}</p>
        <p className="break-words text-xs text-muted-foreground">
          {connection.project_ids.map(id => projects.find(project => project.id === id)?.name ?? `Removed project (${id})`).join(', ')}
        </p>
      </div>
      {connection.revoked_at ? <span className="text-xs text-muted-foreground">Revoked</span> :
        <Button type="button" size="sm" variant="ghost" disabled={revoke.isPending}
          aria-label={`Revoke connection ${connection.id}`} onClick={() => {
            pending.current = attemptFor(pending.current, { revoke: connection.id })
            revoke.mutate(pending.current.commandId)
          }}>Revoke</Button>}
    </div>
    {revoke.isError && <ErrorLine error={revoke.error} />}
  </li>
}
