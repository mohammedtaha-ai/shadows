import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import { client, unwrap } from './transport'
import type { components } from './schema'

export type Person = components['schemas']['Person']
export type PersonInput = components['schemas']['PersonPatch']
export const personKeys = { record: () => ['person'] }
export const personQuery = queryOptions({ queryKey: personKeys.record(), queryFn: () => unwrap(client.GET('/api/me')) })
export function savePerson(commandId: string, input: PersonInput): Promise<Person> {
  return unwrap(client.PATCH('/api/me', { body: { ...input, command_id: commandId } }))
}

export function savePersonAnswer(client: QueryClient, person: Person) {
  return writeSaved(client, personKeys.record(), person)
}
