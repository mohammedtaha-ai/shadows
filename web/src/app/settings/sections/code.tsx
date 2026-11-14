import { SkeletonContent } from '@/components/ui/skeleton'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useParams } from '@tanstack/react-router'
import { type FormEvent, useRef, useState } from 'react'
import { addServer, codeKeys, codeServersQuery, removeServer, type ServerStatus, setServerOff } from '@/api/code'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ConfirmPanel } from '../../confirm-panel'
import { CopyButton } from '../../copy-button'
import { ErrorLine } from '../../error-line'

const STATE_LABEL: Record<ServerStatus['state']['state'], string> = {
  idle: 'Idle', starting: 'Starting', running: 'Running', not_installed: 'Not installed', off: 'Off', failed: 'Failed',
}

export function CodeSection() {
  const { projectId } = useParams({ strict: false })
  if (!projectId) return <p role="status" className="text-sm text-muted-foreground">Open a project to see its language servers.</p>
  return <Servers projectId={projectId} />
}

function Servers({ projectId }: { projectId: string }) {
  const servers = useQuery(codeServersQuery(projectId))
  return (
    <div className="space-y-5">
      {servers.error && <ErrorLine error={servers.error} />}
      {servers.isPending && !servers.error && <SkeletonContent label="Loading servers…" />}
      {servers.data && <ul aria-label="Language servers" className="divide-y divide-border">
        {servers.data.map(server => <ServerRow key={server.id} projectId={projectId} server={server} />)}
      </ul>}
      <AddServer projectId={projectId} />
    </div>
  )
}

function ServerRow({ projectId, server }: { projectId: string; server: ServerStatus }) {
  const client = useQueryClient()
  const [confirming, setConfirming] = useState(false)
  const attempt = useRef<Attempt | null>(null)
  const off = server.state.state === 'off'
  const refresh = () => client.invalidateQueries({ queryKey: codeKeys.servers(projectId) })
  const toggle = useMutation({
    retry: false,
    mutationFn: (value: boolean) => {
      attempt.current = attemptFor(attempt.current, { server: server.id, off: value })
      return setServerOff(server.id, value, attempt.current.commandId)
    },
    onSuccess: async () => { attempt.current = null; setConfirming(false); await refresh() },
  })
  const remove = useMutation({
    retry: false,
    mutationFn: () => removeServer(server.id, crypto.randomUUID()),
    onSuccess: refresh,
  })
  return (
    <li className="space-y-2 py-3">
      <div className="flex flex-wrap items-center gap-3">
        <span className="flex-1 font-medium">{server.id}</span>
        <span className="text-xs text-muted-foreground">{server.extensions.join(' ')}</span>
        <span className="text-xs text-muted-foreground">{STATE_LABEL[server.state.state]}</span>
        <button
          type="button"
          role="switch"
          aria-checked={!off}
          aria-label={`${server.id} on`}
          disabled={toggle.isPending}
          className="rounded-full border border-border px-3 py-1 text-xs aria-checked:pill-active"
          onClick={() => { if (off) toggle.mutate(false); else setConfirming(true) }}
        >{off ? 'Off' : 'On'}</button>
        {server.added && <Button type="button" variant="outline" size="sm" disabled={remove.isPending} onClick={() => remove.mutate()}>Remove</Button>}
      </div>
      {server.state.state === 'failed' && server.state.error && <p className="text-xs text-muted-foreground">{server.state.error}</p>}
      {server.state.state === 'not_installed' && server.install && <div className="flex items-center gap-2">
        <code className="flex-1 overflow-x-auto rounded bg-secondary px-2 py-1 font-mono text-xs">{server.install}</code>
        <CopyButton text={server.install} size="sm" />
      </div>}
      {confirming && <ConfirmPanel
        question="Switch off?"
        description="Shadows' answers about this language get worse while it is off."
        error={toggle.error}
        confirmLabel="Switch off"
        pending={toggle.isPending}
        pendingLabel="Switching off…"
        onCancel={() => { setConfirming(false); toggle.reset(); attempt.current = null }}
        onConfirm={() => toggle.mutate(true)}
        presentation={{ kind: 'inline', ariaLabel: `Switch off ${server.id}`, className: 'space-y-2 rounded-lg border border-border p-3 text-sm', heading: true }}
      />}
      {!confirming && toggle.error && <ErrorLine error={toggle.error} />}
      {remove.error && <ErrorLine error={remove.error} />}
    </li>
  )
}

function AddServer({ projectId }: { projectId: string }) {
  const client = useQueryClient()
  const empty = { id: '', command: '', args: '', extensions: '' }
  const [form, setForm] = useState(empty)
  const attempt = useRef<Attempt | null>(null)
  const set = (key: keyof typeof empty) => (event: { target: { value: string } }) => setForm(current => ({ ...current, [key]: event.target.value }))
  const extensions = form.extensions.split(',').map(value => value.trim()).filter(Boolean)
  const args = form.args.split(/\s+/).filter(Boolean)
  // Core checks the id, the command and each extension; its refusal is shown as it says it.
  const add = useMutation({
    retry: false,
    mutationFn: () => {
      const server = { id: form.id.trim(), command: form.command.trim(), args, extensions }
      attempt.current = attemptFor(attempt.current, server)
      return addServer(server, attempt.current.commandId)
    },
    onSuccess: async () => {
      attempt.current = null
      setForm(empty)
      await client.invalidateQueries({ queryKey: codeKeys.servers(projectId) })
    },
  })
  const submit = (event: FormEvent) => { event.preventDefault(); add.mutate() }
  return (
    <form aria-label="Add a server" onSubmit={submit} className="space-y-3 rounded-lg border border-border p-4">
      <h2 className="text-sm font-medium">Add a server</h2>
      <label className="grid gap-1 text-sm">Server id<Input value={form.id} onChange={set('id')} /></label>
      <label className="grid gap-1 text-sm">Command<Input value={form.command} onChange={set('command')} /></label>
      <label className="grid gap-1 text-sm">Arguments<Input value={form.args} onChange={set('args')} /></label>
      <label className="grid gap-1 text-sm">Extensions (comma-separated)<Input value={form.extensions} placeholder=".ts, .tsx" onChange={set('extensions')} /></label>
      <p className="text-xs text-muted-foreground">A server you add runs on your machine on your responsibility.</p>
      {add.error && <ErrorLine error={add.error} />}
      <Button type="submit" disabled={add.isPending}>Add server</Button>
    </form>
  )
}
