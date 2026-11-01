import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRef, useState, type FormEvent } from 'react'
import { extensionKeys, saveMcp, type McpInput, type McpItem, type McpValueInput } from '@/api/extensions'
import { attemptFor, type Attempt } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'

type SecretRow = { name: string; value: string; set: boolean; key: number }
export function McpForm({ item, onDone }: { item: McpItem | null; onDone: () => void }) {
  const client = useQueryClient()
  const transport = item?.transport
  const [name, setName] = useState(item?.name ?? '')
  const [description, setDescription] = useState(item?.description ?? '')
  const [type, setType] = useState<'http' | 'stdio'>(transport?.type ?? 'http')
  const [url, setUrl] = useState(transport?.type === 'http' ? transport.url : '')
  const [command, setCommand] = useState(transport?.type === 'stdio' ? transport.command : '')
  const [args, setArgs] = useState(transport?.type === 'stdio' ? transport.args : [])
  const [headers, setHeaders] = useState<SecretRow[]>(transport?.type === 'http' ? transport.headers.map((row, key) => ({ ...row, value: '', key })) : [])
  const [env, setEnv] = useState<SecretRow[]>(transport?.type === 'stdio' ? transport.env.map((row, key) => ({ ...row, value: '', key })) : [])
  const nextKey = useRef(headers.length + env.length)
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const rows = type === 'http' ? headers : env
  const setRows = type === 'http' ? setHeaders : setEnv
  const save = useMutation({ retry: false,
    mutationFn: (input: McpInput) => {
      attempt.current = attemptFor(attempt.current, { id: item?.id ?? null, input })
      return saveMcp(item?.id ?? null, attempt.current.commandId, input)
    },
    onSuccess: async () => {
      attempt.current = null; setHeaders([]); setEnv([])
      await Promise.all([client.invalidateQueries({ queryKey: extensionKeys.library() }), client.invalidateQueries({ queryKey: extensionKeys.agents() })])
      onDone()
    },
    onSettled: () => { locked.current = false },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (locked.current) return
    const values: McpValueInput[] = rows.map(row => ({ name: row.name, ...(item !== null && row.value === '' ? {} : { value: row.value }) }))
    locked.current = true
    save.mutate({ name, description, transport: type === 'http' ? { type, url, headers: values } : { type, command, args, env: values } })
  }
  return <div className="space-y-4">
    <Button type="button" variant="ghost" disabled={save.isPending} onClick={onDone}>Back</Button>
    <form onSubmit={submit} className="space-y-4">
      <fieldset disabled={save.isPending} className="grid gap-4">
        <label className="grid gap-2 text-sm">Name<Input aria-label="MCP name" value={name} onChange={e => setName(e.target.value)} /></label>
        <label className="grid gap-2 text-sm">Description<Input aria-label="MCP description" value={description} onChange={e => setDescription(e.target.value)} /></label>
        <label className="grid gap-2 text-sm">Transport<select aria-label="MCP transport" className="rounded border border-input bg-background p-2" value={type} onChange={e => setType(e.target.value as 'http' | 'stdio')}>
          <option value="http">HTTP</option><option value="stdio">stdio</option>
        </select></label>
        {type === 'http' ? <label className="grid gap-2 text-sm">URL<Input aria-label="MCP URL" value={url} onChange={e => setUrl(e.target.value)} /></label> : <>
          <p className="text-xs text-muted-foreground">Runs on this machine's daemon only</p>
          <label className="grid gap-2 text-sm">Command<Input aria-label="MCP command" value={command} onChange={e => setCommand(e.target.value)} /></label>
          <div className="space-y-2">{args.map((argument, index) => <div key={index} className="flex gap-2">
            <Input aria-label={`Argument ${index + 1}`} value={argument} onChange={e => setArgs(args.map((value, i) => i === index ? e.target.value : value))} />
            <Button type="button" variant="outline" aria-label={`Remove argument ${index + 1}`} onClick={() => setArgs(args.filter((_, i) => i !== index))}>Remove</Button>
          </div>)}<Button type="button" variant="outline" onClick={() => setArgs([...args, ''])}>Add argument</Button></div>
        </>}
        <div className="space-y-3">
          {rows.map((row, index) => <div key={row.key} className="space-y-2 rounded border border-border p-3">
            <label className="grid gap-1 text-sm">{type === 'http' ? 'Header name' : 'Environment name'}<Input aria-label={`Secret name ${index + 1}`} value={row.name} onChange={e => setRows(rows.map(value => value.key === row.key ? { ...value, name: e.target.value } : value))} /></label>
            <label className="grid gap-1 text-sm">Value{row.set && <span className="text-xs text-muted-foreground">Set · leave empty to keep</span>}<Input aria-label={`Secret value ${index + 1}`} type="password" autoComplete="new-password" value={row.value} onChange={e => setRows(rows.map(value => value.key === row.key ? { ...value, value: e.target.value } : value))} /></label>
            <Button type="button" variant="outline" aria-label={`Remove secret ${index + 1}`} onClick={() => setRows(rows.filter(value => value.key !== row.key))}>Remove</Button>
          </div>)}
          <Button type="button" variant="outline" onClick={() => setRows([...rows, { name: '', value: '', set: false, key: nextKey.current++ }])}>{type === 'http' ? 'Add header' : 'Add environment variable'}</Button>
        </div>
        {save.error && <ErrorLine error={save.error} />}
        <Button type="submit" disabled={save.isPending}>{save.isPending ? 'Saving…' : item ? 'Save MCP tool' : 'Add MCP tool'}</Button>
      </fieldset>
    </form>
  </div>
}
