// One job: a question the Planner asked the person, drawn as buttons until it
// is answered, and as the chosen option after.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { attemptFor, type Attempt } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { answerQuestion, readQuestionCard, saveAnsweredQuestion } from '@/api/questions'
import { invalidateEntries, type ThreadEntry } from '@/api/threads'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

function daemonCode(error: unknown): string | null {
  return error instanceof ApiError && error.problem.kind === 'daemon' ? error.problem.code : null
}

export function PersonQuestionCard({ entry }: { entry: ThreadEntry }) {
  const card = readQuestionCard(entry.card)
  const queryClient = useQueryClient()
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  // The card never offers buttons again once Core says it was answered or is another conversation's.
  const [closed, setClosed] = useState(false)
  const answer = useMutation({
    retry: false,
    mutationFn: ({ option }: { option: number }) => {
      attempt.current = attemptFor(attempt.current, { question: card?.questionId, option })
      return answerQuestion(entry.thread_id, card!.questionId, option, attempt.current.commandId)
    },
    onSuccess: async (_queued, { option }) => {
      attempt.current = null
      await saveAnsweredQuestion(queryClient, entry.thread_id, card!.questionId, option)
    },
    onError: error => {
      const code = daemonCode(error)
      if (code === 'COMMAND_CONFLICT') {
        setClosed(true)
        invalidateEntries(queryClient, entry.thread_id)
      } else if (code === 'INVALID_COMMAND') {
        setClosed(true)
      }
    },
    onSettled: () => { locked.current = false },
  })
  if (card === null) {
    return <p dir="auto" className="text-xs text-faint-foreground">{entry.body}</p>
  }
  const chosen = card.chosen ?? (answer.isSuccess ? answer.variables.option : null)
  const offering = chosen === null && !closed
  const refused = daemonCode(answer.error) === 'INVALID_COMMAND' ? null : answer.error
  return (
    <section aria-label="Question" className="surface-glow space-y-3 rounded-lg p-4">
      <p dir="auto" className="text-sm font-medium whitespace-pre-wrap">{card.question}</p>
      {offering ? (
        <div className="flex flex-wrap gap-2">
          {card.options.map((label, index) => (
            <Button key={index} type="button" variant="outline" disabled={answer.isPending}
              onClick={() => {
                if (locked.current) return
                locked.current = true
                answer.mutate({ option: index })
              }}>
              {label}
            </Button>
          ))}
        </div>
      ) : chosen !== null ? (
        <p dir="auto" className="text-xs text-success">You chose: {card.options[chosen] ?? ''}</p>
      ) : (
        <ul className="list-disc space-y-0.5 ps-5 text-xs text-muted-foreground">
          {card.options.map((label, index) => <li key={index} dir="auto">{label}</li>)}
        </ul>
      )}
      {refused && <ErrorLine error={refused} />}
    </section>
  )
}
