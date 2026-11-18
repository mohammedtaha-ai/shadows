import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { projectKeys, projectsQuery, instructionsQuery, saveProjectAnswer, saveRemovedProjectAnswer, saveInstructionsAnswer } from './projects'
import type { Project, InstructionsVersion } from './projects'
import { type ExternalConnection, connectionsQuery, saveRevokedConnection } from './external-connections'
it.each(['modes', 'remove', 'instructions', 'revoke'])('%s answers survive a stale in-flight read', async kind => {
  const client = new QueryClient()
  const project = { id: 'p', name: 'saved' } as Project
  const grant = { id: 'g', revoked_at: 'now' } as ExternalConnection
  const instructions = { number: 2, body: 'saved' } as InstructionsVersion
  const key = kind === 'instructions' ? projectKeys.instructions('p') : kind === 'revoke' ? connectionsQuery.queryKey : projectKeys.all()
  const old = kind === 'instructions' ? null : kind === 'revoke' ? [{ ...grant, revoked_at: null }] : [{ ...project, name: 'old' }]
  client.setQueryData(key, old)
  let release!: (value: unknown) => void
  const pending = client.fetchQuery({ queryKey: key, queryFn: () => new Promise(resolve => { release = resolve }) }).catch(() => undefined)
  if (kind === 'modes') await saveProjectAnswer(client, project)
  if (kind === 'remove') await saveRemovedProjectAnswer(client, 'p')
  if (kind === 'instructions') await saveInstructionsAnswer(client, 'p', instructions)
  if (kind === 'revoke') await saveRevokedConnection(client, grant)
  release(old); await pending
  expect(client.getQueryData(key)).toEqual(kind === 'instructions' ? instructions : kind === 'remove' ? [] : kind === 'revoke' ? [grant] : [project])
  client.clear()
})
it('project reads retain polling identities', () => {
  expect(projectsQuery.queryKey).toEqual(['projects'])
  expect(instructionsQuery('p').queryKey).toEqual(['projects', 'p', 'instructions'])
  expect(connectionsQuery.queryKey).toEqual(['external-connections'])
})
