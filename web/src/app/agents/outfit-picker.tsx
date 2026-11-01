import { OUTFITS, outfitOf } from './outfits'

export function OutfitPicker({ value, onChange, disabled }: {
  value: string; onChange: (id: string) => void; disabled?: boolean
}) {
  return <div role="radiogroup" aria-label="Outfit" className="grid grid-cols-5 gap-2">
    {OUTFITS.map(outfit => <label key={outfit.id} className="relative cursor-pointer text-center has-checked:[filter:drop-shadow(0_0_7px_var(--accent-line))]">
      <input type="radio" name="outfit" aria-label={outfit.label} value={outfit.id}
        checked={outfitOf(value).id === outfit.id} disabled={disabled}
        onChange={() => onChange(outfit.id)}
        className="peer sr-only" />
      <span className="flex flex-col items-center rounded-lg border border-border p-1 text-xs peer-checked:border-accent-line peer-checked:bg-accent-softer peer-checked:ring-1 peer-checked:ring-accent-line peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-disabled:opacity-50">
        <img src={outfit.image} alt="" width={160} height={160} className="aspect-square w-full object-contain" />
        {outfit.label}
      </span>
    </label>)}
  </div>
}
