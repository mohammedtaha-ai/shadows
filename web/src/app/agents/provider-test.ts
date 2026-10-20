import type { ProviderTestOutcome } from '@/api/aylul-profiles'

export function testLine(outcome: ProviderTestOutcome, providerName: string): { kind: 'ok' | 'error'; text: string } {
  if (outcome.ok) return { kind: 'ok', text: outcome.listed ? 'Key works. Choose the models to add.' : 'Key works. This provider does not list its models: add them by id.' }
  const text = outcome.reason === 'key_refused' ? 'The provider refused this key.' :
    outcome.reason === 'unreachable' ? `Could not reach ${providerName}. Check your connection and retry.` : outcome.message || 'The test failed.'
  return { kind: 'error', text }
}
