// Structured fields edit the same OpenAPI document as the JSON view.
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { object } from './agreement-document'
const methods = ['get', 'post', 'put', 'patch', 'delete', 'head', 'options', 'trace']
export function OperationsEditor({ value, onChange, readOnly, onValidity }: {
  value: unknown; onChange: (value: unknown) => void; readOnly: boolean
  onValidity: (label: string, invalid: boolean) => void
}) {
  const doc = object(value), paths = object(doc.paths)
  const [path, setPath] = useState('/login')
  const [method, setMethod] = useState('post')
  const update = (oldPath: string, verb: string, op: Record<string, unknown>) =>
    onChange({ ...doc, paths: { ...paths, [oldPath]: { ...object(paths[oldPath]), [verb]: op } } })
  return <section className="space-y-3" aria-label="HTTP operations">
    <h3 className="font-medium">HTTP operations</h3>
    {Object.entries(paths).flatMap(([url, item]) => methods.flatMap(verb => {
      const raw = object(item)[verb]
      if (raw === undefined) return []
      const op = object(raw)
      return [<article key={`${url}:${verb}`} className="space-y-2 rounded-xl border border-border p-4">
        <strong>{verb.toUpperCase()} {url}</strong>
        {!readOnly && <OperationAddress path={url} method={verb} occupied={(p, m) =>
          object(paths[p])[m] !== undefined && !(p === url && m === verb)} onMove={(p, m) => {
          const oldItem = { ...object(item) }; delete oldItem[verb]
          const nextPaths = { ...paths }
          if (Object.keys(oldItem).length === 0) delete nextPaths[url]; else nextPaths[url] = oldItem
          nextPaths[p] = { ...object(nextPaths[p]), [m]: op }
          onChange({ ...doc, paths: nextPaths })
        }} />}
        <p className="text-xs text-faint-foreground">Operation {String(op['x-shadows-operation-id'] ?? 'identity missing')}</p>
        <label className="block">Description<textarea className="w-full rounded border border-border p-2"
          aria-label={`${verb} ${url} description`} disabled={readOnly} value={String(op.description ?? '')}
          onChange={e => update(url, verb, { ...op, description: e.target.value })} /></label>
        {['parameters', 'requestBody', 'responses', 'security'].map(field => <JsonField
          key={field} label={`${verb} ${url} ${field}`} value={op[field]}
          onValidity={onValidity}
          disabled={readOnly} onChange={v => update(url, verb, { ...op, [field]: v })} />)}
        {!readOnly && <Button size="sm" variant="outline" onClick={() => {
          const next = { ...object(item) }; delete next[verb]
          const remaining = { ...paths }
          if (Object.keys(next).length === 0) delete remaining[url]; else remaining[url] = next
          onChange({ ...doc, paths: remaining })
        }}>Remove operation</Button>}
      </article>]
    }))}
    {!readOnly && <div className="flex gap-2">
      <select aria-label="New operation method" value={method} onChange={e => setMethod(e.target.value)}>
        {methods.map(m => <option key={m}>{m}</option>)}</select>
      <input className="rounded border border-border p-2" aria-label="New operation path" value={path}
        onChange={e => setPath(e.target.value)} />
      <Button variant="outline" disabled={!path.startsWith('/') || object(paths[path])[method] !== undefined}
        onClick={() => update(path, method, { 'x-shadows-operation-id': crypto.randomUUID(),
          responses: { '200': { description: 'Successful response' } } })}>Add operation</Button>
    </div>}
    <JsonField label="Shared security" value={doc.security} disabled={readOnly} onValidity={onValidity}
      onChange={security => onChange({ ...doc, security })} />
    <JsonField label="Reusable schemas and components" value={doc.components} disabled={readOnly} onValidity={onValidity}
      onChange={components => onChange({ ...doc, components })} />
  </section>
}
function OperationAddress({ path, method, occupied, onMove }: {
  path: string; method: string; occupied: (path: string, method: string) => boolean;
  onMove: (path: string, method: string) => void
}) {
  const [nextPath, setPath] = useState(path), [nextMethod, setMethod] = useState(method)
  return <div className="flex gap-2">
    <select aria-label={`${method} ${path} method`} value={nextMethod} onChange={e => setMethod(e.target.value)}>
      {methods.map(m => <option key={m}>{m}</option>)}</select>
    <input className="rounded border border-border p-2" aria-label={`${method} ${path} path`} value={nextPath}
      onChange={e => setPath(e.target.value)} />
    <Button size="sm" variant="outline" disabled={!nextPath.startsWith('/') || occupied(nextPath, nextMethod) ||
      (nextPath === path && nextMethod === method)} onClick={() => onMove(nextPath, nextMethod)}>Change address</Button>
  </div>
}
function JsonField({ label, value, onChange, disabled, onValidity }: {
  label: string; value: unknown; onChange: (v: unknown) => void; disabled: boolean
  onValidity: (label: string, invalid: boolean) => void
}) {
  const [error, setError] = useState('')
  const [draft, setDraft] = useState<string | null>(null)
  return <label className="block text-sm">{label}
    <textarea className="block min-h-20 w-full rounded border border-border p-2 font-mono text-xs"
      aria-label={label} disabled={disabled} value={draft ?? JSON.stringify(value ?? null, null, 2)}
      onChange={e => {
        setDraft(e.target.value)
        try { onChange(JSON.parse(e.target.value)); setError(''); onValidity(label, false) }
        catch { setError('Enter valid JSON before saving'); onValidity(label, true) }
      }} />
    {error && <span role="alert">{error}</span>}
  </label>
}
