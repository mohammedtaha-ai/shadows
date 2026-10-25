import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { sessionKeys, sessionQuery, saveSessionAnswer, replaceChoices, replaceCommands, clearCommands } from './sessions'
import type { SessionChoices } from './client'
it('session mutation answers survive late reads and frames during opening are ignored', async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const saved = { current: { model: 'saved' } } as SessionChoices
  let release!: (value: SessionChoices) => void
  const pending = client.fetchQuery({ ...sessionQuery('t1'), queryFn: () => new Promise<SessionChoices>(resolve => { release = resolve }) }).catch(() => undefined)
  replaceChoices(client, 't1', saved)
  expect(client.getQueryData(sessionKeys.choices('t1'))).toBeUndefined()
  await saveSessionAnswer(client, 't1', saved)
  release({ current: { model: 'old' } } as SessionChoices)
  await pending
  expect(client.getQueryData(sessionKeys.choices('t1'))).toEqual(saved)
  expect(sessionKeys.choices('t1')).toEqual(['threads', 't1', 'session'])
  expect(sessionKeys.commands('t1')).toEqual(['threads', 't1', 'commands'])
  replaceCommands(client, 't1', ['one'])
  clearCommands(client, 't1')
  expect(client.getQueryData(sessionKeys.commands('t1'))).toEqual([])
  client.clear()
})
