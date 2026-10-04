// A revision-aware vision form that preserves unsaved text across refetches.
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import { editDesign, type VisionContent, type VisionView } from '@/api/design'
import { ApiError } from '@/api/error'
import { visionQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

const FIELDS: [keyof VisionContent, string][] = [
  ['purpose', 'Purpose'], ['users', 'Users'], ['goals', 'Goals'],
  ['boundaries', 'Boundaries'], ['technical_direction', 'Technical direction'],
]

function same(a: VisionContent, b: VisionContent) {
  return FIELDS.every(([key]) => a[key] === b[key])
}

export function VisionEditor({ projectId }: { projectId: string }) {
  const vision = useQuery(visionQuery(projectId))
  return (
    <section>
      {vision.isPending && <p>Loading vision…</p>}
      {vision.error !== null && <ErrorLine error={vision.error} />}
      {vision.data !== undefined && <Editor key={projectId} projectId={projectId} saved={vision.data} />}
    </section>
  )
}

function Editor({ projectId, saved }: { projectId: string; saved: VisionView }) {
  const client = useQueryClient()
  const [base, setBase] = useState(saved)
  const [draft, setDraft] = useState(saved.content)
  const [conflict, setConflict] = useState(false)
  const [reloading, setReloading] = useState(false)
  const [reloadError, setReloadError] = useState<Error | null>(null)
  const attempt = useRef<Attempt | null>(null)
  const dirty = !same(draft, base.content)
  if (saved.revision > base.revision && !dirty) {
    setBase(saved)
    setDraft(saved.content)
  }
  const newer = conflict || saved.revision > base.revision
  const save = useMutation({
    mutationFn: ({ commandId, revision, content }: { commandId: string; revision: number; content: VisionContent }) =>
      editDesign(projectId, { command_id: commandId, expected_revision: revision, ops: [{ kind: 'VisionPut', content }] }),
    onSuccess: (change, sent) => {
      attempt.current = null
      setConflict(false)
      const view = { revision: change.revision, content: sent.content }
      setBase(view)
      // Text typed while saving remains in draft; the submitted content is the new base.
      client.setQueryData(visionQuery(projectId).queryKey, view)
      void client.invalidateQueries({ queryKey: visionQuery(projectId).queryKey })
    },
    onError: (error) => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'REVISION_CONFLICT') {
        setConflict(true)
        void client.invalidateQueries({ queryKey: visionQuery(projectId).queryKey })
      }
    },
  })
  const submit = () => {
    attempt.current = attemptFor(attempt.current, { expected_revision: base.revision, content: draft })
    save.mutate({ commandId: attempt.current.commandId, revision: base.revision, content: draft })
  }
  const reload = async () => {
    setReloading(true)
    setReloadError(null)
    try {
      const view = await client.fetchQuery({ ...visionQuery(projectId), staleTime: 0 })
      setBase(view)
      setDraft(view.content)
      setConflict(false)
      attempt.current = null
      save.reset()
    } catch (error) {
      setReloadError(error instanceof Error ? error : new Error(String(error)))
    } finally {
      setReloading(false)
    }
  }
  return (
    <div className="space-y-4">
      {newer && <div role="status" className="rounded-md border p-3 text-sm">
        Newer vision data exists. Your unsaved text is kept. Reload replaces it with the saved vision.
        <Button size="sm" variant="secondary" className="ml-3" onClick={() => void reload()} disabled={reloading || save.isPending}>Reload</Button>
      </div>}
      {FIELDS.map(([key, label]) => <label key={key} className="block space-y-1 text-sm">
        <span>{label}</span>
        <textarea dir="auto" aria-label={label} rows={4} value={draft[key]}
          onChange={(event) => setDraft({ ...draft, [key]: event.target.value })}
          disabled={reloading}
          className="w-full resize-y rounded-md border border-input bg-input-background px-3 py-2 outline-none focus-visible:ring-2 focus-visible:ring-ring" />
      </label>)}
      <div className="flex items-center gap-3">
        <Button size="sm" onClick={submit} disabled={!dirty || save.isPending || reloading}>Save vision</Button>
        <span className="text-xs text-muted-foreground">Revision {base.revision}{dirty ? ' · Unsaved changes' : ' · Saved'}</span>
      </div>
      {save.error !== null && <ErrorLine error={save.error} />}
      {reloadError !== null && <ErrorLine error={reloadError} />}
    </div>
  )
}
