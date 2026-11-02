// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import manifest from '@/assets/outfits/manifest.json'
import { OutfitPicker } from './outfit-picker'

let root: Root | undefined
afterEach(() => { act(() => root?.unmount()); document.body.replaceChildren() })
function render(value: string, onChange = vi.fn(), disabled = false) {
  const container = document.createElement('div')
  document.body.append(container)
  root = createRoot(container)
  act(() => root!.render(<OutfitPicker value={value} onChange={onChange} disabled={disabled} />))
  return container
}

it('every manifest outfit is a radio with its label and image', () => {
  const container = render('moon')
  const radios = [...container.querySelectorAll<HTMLInputElement>('input[type=radio]')]
  expect(radios.map(radio => radio.value)).toEqual(manifest.map(outfit => outfit.id))
  for (const [index, outfit] of manifest.entries()) {
    expect(radios[index].getAttribute('aria-label')).toBe(outfit.label)
    expect(radios[index].checked).toBe(outfit.id === 'moon')
    const image = radios[index].closest('label')!.querySelector('img')!
    expect(image.getAttribute('src')).toContain(`${outfit.id}.png`)
    expect(image.alt).toBe('')
  }
})

it('an unknown id selects default', () => {
  const container = render('future-outfit')
  expect(container.querySelector<HTMLInputElement>('input:checked')!.value).toBe('default')
})

it('choosing an outfit reports its id and a disabled picker cannot change', () => {
  const change = vi.fn()
  const container = render('default', change)
  act(() => container.querySelector<HTMLInputElement>('input[value=crown]')!.click())
  expect(change).toHaveBeenCalledWith('crown')
  act(() => root!.render(<OutfitPicker value="default" onChange={change} disabled />))
  change.mockClear()
  for (const radio of container.querySelectorAll<HTMLInputElement>('input')) expect(radio.disabled).toBe(true)
  act(() => container.querySelector<HTMLInputElement>('input[value=moon]')!.click())
  expect(change).not.toHaveBeenCalled()
})
