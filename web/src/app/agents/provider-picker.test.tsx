// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { ProviderPicker } from './provider-picker'
import { PRESETS, presetFor, presetGroups } from './provider-presets'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })
it('presets are grouped by name', () => {
  const groups = presetGroups(PRESETS)
  expect(groups.map(group => group.name)).toEqual(['MiniMax', 'Anthropic', 'DeepSeek', 'OpenRouter', 'Moonshot (Kimi)'])
  expect(groups[0].presets.map(preset => preset.id)).toEqual(['minimax', 'minimax-china'])
  for (const preset of PRESETS) expect(presetFor(preset.baseUrl)).toBe(preset)
  expect(presetFor('https://example.com')).toBeNull()
})
it('search filters the presets and keeps Custom', async () => {
  const host = document.createElement('div'); document.body.append(host); const root = createRoot(host)
  try {
    await act(async () => root.render(<ProviderPicker onChoose={vi.fn()} />))
    const search = host.querySelector('input')!
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!
    act(() => { setter.call(search, 'mini'); search.dispatchEvent(new Event('input', { bubbles: true })) })
    expect([...host.querySelectorAll('[aria-pressed]')].map(button => button.textContent)).toEqual(['MiniMax', 'Custom'])
    act(() => { setter.call(search, 'zzz'); search.dispatchEvent(new Event('input', { bubbles: true })) })
    expect([...host.querySelectorAll('[aria-pressed]')].map(button => button.textContent)).toEqual(['Custom'])
    expect(host.textContent).toContain('No provider matches.')
  } finally { await act(async () => root.unmount()); host.remove() }
})
