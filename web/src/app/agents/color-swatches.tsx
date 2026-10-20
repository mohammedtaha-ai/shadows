import { CHARACTER_COLORS, colorOf } from './character-colors'

export function ColorSwatches({ value, onChange, disabled }: { value: string; onChange: (id: string) => void; disabled?: boolean }) {
  return <div role="radiogroup" aria-label="Character colour" className="flex gap-3">
    {CHARACTER_COLORS.map(color => <label key={color.id} title={color.label} className="relative flex cursor-pointer items-center rounded-full">
      <input type="radio" name="character-color" aria-label={color.label} value={color.id} title={color.label}
        checked={colorOf(value).id === color.id} disabled={disabled} onChange={() => onChange(color.id)}
        className="peer size-7 cursor-pointer appearance-none rounded-full border-2 border-background checked:ring-2 checked:ring-foreground checked:ring-offset-2 checked:ring-offset-background focus-visible:outline-2 focus-visible:outline-offset-4 disabled:cursor-default disabled:opacity-50"
        style={{ backgroundColor: color.swatch }} />
    </label>)}
  </div>
}

export function ColorDot({ id }: { id: string | null | undefined }) {
  const color = colorOf(id)
  return <span aria-hidden="true" data-color={color.id} className="inline-block size-2.5 shrink-0 rounded-full" style={{ backgroundColor: color.swatch }} />
}
