import { beforeEach, expect, it, vi } from 'vitest'
import { useCloseSettings, useOpenSettings } from './index'

const navigate = vi.hoisted(() => vi.fn())
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => navigate }))
beforeEach(() => navigate.mockClear())

it('opens settings over the current route while preserving unrelated search', () => {
  useOpenSettings()('agents', 'aylul')
  const options = navigate.mock.calls[0]![0]
  expect(options.to).toBe('.')
  expect(options.search({ plan: 'plan-1', settings: 'general' })).toEqual({ plan: 'plan-1', settings: 'agents', agent: 'aylul' })
})
it('opening another section clears the selected agent', () => {
  useOpenSettings()('providers', 'aylul')
  expect(navigate.mock.calls[0]![0].search({ agent: 'old', plan: 'plan-1' })).toEqual({ plan: 'plan-1', settings: 'providers', agent: undefined })
})
it('closing settings clears only settings search keys', () => {
  useCloseSettings()()
  const options = navigate.mock.calls[0]![0]
  expect(options.to).toBe('.')
  expect(options.search({ settings: 'agents', agent: 'aylul', plan: 'plan-1' })).toEqual({ settings: undefined, agent: undefined, plan: 'plan-1' })
})
