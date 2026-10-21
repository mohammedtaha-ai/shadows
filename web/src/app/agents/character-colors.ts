import table from '../../../../assets/aylul/character-colors.json'

export type CharacterColor = { id: string; label: string; swatch: string }
export const CHARACTER_COLORS: readonly CharacterColor[] = table.colors.map(({ id, label, swatch }) => ({ id, label, swatch }))
export const DEFAULT_COLOR = table.base
export function colorOf(id: string | null | undefined): CharacterColor {
  return CHARACTER_COLORS.find(color => color.id === id) ?? CHARACTER_COLORS.find(color => color.id === DEFAULT_COLOR)!
}
