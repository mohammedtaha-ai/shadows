// A mode is a permission decision: the daemon checks every turn against this
// set, whatever a client shows. Each change is one command; changes are sent
// in order, and the list shows the set as last asked for while they travel.

import { notifyManager, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type Project, setProjectModes } from '@/api/projects'
import { type Attempt, attemptFor } from '@/api/command-id'
import { harnessesQuery, policyOf } from '@/api/harnesses'
import { saveProjectAnswer } from '@/api/projects'
import { ErrorLine } from '../error-line'

type Allowed = Record<string, string[]>

export function ProjectModes({ project }: { project: Project }) {
  const queryClient = useQueryClient()
  const harnesses = useQuery(harnessesQuery).data
  const pending = useRef<Attempt | null>(null)
  // Keep the requested set through cache notification so rapid clicks use the latest choice.
  const [asked, setAsked] = useState<Allowed | null>(null)
  const latest = useRef<Allowed | null>(null)
  // Clear on the query notification schedule so stale props cannot flash an outdated set.
  const settle = (allowed: Allowed) =>
    notifyManager.schedule(() => {
      if (latest.current === allowed) setAsked(null)
    })

  const save = useMutation({
    // One project's changes wait for each other, so the last one asked for
    // is the one that stands.
    scope: { id: `project-modes-${project.id}` },
    mutationFn: ({ commandId, allowed }: { commandId: string; allowed: Allowed }) =>
      setProjectModes(project.id, commandId, allowed),
    onSuccess: async (saved, { allowed }) => {
      await saveProjectAnswer(queryClient, saved)
      settle(allowed)
    },
    // A refused change shows the set the daemon holds, with the error.
    onError: (_error, { allowed }) => settle(allowed),
  })

  const allowed: Allowed = asked ?? project.allowed_modes

  const toggle = (harness: string, mode: string, on: boolean) => {
    const order = policyOf(harnesses, harness).modes.map((m) => m.id)
    const current = new Set(allowed[harness] ?? [])
    if (on) current.add(mode)
    else current.delete(mode)
    const next = { ...allowed, [harness]: order.filter((id) => current.has(id)) }
    // A new set is a new command; only a retry of the same set reuses its id.
    latest.current = next
    setAsked(next)
    pending.current = attemptFor(pending.current, next)
    save.mutate({ commandId: pending.current.commandId, allowed: next })
  }

  return (
    <section className="w-full max-w-sm space-y-3 text-left">
      <h2 className="text-sm font-medium">Allowed modes</h2>
      {(harnesses ?? []).map((h) => {
        const modes = allowed[h.kind] ?? []
        return (
          <fieldset key={h.kind} className="space-y-1.5">
            <legend className="text-xs text-muted-foreground">{h.label}</legend>
            {h.modes.length === 0 ? (
              <p className="text-xs text-faint-foreground">
                Modes are decided when {h.label} is enabled.
              </p>
            ) : (
              <>
                {h.modes.map((m) => (
                  <label key={m.id} className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={modes.includes(m.id)}
                      onChange={(e) => toggle(h.kind, m.id, e.target.checked)}
                      className="size-4 accent-(--accent-line)"
                    />
                    {m.label}
                  </label>
                ))}
                {modes.length === 0 && (
                  <p className="text-xs text-destructive-foreground">
                    No mode left: turns cannot start on {h.label} in this project.
                  </p>
                )}
              </>
            )}
          </fieldset>
        )
      })}
      {save.error !== null && <ErrorLine error={save.error} />}
    </section>
  )
}
