// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { providerPresetsQuery } from '@/api/aylul-profiles'
import { expect, it, vi } from 'vitest'
import { ProviderPicker } from './provider-picker'
import { presetGroups, type Preset } from './provider-presets'
import { answers } from '@/test/fake-daemon'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { until } from '../test-app'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })
it('presets are grouped by name', () => {
  const presets = answers()['GET /api/aylul/provider-presets'] as Preset[]
  const groups = presetGroups(presets)
  expect(groups.map(group => group.name)).toEqual(['MiniMax', 'Anthropic', 'DeepSeek', 'OpenRouter', 'Moonshot (Kimi)'])
  expect(groups[0].presets.map(preset => preset.id)).toEqual(['minimax', 'minimax-china'])

})
it('search filters the presets and keeps Custom', async () => {
  const host = document.createElement('div'); document.body.append(host); const root = createRoot(host)
  const client = new QueryClient()
  client.setQueryData(providerPresetsQuery.queryKey, answers()['GET /api/aylul/provider-presets'] as Preset[])
  try {
    await act(async () => root.render(<QueryClientProvider client={client}><ProviderPicker onChoose={vi.fn()} /></QueryClientProvider>))
    await until(() => host.textContent?.includes('MiniMax') === true)
    const search = host.querySelector('input')!
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!
    act(() => { setter.call(search, 'mini'); search.dispatchEvent(new Event('input', { bubbles: true })) })
    expect([...host.querySelectorAll('[aria-pressed]')].map(button => button.textContent)).toEqual(['MiniMax', 'Custom'])
    act(() => { setter.call(search, 'zzz'); search.dispatchEvent(new Event('input', { bubbles: true })) })
    expect([...host.querySelectorAll('[aria-pressed]')].map(button => button.textContent)).toEqual(['Custom'])
    expect(host.textContent).toContain('No provider matches.')
  } finally { await act(async () => root.unmount()); host.remove(); client.clear() }
})
