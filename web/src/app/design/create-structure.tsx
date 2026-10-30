// The person's structure confirmation, write and retained result.
import { useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createStructure, type StructureWritten } from '@/api/design'
import { ApiError } from '@/api/error'
import { stageQuery } from '@/api/design'
import { projectsQuery } from '@/api/projects'
import { designKeys } from '@/api/design'
import { Button } from '@/components/ui/button'

export function CreateStructure({ projectId }: {projectId:string}) {
  const client = useQueryClient()
  const stage = useQuery(stageQuery(projectId))
  const projects = useQuery(projectsQuery)
  const project = projects.data?.find(p => p.id === projectId)
  const [confirming, setConfirming] = useState(false)
  const [result, setResult] = useState<StructureWritten | null>(null)
  const [moved, setMoved] = useState(false)
  // One command id per attempt: the revision is part of the fingerprint, so a new
  // revision is a new attempt. An uncertain response retries the same attempt.
  const command = useRef<{ id: string; revision: number } | null>(null)
  const write = useMutation({
    mutationFn: (revision: number) => {
      if (command.current?.revision !== revision) command.current = { id: crypto.randomUUID(), revision }
      return createStructure(projectId, command.current.id, revision)
    },
    onSuccess: written => {
      setResult(written)
      setConfirming(false)
      command.current = null
      void client.invalidateQueries({queryKey:designKeys.all(projectId)})
    },
    onError: error => {
      const daemon = error instanceof ApiError && error.problem.kind === 'daemon' ? error.problem : null
      if (daemon?.code === 'REVISION_CONFLICT') {
        // Nothing was written; the dialog stays open on the re-read count.
        command.current = null
        setMoved(true)
        void client.invalidateQueries({ queryKey: designKeys.stage(projectId) })
        return
      }
      setConfirming(false)
      // Retry an uncertain response with the same id to recover the committed result.
      if (daemon?.status === 409) command.current = null
    },
  })
  if (result) return <section role="status" className="rounded border p-3 text-sm">
    <p>Structure written · {result.paths.length} files</p>
    <details><summary>Written paths</summary>
      <ul>{result.paths.map(path => <li key={path}><bdi dir="auto">{path}</bdi></li>)}</ul>
    </details>
  </section>
  if (stage.data?.stage === 'roadmap') return <p role="status">Structure written</p>
  if (stage.data?.stage !== 'structure') return null
  const missing = stage.data.missing
  const reason = missing.length > 0 ? `missing part: ${missing.join(', ')}` : null
  const error = moved ? null : write.error ?? projects.error
  const disabled = write.isPending || reason !== null || !project?.directory
  const revision = stage.data.map_revision
  const count = stage.data.structure_file_count
  const confirmDisabled = disabled || revision === undefined || stage.isFetching
  return <section className="space-y-2 rounded border p-3 text-sm" aria-label="Create structure">
    <Button disabled={disabled} onClick={() => { setMoved(false); setConfirming(true) }}>Create structure</Button>
    {reason && <p>{reason}</p>}
    {error && <p role="alert">{error.message}</p>}
    {confirming && <div role="dialog" aria-label="Confirm create structure"
      className="space-y-3 rounded border bg-card p-4">
      {moved && <p role="alert">The map changed; check the new count.</p>}
      <p>{typeof count === 'number' ? `Write ${count} files` : 'Write the structure'} in <bdi dir="auto">{project?.directory}</bdi>?</p>
      <div className="flex gap-2">
        <Button disabled={confirmDisabled} onClick={() => { if (revision !== undefined) { setMoved(false); write.mutate(revision) } }}>Confirm create structure</Button>
        <Button variant="secondary" disabled={write.isPending} onClick={() => setConfirming(false)}>Cancel</Button>
      </div>
    </div>}
  </section>
}
