import { expect, it } from 'vitest'
import { CHARACTER_COLORS, DEFAULT_COLOR, colorOf } from './character-colors'

it("the table's colours, in order", () => {
  expect(CHARACTER_COLORS.map(color => color.id)).toEqual(['blue', 'green', 'purple', 'orange', 'pink'])
})
it('every colour is complete', () => {
  expect(new Set(CHARACTER_COLORS.map(color => color.id)).size).toBe(CHARACTER_COLORS.length)
  for (const color of CHARACTER_COLORS) {
    expect(color.id).toMatch(/^[a-z][a-z0-9-]{0,31}$/)
    expect(color.label.trim()).not.toBe('')
    expect(color.swatch).toMatch(/^#[0-9a-f]{6}$/i)
    expect(Object.keys(color).sort()).toEqual(['id', 'label', 'swatch'])
  }
  expect(CHARACTER_COLORS.some(color => color.id === DEFAULT_COLOR)).toBe(true)
})
it('an unknown or missing id reads as purple', () => {
  for (const id of ['red', 'teal', '', null, undefined]) expect(colorOf(id)).toBe(colorOf(DEFAULT_COLOR))
  for (const color of CHARACTER_COLORS) expect(colorOf(color.id)).toBe(color)
})
