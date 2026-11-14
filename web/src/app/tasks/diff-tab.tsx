import { SkeletonContent } from '@/components/ui/skeleton'
import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { type Attempt, type DiffHunk, attemptDiffQuery, attemptFileQuery } from '@/api/runs'
import { ErrorLine } from '../error-line'
import { FileView } from './file-view'

/** The attempt's commit: the open file above, the changed files in a box below. */
export function DiffTab({ attempt }: { attempt: Attempt | null }) {
  if (attempt === null || attempt.state !== 'committed') {
    return <p className="px-6 py-4 text-sm text-faint-foreground">Nothing committed yet.</p>
  }
  return <Files attemptId={attempt.id} />
}

function OpenFile({ attemptId, path, hunks }: { attemptId: string; path: string; hunks: readonly DiffHunk[] }) {
  const file = useQuery(attemptFileQuery(attemptId, path))
  if (file.isError) return <div className="px-3 py-2"><ErrorLine error={file.error} /></div>
  if (file.data === undefined) return <SkeletonContent label="Reading the file…" className="px-3 py-2" />
  return <FileView file={file.data} hunks={hunks} />
}

function Files({ attemptId }: { attemptId: string }) {
  const diff = useQuery(attemptDiffQuery(attemptId))
  const [chosen, setChosen] = useState<string | null>(null)
  if (diff.isError) return <div className="px-6 py-4"><ErrorLine error={diff.error} /></div>
  if (diff.data === undefined) return <SkeletonContent label="Reading the diff…" layout="detail" className="px-6 py-4" />
  const files = diff.data
  const open = files.find((file) => file.path === chosen) ?? files[0] ?? null
  return (
    <div className="flex h-full min-h-0 flex-col gap-3 px-6 py-4 text-xs">
      <div className="flex min-h-0 flex-1 flex-col rounded border border-border">
        {open === null ? (
          <p className="px-3 py-2 text-faint-foreground">No files changed.</p>
        ) : (
          <>
            <div className="flex items-center gap-3 border-b border-border px-3 py-1.5 font-mono">
              <span className="min-w-0 flex-1 truncate">{open.path}</span>
              <span>+{open.added}</span>
              <span>-{open.removed}</span>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto"><OpenFile attemptId={attemptId} path={open.path} hunks={open.hunks} /></div>
          </>
        )}
      </div>
      <ul aria-label="Changed files" className="max-h-48 shrink-0 overflow-y-auto rounded border border-border">
        {files.map((file) => (
          <li key={file.path}>
            <button type="button" aria-pressed={open?.path === file.path}
              onClick={() => { setChosen(file.path) }}
              className="flex w-full items-center gap-3 px-3 py-1 text-start font-mono hover:bg-muted/40 aria-pressed:bg-muted">
              <span className="min-w-0 flex-1 truncate">{file.path}</span>
              <span>+{file.added}</span>
              <span>-{file.removed}</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
