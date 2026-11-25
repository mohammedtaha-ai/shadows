import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import { client, unwrap, unwrapEmpty } from './transport'
import type { components } from './schema'

export type ExtensionKind = components['schemas']['ExtensionKind']
export type Item = components['schemas']['Item']
export type AgentItem = components['schemas']['AgentItem']
export type AgentExtensions = components['schemas']['AgentExtensions']
export type McpItem = components['schemas']['McpItem']
export type McpInput = components['schemas']['McpInput']
export type McpValueInput = components['schemas']['McpValueInput']
export type ToolRuleEntry = components['schemas']['ToolRuleEntry']
export type ToolRule = components['schemas']['ToolRule']
type AgentTools = components['schemas']['AgentTools']

export function agentToolsQuery(slug: string, id: string) {
  return queryOptions({ queryKey: ['aylul-agent-mcp-tools', slug, id], retry: false,
    queryFn: () => unwrap(client.GET('/api/aylul/agents/{slug}/extensions/mcp/{id}/tools', { params: { path: { slug, id } } })),
  })
}
export function setToolRules(slug: string, id: string, rules: ToolRuleEntry[], commandId: string): Promise<ToolRuleEntry[]> {
  return unwrap(client.PUT('/api/aylul/agents/{slug}/extensions/mcp/{id}/tool-rules', {
    params: { path: { slug, id } }, body: { command_id: commandId, rules },
  }))
}
export function saveToolRulesAnswer(client: QueryClient, slug: string, id: string, rules: ToolRuleEntry[]) {
  return writeSaved<AgentTools>(client, agentToolsQuery(slug, id).queryKey, current => current ? { ...current, rules } : current)
}

export function saveMcp(id: string | null, commandId: string, input: McpInput): Promise<McpItem> {
  const body = { command_id: commandId, ...input }
  return id === null ? unwrap(client.POST('/api/aylul/extensions/mcp', { body }))
    : unwrap(client.PUT('/api/aylul/extensions/mcp/{id}', { params: { path: { id } }, body }))
}

/** The kinds a project may switch for its Planner. Core refuses a plugin. */
export type PlannerExtensionKind = Exclude<ExtensionKind, 'plugin'>

export const extensionKeys = {
  library: () => ['aylul-extensions'],
  agents: () => ['aylul-agent-extensions'],
  agent: (slug: string) => ['aylul-agent-extensions', slug],
  projects: () => ['aylul-project-extensions'],
  project: (projectId: string) => ['aylul-project-extensions', projectId],
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

function withSaved(kind: ExtensionKind, saved: AgentItem) {
  const field = kind === 'plugin' ? 'plugins' : kind === 'skill' ? 'skills' : 'mcp'
  return (current: AgentExtensions | undefined): AgentExtensions | undefined => current
    ? { ...current, [field]: current[field].map(value => value.id === saved.id ? saved : value) } : current
}

export function saveAgentExtensionAnswer(client: QueryClient, slug: string, kind: ExtensionKind, saved: AgentItem) {
  return writeSaved<AgentExtensions>(client, extensionKeys.agent(slug), withSaved(kind, saved))
}

/** GET /api/projects/{id}/planner-extensions: the library with this project's switches. */
export function projectExtensionsQuery(projectId: string) {
  return queryOptions({ queryKey: extensionKeys.project(projectId), retry: false,
    queryFn: () => unwrap(client.GET('/api/projects/{id}/planner-extensions', { params: { path: { id: projectId } } })),
  })
}

/** PUT /api/projects/{id}/planner-extensions/{kind}/{item}, body { command_id, enabled }. */
export function setProjectExtension(projectId: string, kind: PlannerExtensionKind, id: string, enabled: boolean, commandId: string): Promise<AgentItem> {
  return unwrap(client.PUT('/api/projects/{id}/planner-extensions/{kind}/{item}', {
    params: { path: { id: projectId, kind, item: id } }, body: { command_id: commandId, enabled },
  }))
}

/** Writes a saved project switch into that project's view, cancelling a stale read. */
export function saveProjectExtensionAnswer(client: QueryClient, projectId: string, kind: PlannerExtensionKind, saved: AgentItem) {
  return writeSaved<AgentExtensions>(client, extensionKeys.project(projectId), withSaved(kind, saved))
}

/** Refreshes every view that lists the library: the library, each agent's, each project's. */
export async function invalidateLibraryViews(client: QueryClient): Promise<void> {
  await Promise.all([extensionKeys.library(), extensionKeys.agents(), extensionKeys.projects()]
    .map(queryKey => client.invalidateQueries({ queryKey })))
}
