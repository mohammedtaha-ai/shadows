import { queryOptions } from '@tanstack/react-query'
import { client } from './client'
import { unwrap } from './error'
import type { components } from './schema'

export type UsageReport = components['schemas']['UsageReport']
export type ProviderUsage = components['schemas']['ProviderUsage']
export type ModelUsage = components['schemas']['ModelUsage']
export function usageQuery(from: string, to: string) {
  return queryOptions({ queryKey: ['aylul-usage', from, to], retry: false,
    queryFn: () => unwrap(client.GET('/api/aylul/usage', { params: { query: { from, to } } })),
  })
}
