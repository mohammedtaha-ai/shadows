import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { PRESETS, presetGroups, type PresetGroup } from './provider-presets'

export function ProviderPicker({ onChoose }: { onChoose: (group: PresetGroup | null) => void }) {
  const [search, setSearch] = useState('')
  const [picked, setPicked] = useState<PresetGroup | null | undefined>()
  const groups = presetGroups(PRESETS).filter(group => group.name.toLowerCase().includes(search.trim().toLowerCase()))
  return <div className="space-y-4">
    <Input aria-label="Search providers" placeholder="Search providers" value={search} onChange={event => setSearch(event.target.value)} />
    <div className="grid grid-cols-2 gap-3">
      {groups.map(group => <button type="button" key={group.name} aria-pressed={picked?.name === group.name}
        className="rounded-lg border border-border p-4 text-left aria-pressed:border-accent-line aria-pressed:bg-secondary"
        onClick={() => setPicked(group)}>{group.name}</button>)}
      <button type="button" aria-pressed={picked === null} className="rounded-lg border border-border p-4 text-left aria-pressed:border-accent-line aria-pressed:bg-secondary"
        onClick={() => setPicked(null)}>Custom</button>
    </div>
    {groups.length === 0 && <p className="text-sm text-muted-foreground">No provider matches.</p>}
    <Button type="button" disabled={picked === undefined} onClick={() => { if (picked !== undefined) onChoose(picked) }}>Next</Button>
  </div>
}
