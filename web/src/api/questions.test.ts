import { QueryClient } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { entryOfKind } from '@/test/contract-fixtures'
import { answerQuestion, readQuestionCard, saveAnsweredQuestion } from './questions'
import { threadEntriesKey } from './threads'

afterEach(() => vi.unstubAllGlobals())

const card = { question_id: 'q1', question: 'Which?', options: [{ label: 'A' }, { label: 'B' }], chosen: null }

it('answers through the generated route with the caller command', async () => {
  const calls: Request[] = []
  vi.stubGlobal('fetch', async (request: Request) => {
    calls.push(request)
    return Response.json({ id: 'm1' }, { status: 202 })
  })
  await answerQuestion('t1', 'q1', 1, 'cmd1')
  expect(new URL(calls[0].url).pathname).toBe('/api/threads/t1/questions/q1/answer')
  expect(calls[0].method).toBe('POST')
  expect(await calls[0].json()).toEqual({ command_id: 'cmd1', option: 1 })
})

it('reads a question card and refuses a broken one', () => {
  expect(readQuestionCard(card)).toEqual({ questionId: 'q1', threadId: null, question: 'Which?', options: ['A', 'B'], chosen: null })
  expect(readQuestionCard({ ...card, thread_id: 't0' })?.threadId).toBe('t0')
  expect(readQuestionCard({ ...card, chosen: 1 })?.chosen).toBe(1)
  expect(readQuestionCard({ ...card, options: [1] })).toBeNull()
  expect(readQuestionCard(null)).toBeNull()
})

it('writes the chosen option into the cached entry', async () => {
  const queryClient = new QueryClient()
  const entry = { ...entryOfKind('e1', 'PersonQuestion', 'Which?'), card }
  queryClient.setQueryData(threadEntriesKey('t1'), [entry])
  await saveAnsweredQuestion(queryClient, 't1', 'q1', 0)
  const saved = queryClient.getQueryData<(typeof entry)[]>(threadEntriesKey('t1'))!
  expect(readQuestionCard(saved[0].card)?.chosen).toBe(0)
})
