// One job: an edit's hunks drawn as lines with their old and new numbers.

import { Fragment } from 'react'
import type { DiffHunk } from '@/api/threads'

const MARKS = { add: '+', remove: '−', context: ' ' } as const
const TINTS = {
  add: 'bg-green-500/15',
  remove: 'bg-red-500/15',
  context: '',
} as const

/** The numbers a hunk's lines carry: `null` where the hunk has no start. */
function numbered(hunk: DiffHunk) {
  let oldAt = hunk.old_start ?? null
  let newAt = hunk.new_start ?? null
  return hunk.lines.map((line) => {
    const old = line.op === 'add' ? null : oldAt
    const next = line.op === 'remove' ? null : newAt
    if (oldAt !== null && line.op !== 'add') oldAt += 1
    if (newAt !== null && line.op !== 'remove') newAt += 1
    return { line, old, next }
  })
}

/** `diff` null with a count set is a diff Core left out for its size. */
export function ToolDiff({ diff }: { diff: readonly DiffHunk[] | null }) {
  if (diff === null || diff.length === 0) {
    return <p className="ps-6 text-xs text-muted-foreground">Diff too long to show</p>
  }
  return (
    <div dir="ltr" className="ms-6 overflow-x-auto rounded border border-border font-mono text-xs">
      {diff.map((hunk, index) => {
        const before = diff[index - 1]
        return (
          <Fragment key={index}>
            {before !== undefined && before.path === hunk.path && (
              <div aria-label="Gap" className="px-2 text-muted-foreground">
                ⋯
              </div>
            )}
            {before !== undefined && before.path !== hunk.path && (
              <div className="border-t border-border px-2 py-0.5 text-muted-foreground">
                {hunk.path}
              </div>
            )}
            {numbered(hunk).map(({ line, old, next }, at) => (
              <div
                key={at}
                data-diff={line.op}
                className={`flex whitespace-pre ${TINTS[line.op]}`}
              >
                {(hunk.old_start != null || hunk.new_start != null) && (
                  <>
                    <span className="w-9 shrink-0 pe-1 text-end text-muted-foreground tabular-nums select-none">
                      {old}
                    </span>
                    <span className="w-9 shrink-0 pe-1 text-end text-muted-foreground tabular-nums select-none">
                      {next}
                    </span>
                  </>
                )}
                <span aria-hidden className="w-4 shrink-0 text-center select-none">
                  {MARKS[line.op]}
                </span>
                <span>{line.text}</span>
              </div>
            ))}
          </Fragment>
        )
      })}
    </div>
  )
}
