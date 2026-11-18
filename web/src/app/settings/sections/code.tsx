import { SkeletonContent } from '@/components/ui/skeleton'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useParams } from '@tanstack/react-router'
import { ChevronRight } from 'lucide-react'
import { type SyntheticEvent, useRef, useState } from 'react'
import { addServer, codeKeys, codeServersQuery, removeServer, type ServerStatus, setServerOff } from '@/api/code'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ConfirmPanel } from '../../confirm-panel'
import { ErrorLine } from '../../error-line'

const STATE_LABEL: Record<ServerStatus['state']['state'], string> = {
  idle: 'Idle', starting: 'Starting', running: 'Running', not_installed: 'Not installed', off: 'Off', failed: 'Failed',
}

const STATE_DOT: Record<ServerStatus['state']['state'], string> = {
  idle: 'bg-muted-foreground', starting: 'bg-amber-500', running: 'bg-emerald-500',
  not_installed: 'bg-muted-foreground', off: 'bg-border', failed: 'bg-red-500',
}

// Display names for the catalog's server ids; an added server or an unmapped id shows its id.
const LANGUAGE: Record<string, string> = {
  gopls: 'Go',
  'typescript-language-server': 'TypeScript',
  'rust-analyzer': 'Rust',
  pyright: 'Python',
  'python-lsp-server': 'Python',
  clangd: 'C and C++',
  jdtls: 'Java',
  'lua-language-server': 'Lua',
  'ruby-lsp': 'Ruby',
  solargraph: 'Ruby',
  intelephense: 'PHP',
  omnisharp: 'C#',
  zls: 'Zig',
}

const languageName = (id: string) => LANGUAGE[id] ?? id

export function CodeSection() {
  const { projectId } = useParams({ strict: false })
  if (!projectId) return <p role="status" className="text-sm text-muted-foreground">Open a project to see its language servers.</p>
  return <Servers projectId={projectId} />
}

function Servers({ projectId }: { projectId: string }) {
  const servers = useQuery(codeServersQuery(projectId))
  const [open, setOpen] = useState(false)
  const [showMore, setShowMore] = useState(false)
  const [adding, setAdding] = useState(false)
  const wanted = servers.data?.filter(server => server.wanted) ?? []
  const others = servers.data?.filter(server => !server.wanted) ?? []
  const running = servers.data?.filter(server => server.state.state === 'running').length ?? 0
  const summary = running > 0 ? `${running} running` : 'None running'
  const list = (items: ServerStatus[]) => <ul aria-label="Language servers" className="divide-y divide-border">
    {items.map(server => <ServerRow key={server.id} projectId={projectId} server={server} />)}
  </ul>
  return (
    <div className="divide-y divide-border">
      {servers.error && <ErrorLine error={servers.error} />}
      {servers.isPending && !servers.error && <SkeletonContent label="Loading servers…" />}
      {servers.data && <div className="py-3">
        <button
          type="button"
          aria-expanded={open}
          aria-label="Language servers"
          className="flex w-full items-center gap-4 text-left"
          onClick={() => setOpen(value => !value)}
        >
          <span className="flex-1">
            <span className="block font-medium">Language servers</span>
            <span className="block text-sm text-muted-foreground">Help the Planner read your code. It installs what's missing when it needs it.</span>
          </span>
          <span className="text-sm text-muted-foreground">{summary}</span>
          <ChevronRight aria-hidden className={`size-4 text-muted-foreground transition-transform ${open ? 'rotate-90' : ''}`} />
        </button>
        {open && <div className="mt-3 space-y-3 pl-4">
          {list(wanted)}
          {others.length > 0 && !showMore && <button type="button" className="text-sm text-muted-foreground underline" onClick={() => setShowMore(true)}>
            {others.length} more {others.length === 1 ? 'language' : 'languages'}
          </button>}
          {showMore && list(others)}
          {adding
            ? <AddServer projectId={projectId} />
            : <button type="button" className="text-sm text-muted-foreground underline" onClick={() => setAdding(true)}>Add a server</button>}
        </div>}
      </div>}
    </div>
  )
}

function ServerRow({ projectId, server }: { projectId: string; server: ServerStatus }) {
  const client = useQueryClient()
  const [confirming, setConfirming] = useState(false)
  const attempt = useRef<Attempt | null>(null)
  const off = server.state.state === 'off'
  const name = languageName(server.id)
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
        <span className="flex-1 font-medium">{name}</span>
        <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span aria-hidden className={`size-2 rounded-full ${STATE_DOT[server.state.state]}`} />
          {STATE_LABEL[server.state.state]}
        </span>
        <button
          type="button"
          role="switch"
          aria-checked={!off}
          aria-label={`${name} on`}
          disabled={toggle.isPending}
          className="rounded-full border border-border px-3 py-1 text-xs aria-checked:pill-active"
          onClick={() => { if (off) toggle.mutate(false); else setConfirming(true) }}
        >{off ? 'Off' : 'On'}</button>
        {server.added && <Button type="button" variant="outline" size="sm" disabled={remove.isPending} onClick={() => remove.mutate()}>Remove</Button>}
      </div>
      {server.state.state === 'failed' && server.state.error && <p className="text-xs text-muted-foreground">{server.state.error}</p>}
      {confirming && <ConfirmPanel
        question="Switch off?"
        description="Shadows' answers about this language get worse while it is off. Off in every project."
        error={toggle.error}
        confirmLabel="Switch off"
        pending={toggle.isPending}
        pendingLabel="Switching off…"
        onCancel={() => { setConfirming(false); toggle.reset(); attempt.current = null }}
        onConfirm={() => toggle.mutate(true)}
        presentation={{ kind: 'inline', ariaLabel: `Switch off ${name}`, className: 'space-y-2 rounded-lg border border-border p-3 text-sm', heading: true }}
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
  const submit = (event: SyntheticEvent) => { event.preventDefault(); add.mutate() }
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
