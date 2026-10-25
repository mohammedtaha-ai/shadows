import { useState } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'

import { idError, type ModelPick } from './model-picks'
type ListedModel = { id: string; context_window?: number | null }

export function ProviderModels({ listed, picked, onChange, manual, onManualChange }: {
  listed: ListedModel[]
  picked: ModelPick[]
  onChange: (picked: ModelPick[]) => void
  manual: string
  onManualChange: (value: string) => void
}) {
  const [search, setSearch] = useState('')
  const [error, setError] = useState<string | null>(null)
  const add = () => {
    const invalid = idError(manual) ?? (picked.some(pick => pick.id === manual) ? 'Models must be listed once each.' : picked.length >= 20 ? 'Models must be 1 to 20 ids.' : null)
    setError(invalid)
    if (!invalid) { onChange([...picked, { id: manual, contextWindow: listed.find(item => item.id === manual)?.context_window ?? 200000 }]); onManualChange('') }
  }
  const rows = [...new Map([...listed, ...picked.filter(pick => !listed.some(item => item.id === pick.id)).map(pick => ({ id: pick.id, context_window: pick.contextWindow }))].map(item => [item.id, item])).values()]
  return <section className="grid gap-3" aria-label="Provider models">
    {rows.length > 0 && <Input aria-label="Search models" placeholder="Search models" value={search} onChange={event => setSearch(event.target.value)} />}
    <div className="max-h-64 space-y-3 overflow-y-auto">
      {rows.filter(row => row.id.toLowerCase().includes(search.toLowerCase())).map(row => {
        const chosen = picked.find(pick => pick.id === row.id)
        return <div key={row.id} className="grid gap-2 rounded border border-border p-3">
          <label className="flex gap-2 text-sm"><input type="checkbox" aria-label={row.id} checked={!!chosen} onChange={event => {
            onChange(event.target.checked ? [...picked, { id: row.id, contextWindow: row.context_window ?? 200000 }] : picked.filter(pick => pick.id !== row.id))
          }} />{row.id}</label>
          {chosen && <div className="flex flex-wrap items-center gap-3">
            <label className="flex gap-2 text-xs"><input type="radio" name="provider-default-model" aria-label={`${row.id} default`} checked={picked[0]?.id === row.id} onChange={() => onChange([chosen, ...picked.filter(pick => pick.id !== row.id)])} />Default</label>
            <label className="text-xs">Context window<Input type="number" aria-label={`Context window for ${row.id}`} value={Number.isNaN(chosen.contextWindow) ? '' : chosen.contextWindow} onChange={event => onChange(picked.map(pick => pick.id === row.id ? { ...pick, contextWindow: event.target.value === '' ? NaN : Number(event.target.value) } : pick))} /></label>
          </div>}
        </div>
      })}
    </div>
    <div className="flex gap-2"><Input aria-label="Add a model by id" placeholder="Add a model by id" value={manual} onChange={event => onManualChange(event.target.value)}
      onKeyDown={event => {
        if (event.key === 'Enter' && !event.nativeEvent.isComposing) { event.preventDefault(); add() }
      }} />
      <Button type="button" variant="outline" onClick={add}>Add</Button>
    </div>
    {error && <p role="alert" className="text-xs text-destructive-foreground">{error}</p>}
  </section>
}
