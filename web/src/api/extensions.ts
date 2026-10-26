import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import { client, unwrap, unwrapEmpty } from './transport'
import type { components } from './schema'

export type ExtensionKind = components['schemas']['ExtensionKind']
export type Item = components['schemas']['Item']
export type AgentItem = components['schemas']['AgentItem']
export type ExtensionLibrary = components['schemas']['ExtensionLibrary']
export type AgentExtensions = components['schemas']['AgentExtensions']
export type ExtensionImport = components['schemas']['Import']

export const extensionKeys = {
  library: () => ['aylul-extensions'],
  agents: () => ['aylul-agent-extensions'],
  agent: (slug: string) => ['aylul-agent-extensions', slug],
}

export const extensionsQuery = queryOptions({ queryKey: extensionKeys.library(), retry: false,
  queryFn: () => unwrap(client.GET('/api/aylul/extensions')),
})
export function addExtension(commandId: string, kind: ExtensionKind, path: string): Promise<Item> {
  return unwrap(client.POST('/api/aylul/extensions', { body: { command_id: commandId, kind, path } }))
}
export function removeExtension(commandId: string, kind: ExtensionKind, id: string): Promise<void> {
  return unwrapEmpty(client.DELETE('/api/aylul/extensions/{kind}/{id}', { params: { path: { kind, id }, query: { command_id: commandId } } }))
}
export function agentExtensionsQuery(slug: string) {
  return queryOptions({ queryKey: extensionKeys.agent(slug), retry: false,
    queryFn: () => unwrap(client.GET('/api/aylul/agents/{slug}/extensions', { params: { path: { slug } } })),
  })
}
export function setAgentExtension(slug: string, kind: ExtensionKind, id: string, enabled: boolean, commandId: string): Promise<AgentItem> {
  return unwrap(client.PUT('/api/aylul/agents/{slug}/extensions/{kind}/{id}', {
    params: { path: { slug, kind, id } }, body: { command_id: commandId, enabled },
  }))
}

export function saveAgentExtensionAnswer(client: QueryClient, slug: string, kind: ExtensionKind, saved: AgentItem) {
  const field = kind === 'plugin' ? 'plugins' : 'skills'
  return writeSaved<AgentExtensions>(client, extensionKeys.agent(slug), current => current
    ? { ...current, [field]: current[field].map(value => value.id === saved.id ? saved : value) } : current)
}
