type Row = { kind: 'hunk' | 'added' | 'removed' | 'context' | 'plain'; text: string; oldNo: number | null; newNo: number | null }

const HUNK = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/

/** Reads git's unified diff text line by line; the numbers come from each hunk's `@@` header. */
function rowsOf(patch: string): Row[] {
  let oldNo = 0
  let newNo = 0
  let inHunk = false
  return patch.split('\n').map((line): Row => {
    const hunk = HUNK.exec(line)
    if (hunk) {
      oldNo = Number(hunk[1])
      newNo = Number(hunk[2])
      inHunk = true
      return { kind: 'hunk', text: line, oldNo: null, newNo: null }
    }
    if (inHunk && line.startsWith('+')) return { kind: 'added', text: line, oldNo: null, newNo: newNo++ }
    if (inHunk && line.startsWith('-')) return { kind: 'removed', text: line, oldNo: oldNo++, newNo: null }
    if (inHunk && line.startsWith(' ')) return { kind: 'context', text: line, oldNo: oldNo++, newNo: newNo++ }
    return { kind: 'plain', text: line, oldNo: null, newNo: null }
  })
}

const TINT: Record<Row['kind'], string> = {
  hunk: 'bg-muted/40 text-muted-foreground',
  added: 'bg-success/15',
  removed: 'bg-destructive/15',
  context: '',
  plain: 'text-muted-foreground',
}

/** One file's patch drawn with line numbers. It draws git's text and decides nothing about it. */
export function FilePatch({ patch }: { patch: string }) {
  return (
    <div dir="ltr" className="overflow-x-auto font-mono text-2xs">
      {rowsOf(patch).map((row, index) => (
        <div key={index} className={`flex min-w-max ${TINT[row.kind]}`}>
          <span className="w-10 shrink-0 select-none px-1 text-end text-faint-foreground">{row.oldNo}</span>
          <span className="w-10 shrink-0 select-none px-1 text-end text-faint-foreground">{row.newNo}</span>
          <span className="whitespace-pre px-2">{row.text}</span>
        </div>
      ))}
    </div>
  )
}
