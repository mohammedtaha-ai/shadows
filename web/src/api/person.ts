import { queryOptions } from '@tanstack/react-query'
import { client } from './client'
import { unwrap } from './error'
import type { components } from './schema'

export type Person = components['schemas']['Person']
export type PersonInput = components['schemas']['PersonInput']
export const personQuery = queryOptions({ queryKey: ['person'], queryFn: () => unwrap(client.GET('/api/me')) })
export function savePerson(commandId: string, input: PersonInput): Promise<Person> {
  return unwrap(client.PUT('/api/me', { body: { ...input, command_id: commandId } }))
}
