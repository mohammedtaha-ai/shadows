import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import {
  type ExternalConnection, type IssuedConnection, connectionsQuery, issueConnection,
  revokeConnection, saveConnectionAnswer, updateConnectionProjects,
} from '@/api/external-connections'
import { type Project, projectsQuery } from '@/api/projects'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { CopyButton } from '../../copy-button'
import { ErrorLine } from '../../error-line'
import { type HookClient, type HookPlatform, hookSetup } from './external-hooks'

export function ExternalAgentsSection() {
  const queryClient = useQueryClient()
  const projects = useQuery(projectsQuery)
  const connections = useQuery(connectionsQuery)
  const [name, setName] = useState('')
  const [selected, setSelected] = useState<string[]>([])
  const [issued, setIssued] = useState<IssuedConnection>()
  const pending = useRef<Attempt | null>(null)
  const available = projects.data?.filter(project => !project.assistant)
  const projectIds = selected.filter(id => available?.some(project => project.id === id)).sort()
  const canCreate = name.trim().length > 0 && projectIds.length > 0 && !issued
  const connect = useMutation({
    mutationFn: ({ name, projectIds, commandId }: {
      name: string; projectIds: string[]; commandId: string
    }) => issueConnection(name, projectIds, commandId),
    gcTime: 0,
    onSuccess: answer => {
      pending.current = null
      setIssued(answer)
      void queryClient.invalidateQueries(connectionsQuery)
    },
  })
  const closeSetup = () => {
    setIssued(undefined)
    setName('')
    setSelected([])
    connect.reset()
  }

  return (
    <div className="space-y-6">
      <form aria-label="Create external connection" className="space-y-3" onSubmit={event => {
        event.preventDefault()
        if (!canCreate || connect.isPending) return
        const request = { name: name.trim(), projectIds }
        pending.current = attemptFor(pending.current, request)
        connect.mutate({ ...request, commandId: pending.current.commandId })
      }}>
        <p className="text-sm text-muted-foreground">
          Connect an MCP client. Each connection can access the projects you select.
        </p>
        <label className="block space-y-1 text-sm font-medium">
          <span>Connection name</span>
          <Input type="text" value={name} onChange={event => setName(event.currentTarget.value)}
            placeholder="e.g. Laptop Codex" disabled={connect.isPending || issued !== undefined} />
        </label>
        {projects.isError && <ErrorLine error={projects.error} />}
        {projects.isPending && <p role="status" className="text-sm text-muted-foreground">Loading projects…</p>}
        {available && <ProjectChoices projects={available} selected={selected} onChange={setSelected}
          disabled={connect.isPending || issued !== undefined} />}
        <Button type="submit" size="sm" disabled={!canCreate || connect.isPending}>
          {connect.isPending ? 'Creating…' : 'Create connection'}
        </Button>
        {connect.isError && <ErrorLine error={connect.error} />}
      </form>
      {issued && <ConnectionSetup issued={issued} close={closeSetup} />}
      <section aria-label="Connections" className="space-y-2">
        <h3 className="text-sm font-medium">Connections</h3>
        {connections.isPending && <p role="status" className="text-sm text-muted-foreground">Loading connections…</p>}
        {connections.isError && <ErrorLine error={connections.error} />}
        {connections.data?.length === 0 && <p className="text-sm text-muted-foreground">No connections yet.</p>}
        <ul className="divide-y divide-border rounded-md border border-border empty:hidden">
          {connections.data?.map(connection => (
            <ConnectionRow key={connection.id} connection={connection} projects={available}
              onRevoked={() => {
                if (issued?.connection.id === connection.id) closeSetup()
              }} />
          ))}
        </ul>
      </section>
    </div>
  )
}

function ProjectChoices({ projects, selected, onChange, disabled }: {
  projects: Project[]
  selected: string[]
  onChange: (ids: string[]) => void
  disabled: boolean
}) {
  return <fieldset className="space-y-2" disabled={disabled}>
    <legend className="mb-2 text-sm font-medium">Allowed projects</legend>
    {projects.length === 0 && <p className="text-sm text-muted-foreground">No projects available.</p>}
    {projects.map(project => (
      <label key={project.id} className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={selected.includes(project.id)} onChange={event => {
          onChange(event.currentTarget.checked
            ? [...selected, project.id]
            : selected.filter(id => id !== project.id))
        }} />
        <span dir="auto">{project.name}</span>
        <span className="text-xs text-muted-foreground">({project.slug})</span>
      </label>
    ))}
  </fieldset>
}

function ConnectionSetup({ issued, close }: { issued: IssuedConnection; close: () => void }) {
  const [client, setClient] = useState('general')
  return (
    <section aria-label="Connection setup" className="space-y-3 rounded-lg border border-accent-line bg-accent-softer p-3">
      {issued.token ? <>
        <label className="flex items-center gap-2 text-sm">
          Client
          <select value={client} onChange={event => setClient(event.currentTarget.value)}
            className="rounded border border-border bg-background px-2 py-1">
            <option value="general">General setup</option>
            <option value="codex">Codex</option>
            <option value="claude">Claude Code</option>
          </select>
        </label>
        {client === 'general' && <>
          <p className="text-xs text-muted-foreground">
            Add a server named <code>shadows</code> in your MCP client and select Streamable HTTP.
            Use the URL below and set <code>Authorization</code> to <code>Bearer &lt;token&gt;</code>.
          </p>
          <CopyLine label="MCP URL" value={issued.mcp_url} />
          <CopyLine label="Token" value={issued.token} />
        </>}
        {client === 'codex' && <>
          <p className="text-xs text-muted-foreground">
            Set <code>SHADOWS_TOKEN</code> in the environment that starts Codex,
            then run the registration command.
          </p>
          <CopyLine label="Token" value={issued.token} />
          <CopyLine label="Codex command"
            value={`codex mcp add shadows --url "${issued.mcp_url}" --bearer-token-env-var SHADOWS_TOKEN`} />
        </>}
        {client === 'claude' && <CopyLine label="Claude Code command"
          value={`claude mcp add --transport http --scope user shadows "${issued.mcp_url}" --header "Authorization: Bearer ${issued.token}"`} />}
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
  projects: Project[] | undefined
  onRevoked: () => void
}) {
  const queryClient = useQueryClient()
  const [edit, setEdit] = useState<{ revision: number; selected: string[] }>()
  const [hooks, setHooks] = useState(false)
  const pending = useRef<Attempt | null>(null)
  const revoke = useMutation({
    mutationFn: (commandId: string) => revokeConnection(connection.id, commandId),
    onSuccess: async revoked => {
      pending.current = null
      await saveConnectionAnswer(queryClient, revoked)
      onRevoked()
    },
  })
  const connected = new Date(connection.created_at).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' })
  return <li className="space-y-1 px-3 py-2">
    <div className="flex items-center gap-3 text-sm">
      <div className="min-w-0 flex-1">
        <p className="break-words font-medium" dir="auto">{connection.name ?? 'Unnamed connection'}</p>
        <p className="text-xs text-muted-foreground">Created {connected}</p>
        <p className="break-words text-xs text-muted-foreground">
          {connection.project_ids.map(id => projects?.find(project => project.id === id)?.name
            ?? (projects ? 'Removed project' : id)).join(', ') || 'No projects allowed'}
        </p>
      </div>
      {connection.revoked_at ? <span className="text-xs text-muted-foreground">Revoked</span> :
        <div className="flex shrink-0 flex-wrap gap-1">
          <Button type="button" size="sm" variant="ghost" aria-expanded={hooks}
            disabled={!projects || !!edit || revoke.isPending} onClick={() => setHooks(open => !open)}>
            Wait hooks
          </Button>
          <Button type="button" size="sm" variant="ghost" disabled={!projects || !!edit || revoke.isPending}
            onClick={() => setEdit({ revision: connection.project_revision, selected: connection.project_ids })}>
            Edit projects
          </Button>
          <Button type="button" size="sm" variant="ghost" disabled={!!edit || revoke.isPending}
            aria-label={`Revoke connection ${connection.id}`} onClick={() => {
              pending.current = attemptFor(pending.current, { revoke: connection.id })
              revoke.mutate(pending.current.commandId)
            }}>Revoke</Button>
        </div>}
    </div>
    {revoke.isError && <ErrorLine error={revoke.error} />}
    {hooks && projects && !connection.revoked_at &&
      <WaitHooks connection={connection} projects={projects} />}
    {edit && projects && !connection.revoked_at &&
      <ConnectionProjects id={connection.id} initial={edit} projects={projects}
        close={() => setEdit(undefined)} />}
  </li>
}

function ConnectionProjects({ id, initial, projects, close }: {
  id: string
  initial: { revision: number; selected: string[] }
  projects: Project[]
  close: () => void
}) {
  const queryClient = useQueryClient()
  const [selected, setSelected] = useState(initial.selected)
  const pending = useRef<Attempt | null>(null)
  const save = useMutation({
    mutationFn: ({ projectIds, commandId }: { projectIds: string[]; commandId: string }) =>
      updateConnectionProjects(id, initial.revision, projectIds, commandId),
    onSuccess: async saved => {
      await saveConnectionAnswer(queryClient, saved)
      close()
    },
    onError: error => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.status === 409) {
        void queryClient.invalidateQueries(connectionsQuery)
      }
    },
  })
  const stale = save.error instanceof ApiError && save.error.problem.kind === 'daemon'
    && save.error.problem.code === 'REVISION_CONFLICT'

  return <form aria-label="Edit allowed projects" className="space-y-2 py-2" onSubmit={event => {
    event.preventDefault()
    if (save.isPending || stale) return
    const projectIds = selected.filter(id => projects.some(project => project.id === id)).sort()
    pending.current = attemptFor(pending.current, { revision: initial.revision, projectIds })
    save.mutate({ projectIds, commandId: pending.current.commandId })
  }}>
    <ProjectChoices projects={projects} selected={selected} onChange={setSelected}
      disabled={save.isPending || stale} />
    {save.isError && <ErrorLine error={save.error} />}
    {stale && <p className="text-xs text-muted-foreground">Close and reopen the editor to use the current projects.</p>}
    <div className="flex gap-2">
      <Button type="submit" size="sm" disabled={save.isPending || stale}>
        {save.isPending ? 'Saving…' : 'Save projects'}
      </Button>
      <Button type="button" size="sm" variant="secondary" disabled={save.isPending} onClick={close}>Cancel</Button>
    </div>
  </form>
}

function WaitHooks({ connection, projects }: { connection: ExternalConnection; projects: Project[] }) {
  const allowed = connection.project_ids.flatMap(id => projects.find(project => project.id === id) ?? [])
  const [projectId, setProjectId] = useState(allowed[0]?.id ?? '')
  const [client, setClient] = useState<HookClient>('claude')
  const [platform, setPlatform] = useState<HookPlatform>(
    () => /Windows/i.test(navigator.userAgent) ? 'windows' : 'unix')
  const project = allowed.find(candidate => candidate.id === projectId)
  const directory = project?.directory
  const setup = project && directory
    ? hookSetup({ client, platform, waitUrl: connection.wait_url, projectId: project.id, directory })
    : undefined
  return <section aria-label="Wait hooks" className="space-y-3 rounded-lg border border-accent-line bg-accent-softer p-3">
    <div className="flex flex-wrap gap-3 text-sm">
      <label className="flex items-center gap-2">
        Project
        <select value={projectId} onChange={event => setProjectId(event.currentTarget.value)}
          className="rounded border border-border bg-background px-2 py-1">
          {allowed.map(candidate => <option key={candidate.id} value={candidate.id}>{candidate.name}</option>)}
        </select>
      </label>
      <label className="flex items-center gap-2">
        Client
        <select value={client} onChange={event => setClient(event.currentTarget.value as HookClient)}
          className="rounded border border-border bg-background px-2 py-1">
          <option value="claude">Claude Code</option>
          <option value="codex">Codex desktop app</option>
        </select>
      </label>
      <label className="flex items-center gap-2">
        Platform
        <select value={platform} onChange={event => setPlatform(event.currentTarget.value as HookPlatform)}
          className="rounded border border-border bg-background px-2 py-1">
          <option value="windows">Windows</option>
          <option value="unix">macOS and Linux</option>
        </select>
      </label>
    </div>
    {allowed.length === 0 && <p className="text-sm text-muted-foreground">This connection has no projects.</p>}
    {project && !directory && <p className="text-sm text-muted-foreground">This project has no folder recorded.</p>}
    {setup && <>
      <CopyLine label="Wait address" value={connection.wait_url} />
      <CopyLine label={`Script, save as ${setup.scriptPath}`} value={setup.script} />
      <CopyLine label={`Hook entry, add to ${setup.entryPath}`} value={setup.entry} />
      <p className="text-xs text-muted-foreground">
        The files hold no token. The script reads <code>SHADOWS_TOKEN</code>, this connection's token
        saved when it was created, from the environment that starts the client.
        {platform === 'unix' && ' The script needs curl and jq.'}
      </p>
    </>}
  </section>
}
