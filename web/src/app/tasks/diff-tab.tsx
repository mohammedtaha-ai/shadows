import { SkeletonContent } from '@/components/ui/skeleton'
import { useQuery } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { ChevronDown, FileCode2 } from 'lucide-react'
import { type Attempt, type DiffHunk, attemptDiffQuery, attemptFileQuery } from '@/api/runs'
import { ErrorLine } from '../error-line'
import { FileView } from './file-view'

/** The attempt's commit, with one changed file expanded at a time. */
export function DiffTab({ attempt }: { attempt: Attempt | null }) {
  if (attempt === null || attempt.state !== 'committed') {
    return <p className="px-6 py-4 text-sm text-faint-foreground">Nothing committed yet.</p>
  }
  return <Files key={attempt.id} attemptId={attempt.id} />
}

function OpenFile({ attemptId, path, hunks }: { attemptId: string; path: string; hunks: readonly DiffHunk[] }) {
  const file = useQuery(attemptFileQuery(attemptId, path))
  if (file.isError) return <div className="px-3 py-2"><ErrorLine error={file.error} /></div>
  if (file.data === undefined) return <SkeletonContent label="Reading the file…" className="px-3 py-2" />
  return <FileView file={file.data} hunks={hunks} />
}

function Files({ attemptId }: { attemptId: string }) {
  const diff = useQuery(attemptDiffQuery(attemptId))
  // Undefined chooses the first file; null is the person's explicit collapse.
  const [chosen, setChosen] = useState<string | null>()
  const id = useId()
  if (diff.isError) return <div className="px-6 py-4"><ErrorLine error={diff.error} /></div>
  if (diff.data === undefined) return <SkeletonContent label="Reading the diff…" layout="detail" className="px-6 py-4" />
  const files = diff.data
  const open = chosen === null ? null : files.find((file) => file.path === chosen) ?? files[0] ?? null
  if (files.length === 0) return <p className="px-6 py-4 text-sm text-faint-foreground">No files changed.</p>
  return (
    <div className="space-y-3 px-6 py-4 text-xs">
      <p className="text-muted-foreground">{files.length} changed {files.length === 1 ? 'file' : 'files'} · Select a file to expand or collapse its diff.</p>
      <ul aria-label="Changed files" className="space-y-3">
        {files.map((file, index) => {
          const expanded = open?.path === file.path
          const buttonId = `${id}-file-${index}`
          const regionId = `${buttonId}-content`
          return <li key={file.path} className={`overflow-hidden rounded-xl border ${expanded ? 'border-accent-line/50' : 'border-border'}`}>
            <button id={buttonId} type="button" aria-expanded={expanded} aria-controls={regionId}
              aria-label={`${expanded ? 'Hide' : 'Show'} diff for ${file.path}`}
              onClick={() => { setChosen(expanded ? null : file.path) }}
              className="flex w-full cursor-pointer flex-wrap items-center gap-x-3 gap-y-2 bg-muted/25 px-4 py-3 text-start transition-colors hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring aria-expanded:bg-accent-line/10">
              <FileCode2 aria-hidden className="size-4 shrink-0 text-accent-line" />
              <span className="min-w-0 flex-1 break-all font-mono font-medium text-foreground">{file.path}</span>
              <span className="font-mono text-success">+{file.added}</span>
              <span className="font-mono text-destructive-foreground">-{file.removed}</span>
              <span className="inline-flex items-center gap-1.5 text-muted-foreground">
                {expanded ? 'Hide diff' : 'Show diff'}
                <ChevronDown aria-hidden className={`size-4 ${expanded ? 'rotate-180' : ''}`} />
              </span>
            </button>
            <div id={regionId} role="region" aria-labelledby={buttonId} hidden={!expanded}>
              {expanded && <div className="max-h-[60vh] overflow-auto border-t border-border">
                <OpenFile attemptId={attemptId} path={file.path} hunks={file.hunks} />
              </div>}
            </div>
          </li>
        })}
      </ul>
    </div>
  )
}
