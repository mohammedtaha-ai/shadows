import type { QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { invalidateEntries, threadEntriesKey, type ThreadEntry } from './threads'
import { client, unwrap } from './transport'

type Queued = components['schemas']['Queued']

/** A question the Planner asked, as its entry's card carries it. `chosen` is the answered option's index. */
export interface QuestionCard {
  questionId: string
  question: string
  options: string[]
  chosen: number | null
}

/** The card of a `PersonQuestion` entry, or `null` when it is missing or unreadable. */
export function readQuestionCard(card: ThreadEntry['card']): QuestionCard | null {
  if (card === null) return null
  const { question_id: questionId, question, options, chosen } = card
  if (typeof questionId !== 'string' || typeof question !== 'string' || !Array.isArray(options)) return null
  const labels = options.map(option => (option as { label?: unknown } | null)?.label)
  if (!labels.every((label): label is string => typeof label === 'string')) return null
  return { questionId, question, options: labels, chosen: typeof chosen === 'number' ? chosen : null }
}

/** The person's click on option `option`: their message to the Planner. A second answer is refused as a conflict. */
export function answerQuestion(
  threadId: string,
  questionId: string,
  option: number,
  commandId: string,
): Promise<Queued> {
  return unwrap(
    client.POST('/api/threads/{id}/questions/{question}/answer', {
      params: { path: { id: threadId, question: questionId } },
      body: { command_id: commandId, option },
    }),
  )
}

/** Writes the choice into the entry's card: rewriting the card writes no journal event, so no live frame follows. */
export async function saveAnsweredQuestion(
  queryClient: QueryClient,
  threadId: string,
  questionId: string,
  option: number,
): Promise<void> {
  await writeSaved<ThreadEntry[]>(queryClient, threadEntriesKey(threadId), entries =>
    entries?.map(entry =>
      readQuestionCard(entry.card)?.questionId === questionId
        ? { ...entry, card: { ...entry.card, chosen: option } }
        : entry))
  invalidateEntries(queryClient, threadId)
}
