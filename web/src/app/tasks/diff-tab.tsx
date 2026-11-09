import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { type Attempt, attemptDiffQuery } from '@/api/runs'
import { ErrorLine } from '../error-line'

/** The attempt's commit: its files with counts; a file opens its patch. */
export function DiffTab({ attempt }: { attempt: Attempt | null }) {
  if (attempt === null || attempt.state !== 'committed') {
    return <p className="px-6 py-4 text-sm text-faint-foreground">Nothing committed yet.</p>
  }
  return <Files attemptId={attempt.id} />
}

function Files({ attemptId }: { attemptId: string }) {
  const diff = useQuery(attemptDiffQuery(attemptId))
  const [open, setOpen] = useState<string | null>(null)
  if (diff.isError) return <div className="px-6 py-4"><ErrorLine error={diff.error} /></div>
  if (diff.data === undefined) return <p className="px-6 py-4 text-sm text-faint-foreground">Reading the diff…</p>
  return (
    <ul className="px-6 py-4 text-xs">
      {diff.data.map((file) => (
        <li key={file.path} className="mb-2">
          <button type="button" aria-expanded={open === file.path}
            onClick={() => { setOpen(open === file.path ? null : file.path) }}
            className="flex w-full items-center gap-3 rounded border border-border px-2 py-1 text-start font-mono">
            <span className="min-w-0 flex-1 truncate">{file.path}</span>
            <span>+{file.added}</span>
            <span>-{file.removed}</span>
          </button>
          {open === file.path && (
            <pre className="mt-1 overflow-x-auto rounded bg-muted/40 p-2 font-mono text-2xs">{file.patch}</pre>
          )}
        </li>
      ))}
    </ul>
  )
}
