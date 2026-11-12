import type { AttemptFile, DiffHunk } from '@/api/runs'

type Mark = 'add' | 'remove' | null
type Row = { mark: Mark; no: number | null; text: string }

/** Joins Core's two answers for drawing: the file's text and the hunks that changed it. It decides nothing about the change. */
function fileRows(text: string, hunks: readonly DiffHunk[]): Row[] {
  const lines = text.split('\n')
  if (lines[lines.length - 1] === '') lines.pop()
  const added = new Set<number>()
  const removedBefore = new Map<number, string[]>()
  for (const hunk of hunks) {
    if (hunk.new_start === null || hunk.new_start === undefined) continue
    let n = hunk.new_start
    for (const line of hunk.lines) {
      if (line.op === 'remove') {
        removedBefore.set(n, [...(removedBefore.get(n) ?? []), line.text])
        continue
      }
      if (line.op === 'add') added.add(n)
      n += 1
    }
  }
  const rows: Row[] = []
  for (let n = 1; n <= lines.length + 1; n += 1) {
    for (const gone of removedBefore.get(n) ?? []) rows.push({ mark: 'remove', no: null, text: gone })
    if (n <= lines.length) rows.push({ mark: added.has(n) ? 'add' : null, no: n, text: lines[n - 1] ?? '' })
  }
  return rows
}

/** Each hunk alone, for a file with no text: its lines with their mark and number. */
function hunkRows(hunks: readonly DiffHunk[]): (Row | 'separator')[] {
  const rows: (Row | 'separator')[] = []
  for (const hunk of hunks) {
    rows.push('separator')
    let oldNo = hunk.old_start ?? 0
    let newNo = hunk.new_start ?? 0
    for (const line of hunk.lines) {
      if (line.op === 'remove') {
        rows.push({ mark: 'remove', no: oldNo, text: line.text })
        oldNo += 1
      } else {
        rows.push({ mark: line.op === 'add' ? 'add' : null, no: newNo, text: line.text })
        if (line.op === 'context') oldNo += 1
        newNo += 1
      }
    }
  }
  return rows
}

const TINT: Record<Exclude<Mark, null>, string> = { add: 'bg-success/15', remove: 'bg-destructive/15' }
const SIGN: Record<Exclude<Mark, null>, string> = { add: '+', remove: '-' }

function LineRow({ row }: { row: Row }) {
  return (
    <div data-diff={row.mark ?? undefined} className={`flex min-w-max ${row.mark === null ? '' : TINT[row.mark]}`}>
      <span className="w-10 shrink-0 select-none px-1 text-end text-faint-foreground">{row.no}</span>
      <span className="w-4 shrink-0 select-none text-center text-faint-foreground">{row.mark === null ? '' : SIGN[row.mark]}</span>
      <span className="whitespace-pre px-2">{row.text}</span>
    </div>
  )
}

/** One file at the attempt's commit, its changed lines marked from Core's hunks. */
export function FileView({ file, hunks }: { file: AttemptFile; hunks: readonly DiffHunk[] }) {
  return (
    <div dir="ltr" className="overflow-x-auto font-mono text-2xs">
      {file.text === null || file.text === undefined ? (
        <>
          <p className="px-3 py-1.5 text-muted-foreground">No text to show</p>
          {hunkRows(hunks).map((row, index) =>
            row === 'separator'
              ? <div key={index} className="min-w-max bg-muted/40 px-3 text-muted-foreground">…</div>
              : <LineRow key={index} row={row} />)}
        </>
      ) : (
        fileRows(file.text, hunks).map((row, index) => <LineRow key={index} row={row} />)
      )}
      {file.cut && <p className="px-3 py-1.5 text-muted-foreground">Core shortened this file; the rest is not shown.</p>}
    </div>
  )
}
