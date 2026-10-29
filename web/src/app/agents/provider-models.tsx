import { useState } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'

export type ModelPick = { id: string; contextWindow?: number | null }
type ListedModel = { id: string; context_window?: number | null }

export function ProviderModels({ listed, picked, onChange, manual, onManualChange, refusal }: {
  listed: ListedModel[]
  picked: ModelPick[]
  onChange: (picked: ModelPick[]) => void
  manual: string
  onManualChange: (value: string) => void
  refusal: { field: string; message: string } | null
}) {
  const [search, setSearch] = useState('')
  const [error, setError] = useState<string | null>(null)
  const add = () => {
    const invalid = picked.some(pick => pick.id === manual) ? 'Models must be listed once each.' : null
    setError(invalid)
    if (!invalid) { onChange([...picked, { id: manual, contextWindow: listed.find(item => item.id === manual)?.context_window }]); onManualChange('') }
  }
  const rows = [...new Map([...listed, ...picked.filter(pick => !listed.some(item => item.id === pick.id)).map(pick => ({ id: pick.id, context_window: pick.contextWindow }))].map(item => [item.id, item])).values()]
  return <section className="grid gap-3" aria-label="Provider models">
    {rows.length > 0 && <Input aria-label="Search models" placeholder="Search models" value={search} onChange={event => setSearch(event.target.value)} />}
    <div className="max-h-64 space-y-3 overflow-y-auto">
      {rows.filter(row => row.id.toLowerCase().includes(search.toLowerCase()) || (refusal?.field.startsWith(`models[${picked.findIndex(pick => pick.id === row.id)}].`) ?? false)).map(row => {
        const chosen = picked.find(pick => pick.id === row.id)
        const index = picked.indexOf(chosen!)
        return <div key={row.id} className="grid gap-2 rounded border border-border p-3">
          <label className="flex gap-2 text-sm"><input type="checkbox" aria-label={row.id} checked={!!chosen} onChange={event => {
            onChange(event.target.checked ? [...picked, { id: row.id, contextWindow: row.context_window }] : picked.filter(pick => pick.id !== row.id))
          }} />{row.id}</label>
          {chosen && refusal?.field === `models[${index}].id` && <p role="alert" className="text-xs text-destructive-foreground">{refusal.message}</p>}
          {chosen && <div className="flex flex-wrap items-center gap-3">
            <label className="flex gap-2 text-xs"><input type="radio" name="provider-default-model" aria-label={`${row.id} default`} checked={picked[0]?.id === row.id} onChange={() => onChange([chosen, ...picked.filter(pick => pick.id !== row.id)])} />Default</label>
            <label className="text-xs">Context window<Input type="number" aria-label={`Context window for ${row.id}`} value={chosen.contextWindow ?? ''} onChange={event => onChange(picked.map(pick => pick.id === row.id ? { ...pick, contextWindow: event.target.value === '' ? undefined : Number(event.target.value) } : pick))} /><span>Leave blank for Core's default.</span></label>
            {refusal?.field === `models[${index}].context_window` && <p role="alert" className="text-xs text-destructive-foreground">{refusal.message}</p>}
          </div>}
        </div>
      })}
    </div>
    <div className="flex gap-2"><Input aria-label="Add a model by id" placeholder="Add a model by id" value={manual} onChange={event => onManualChange(event.target.value)}
      onKeyDown={event => {
        // An Enter that confirms an IME composition is not an Add; Safari reports it
        // after compositionend, with only keyCode 229 to tell it apart.
        if (event.key !== 'Enter' || event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229) return
        // Enter here would otherwise submit the provider form with the id still unadded.
        event.preventDefault(); add()
      }} />
      <Button type="button" variant="outline" onClick={add}>Add</Button>
    </div>
    {error && <p role="alert" className="text-xs text-destructive-foreground">{error}</p>}
    {refusal?.field === 'models' && <p role="alert" className="text-xs text-destructive-foreground">{refusal.message}</p>}
  </section>
}
