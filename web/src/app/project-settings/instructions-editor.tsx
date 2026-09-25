// One job: the project's Planner instructions (§13.8) as text the person
// edits and saves.
//
// Each save is a new version on the daemon; nothing earlier is overwritten. A
// running Planner hears of the change before its next turn, never mid-turn.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type InstructionsVersion, saveInstructions } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { instructionsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { when } from './when'

export function InstructionsEditor({ projectId }: { projectId: string }) {
  const saved = useQuery(instructionsQuery(projectId))

  return (
    <section className="space-y-3">
      <div className="space-y-1">
        <h2 className="text-sm font-medium">Planner instructions</h2>
        <p className="text-xs text-muted-foreground">
          Given to the Planner in this project, after Shadows&apos; own instructions.
        </p>
      </div>
      {saved.error !== null && <ErrorLine error={saved.error} />}
      {saved.data !== undefined && <Editor projectId={projectId} saved={saved.data} />}
    </section>
  )
}

function Editor({ projectId, saved }: { projectId: string; saved: InstructionsVersion | null }) {
  const queryClient = useQueryClient()
  const [draft, setDraft] = useState(saved?.body ?? '')
  // The version the draft was last in step with. A newer one — this page's
  // own save, or another tab's brought by a refetch — replaces the text only
  // when the person has not changed it since; unsaved edits, including those
  // typed while a save was on its way, are never overwritten.
  const [base, setBase] = useState(saved)
  if ((saved?.number ?? 0) !== (base?.number ?? 0)) {
    setBase(saved)
    if (draft === (base?.body ?? '')) setDraft(saved?.body ?? '')
  }
  const pending = useRef<Attempt | null>(null)

  const save = useMutation({
    mutationFn: ({ commandId, body }: { commandId: string; body: string }) =>
      saveInstructions(projectId, commandId, body),
    onSuccess: (version) => {
      pending.current = null
      queryClient.setQueryData(instructionsQuery(projectId).queryKey, version)
    },
  })

  const submit = () => {
    // A retry of the same text reuses its command id; changed text is new.
    pending.current = attemptFor(pending.current, { body: draft })
    save.mutate({ commandId: pending.current.commandId, body: draft })
  }

  const unchanged = draft === (saved?.body ?? '')

  return (
    <div className="space-y-2">
      <textarea
        dir="auto"
        aria-label="Planner instructions"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        rows={10}
        placeholder="How the Planner should work in this project…"
        className="w-full resize-y rounded-lg border border-input bg-input-background px-3 py-2 text-sm text-foreground outline-none placeholder:text-faint-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
      />
      <div className="flex items-center gap-3">
        <Button size="sm" onClick={submit} disabled={save.isPending || unchanged}>
          Save
        </Button>
        <span className="text-xs text-faint-foreground">
          {saved === null
            ? 'Not saved yet'
            : `Last changed ${when(saved.created_at)} · version ${saved.number}`}
        </span>
      </div>
      {save.error !== null && <ErrorLine error={save.error} />}
    </div>
  )
}
