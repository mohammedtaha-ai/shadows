// The person's structure confirmation, write and retained result.
import { useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createStructure, getParts, type StructureWritten } from '@/api/design'
import { ApiError } from '@/api/error'
import { stageQuery } from '@/api/queries'
import { projectsQuery } from '@/api/projects'
import { designKeys } from '@/api/design'
import { Button } from '@/components/ui/button'

async function fileCount(projectId: string): Promise<number> {
  const parents: (string | undefined)[] = [undefined]
  const seen = new Set<string>()
  let count = 1 // root SHADOWS.md
  let revision: number | undefined
  for (const parent of parents) {
    let after: string | undefined
    do {
      const page = await getParts(projectId, parent, after)
      if (revision !== undefined && revision !== page.revision) {
        throw new Error('Map changed; try again.')
      }
      revision = page.revision
      for (const part of page.items) {
        if (seen.has(part.id)) throw new Error('Map changed; try again.')
        seen.add(part.id)
        count += 1 + (part.content.files?.length ?? 0)
        parents.push(part.id)
      }
      after = page.next ?? undefined
    } while (after !== undefined)
  }
  return count
}

export function CreateStructure({ projectId }: {projectId:string}) {
  const client = useQueryClient()
  const stage = useQuery(stageQuery(projectId))
  const projects = useQuery(projectsQuery)
  const project = projects.data?.find(p => p.id === projectId)
  const [confirming, setConfirming] = useState(false)
  const [result, setResult] = useState<StructureWritten | null>(null)
  const command = useRef<string | null>(null)
  const preview = useQuery({
    queryKey: designKeys.preview(projectId),
    queryFn: () => fileCount(projectId),
    enabled: confirming && stage.data?.stage === 'structure' && stage.data.missing.length === 0,
  })
  const write = useMutation({
    mutationFn: () => createStructure(projectId, command.current ??= crypto.randomUUID()),
    onSuccess: written => {
      setResult(written)
      setConfirming(false)
      command.current = null
      void client.invalidateQueries({queryKey:designKeys.all(projectId)})
    },
    onError: error => {
      setConfirming(false)
      // Retry an uncertain response with the same id to recover the committed result.
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.status === 409) {
        command.current = null
      }
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
  const error = write.error ?? preview.error ?? projects.error
  const disabled = write.isPending || reason !== null || !project?.directory
  const confirmDisabled = disabled || preview.data === undefined || preview.isFetching
  return <section className="space-y-2 rounded border p-3 text-sm" aria-label="Create structure">
    <Button disabled={disabled} onClick={() => setConfirming(true)}>Create structure</Button>
    {reason && <p>{reason}</p>}
    {error && <p role="alert">{error.message}</p>}
    {preview.error && <Button variant="secondary" onClick={() => void preview.refetch()}>Retry file count</Button>}
    {confirming && <div role="dialog" aria-label="Confirm create structure"
      className="space-y-3 rounded border bg-card p-4">
      <p>{preview.data === undefined ? 'Reading file count' : `Write ${preview.data} files`} in <bdi dir="auto">{project?.directory}</bdi>?</p>
      <div className="flex gap-2">
        <Button disabled={confirmDisabled} onClick={() => write.mutate()}>Confirm create structure</Button>
        <Button variant="secondary" disabled={write.isPending} onClick={() => setConfirming(false)}>Cancel</Button>
      </div>
    </div>}
  </section>
}
