// AYLUL configuration calls over the shared generated transport.
import { queryOptions } from '@tanstack/react-query'
import { client } from './client'
import { unwrap, unwrapEmpty } from './error'
import type { components } from './schema'

export type Provider = components['schemas']['Provider']
export type ProviderInput = components['schemas']['ProviderInput']
export type Agent = components['schemas']['Agent']
export type AgentInput = components['schemas']['AgentInput']
export type EnvEntry = components['schemas']['EnvEntry']
export type ProviderPreset = components['schemas']['ProviderPreset']
export type ProviderTestOutcome = components['schemas']['ProviderTestOutcome']
export type ProviderTest = components['schemas']['ProviderTest']

export const providerPresetsQuery = queryOptions({
  queryKey: ['aylul-provider-presets'], staleTime: Infinity,
  queryFn: () => unwrap(client.GET('/api/aylul/provider-presets')),
})
export function testProvider(body: ProviderTest): Promise<ProviderTestOutcome> {
  return unwrap(client.POST('/api/aylul/providers/test', { body }))
}
export function testSavedProvider(slug: string, apiKey: string | null): Promise<ProviderTestOutcome> {
  return unwrap(client.POST('/api/aylul/providers/{slug}/test', {
    params: { path: { slug } }, body: apiKey === null ? {} : { api_key: apiKey },
  }))
}

export const providersQuery = queryOptions({
  queryKey: ['aylul-providers'], queryFn: () => unwrap(client.GET('/api/aylul/providers')),
})
export const agentsQuery = queryOptions({
  queryKey: ['aylul-agents'], queryFn: () => unwrap(client.GET('/api/aylul/agents')),
})
export function agentThreadsQuery(slug: string) {
  return queryOptions({ queryKey: ['aylul-agent', slug, 'threads'],
    queryFn: () => unwrap(client.GET('/api/aylul/agents/{slug}/threads', {
      params: { path: { slug } },
    })), refetchInterval: 10_000 })
}
export function agentEnvQuery(slug: string) {
  return queryOptions({ queryKey: ['aylul-agent', slug, 'env'],
    queryFn: () => unwrap(client.GET('/api/aylul/agents/{slug}/env', {
      params: { path: { slug } },
    })) })
}
export function saveProvider(slug: string | null, commandId: string, input: ProviderInput) {
  const body = { command_id: commandId, ...input }
  return slug === null ? unwrap(client.POST('/api/aylul/providers', { body }))
    : unwrap(client.PUT('/api/aylul/providers/{slug}', { params: { path: { slug } }, body }))
}
export function deleteProvider(slug: string, commandId: string) {
  return unwrapEmpty(client.DELETE('/api/aylul/providers/{slug}', {
    params: { path: { slug }, query: { command_id: commandId } },
  }))
}
export function setProviderKey(slug: string, commandId: string, value: string) {
  return unwrap(client.PUT('/api/aylul/providers/{slug}/key', {
    params: { path: { slug } }, body: { command_id: commandId, value },
  }))
}
export function removeProviderKey(slug: string, commandId: string) {
  return unwrapEmpty(client.DELETE('/api/aylul/providers/{slug}/key', {
    params: { path: { slug }, query: { command_id: commandId } },
  }))
}
export function saveAgent(slug: string | null, commandId: string, input: AgentInput) {
  const body = { command_id: commandId, ...input }
  return slug === null ? unwrap(client.POST('/api/aylul/agents', { body }))
    : unwrap(client.PUT('/api/aylul/agents/{slug}', { params: { path: { slug } }, body }))
}
export function deleteAgent(slug: string, commandId: string) {
  return unwrapEmpty(client.DELETE('/api/aylul/agents/{slug}', {
    params: { path: { slug }, query: { command_id: commandId } },
  }))
}
export function setAgentEnv(slug: string, name: string, commandId: string, value: string) {
  return unwrap(client.PUT('/api/aylul/agents/{slug}/env/{name}', {
    params: { path: { slug, name } }, body: { command_id: commandId, value },
  }))
}
export function removeAgentEnv(slug: string, name: string, commandId: string) {
  return unwrapEmpty(client.DELETE('/api/aylul/agents/{slug}/env/{name}', {
    params: { path: { slug, name }, query: { command_id: commandId } },
  }))
}
