// AYLUL environment metadata and write-only value calls over the shared transport.
import { client } from './client'
import { unwrap, unwrapEmpty } from './error'
import type { components } from './schema'

export type EnvEntry = components['schemas']['EnvEntry']

export const aylulEnvQuery = { queryKey: ['aylul-env'], queryFn: listAylulEnv }

export function listAylulEnv(): Promise<EnvEntry[]> {
  return unwrap(client.GET('/api/aylul/env'))
}

export function setAylulEnv(name: string, commandId: string, value: string): Promise<EnvEntry> {
  return unwrap(client.PUT('/api/aylul/env/{name}', {
    params: { path: { name } }, body: { command_id: commandId, value },
  }))
}

export function removeAylulEnv(name: string, commandId: string): Promise<void> {
  return unwrapEmpty(client.DELETE('/api/aylul/env/{name}', {
    params: { path: { name }, query: { command_id: commandId } },
  }))
}
