// One job: an edit's hunks drawn as lines with their old and new numbers.

import { Fragment } from 'react'
import type { DiffHunk } from '@/api/threads'

const MARKS = { add: '+', remove: '−', context: ' ' } as const
const TINTS = {
  add: 'bg-success/15 text-foreground',
  remove: 'bg-destructive/15 text-foreground',
  context: 'text-foreground/85',
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
    <div
      dir="ltr"
      className="ms-6 mt-1.5 overflow-x-auto rounded-xl border border-accent-line/25 bg-background py-1 font-mono text-xs"
    >
      {diff.map((hunk, index) => {
        const before = diff[index - 1]
        return (
          <Fragment key={index}>
            {before !== undefined && before.path === hunk.path && (
              <div
                aria-label="Gap"
                className="border-y border-border/40 bg-accent-softer/40 px-3 py-0.5 text-center text-muted-foreground"
              >
                ⋯
              </div>
            )}
            {before !== undefined && before.path !== hunk.path && (
              <div className="border-y border-border/70 bg-accent-softer px-3 py-1 font-semibold text-accent-line">
                {hunk.path}
              </div>
            )}
            {numbered(hunk).map(({ line, old, next }, at) => (
              <div
                key={at}
                data-diff={line.op}
                className={`flex whitespace-pre px-1.5 py-0.5 ${TINTS[line.op]}`}
              >
                {(hunk.old_start != null || hunk.new_start != null) && (
                  <>
                    <span className="w-9 shrink-0 pe-1.5 text-end text-faint-foreground tabular-nums select-none">
                      {old}
                    </span>
                    <span className="w-9 shrink-0 pe-1.5 text-end text-faint-foreground tabular-nums select-none">
                      {next}
                    </span>
                  </>
                )}
                <span aria-hidden className="w-5 shrink-0 text-center font-bold select-none">
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
