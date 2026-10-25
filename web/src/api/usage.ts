import { queryOptions } from '@tanstack/react-query'
import { client } from './client'
import { unwrap } from './error'
import type { components } from './schema'

export type UsageReport = components['schemas']['UsageReport']
export type ProviderUsage = components['schemas']['ProviderUsage']
export type ModelUsage = components['schemas']['ModelUsage']
export const usageKeys = {
  all: () => ['aylul-usage'],
  report: (from: string, to: string) => ['aylul-usage', from, to],
}
export function usageQuery(from: string, to: string) {
  return queryOptions({ queryKey: usageKeys.report(from, to), retry: false,
    queryFn: () => unwrap(client.GET('/api/aylul/usage', { params: { query: { from, to } } })),
  })
}
