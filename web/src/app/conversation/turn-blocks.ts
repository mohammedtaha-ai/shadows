// One job: grouping a Planner turn's activity rows into one box, leaving the
// person's prompt above it and the answer and the cards they act on under it.

import type { AttemptNotice } from '@/api/runs'
import type { ThreadEntry } from '@/api/threads'
import type { Row } from './activity-rows'
import { noticeOf } from './tool-text'

export interface TurnSummary {
  /** One card per notice the turn received; null where a card could not be read. */
  notices: readonly (AttemptNotice | null)[]
  tools: number
  /** Text written between the box's items. */
  notes: number
}

export type Block =
  | { kind: 'row'; row: Row }
  | {
      kind: 'turn'
      key: string
      inside: Row[]
      /** Every ordinal inside, so a link to one opens the box. */
      ordinals: number[]
      summary: TurnSummary
      live: boolean
    }

type Role = 'prompt' | 'item' | 'text' | 'line' | 'outside'

function roleOf(row: Row): Role {
  if (row.kind === 'tools') return 'item'
  if (row.kind === 'text') return 'text'
  const { entry } = row
  if (entry.kind === 'UserMessage') return noticeOf(entry) === null ? 'prompt' : 'item'
  if (entry.kind === 'AgentMessage') return 'text'
  if (entry.kind === 'PermissionAnswered' || entry.kind === 'PermissionRefused') return 'line'
  // A card the person reads or acts on (a question, a proposal, a plan, a task list) and any
  // other entry stay visible: only what is known to be activity folds.
  return 'outside'
}

/** The turn a row belongs to, or null when it names none. A durable tool step's key is its entry's id. */
function opOf(row: Row, runningOp: string | null, opByEntry: ReadonlyMap<string, string | null>): string | null {
  if (row.kind === 'entry') return row.entry.operation_id
  if (row.kind === 'text') return runningOp
  const first = row.steps[0]
  if (first === undefined) return null
  return first.ordinal === null ? runningOp : (opByEntry.get(first.key) ?? null)
}

const ordinalsOf = (row: Row): number[] =>
  row.kind === 'entry' ? [row.entry.ordinal]
    : row.kind === 'tools' ? row.steps.flatMap((step) => (step.ordinal === null ? [] : [step.ordinal]))
      : []

function rowKey(row: Row): string {
  return row.kind === 'entry' ? row.entry.id : row.key
}

/** One turn's rows, in order, as the blocks it draws. The box holds everything up to the turn's
 * last notice or tool call; the text after it is the final answer and stays outside. A message the
 * person steers in mid-turn stays where it was sent: the activity before it is a box of its own,
 * already ended, and what follows is drawn as the turn's rows from that message on. */
function turnOf(op: string, rows: readonly Row[], live: boolean): Block[] {
  const roles = rows.map(roleOf)
  const steered = roles.findIndex((role, index) => role === 'prompt' && roles.slice(0, index).some((r) => r !== 'prompt'))
  if (steered > 0) return [...turnOf(op, rows.slice(0, steered), false), ...turnOf(op, rows.slice(steered), live)]
  const lastItem = roles.lastIndexOf('item')
  if (lastItem < 0) return rows.map((row) => ({ kind: 'row', row }))
  const before: Block[] = []
  const inside: Row[] = []
  const after: Block[] = []
  const notices: (AttemptNotice | null)[] = []
  let tools = 0
  let notes = 0
  rows.forEach((row, index) => {
    const role = roles[index]
    if (role === 'prompt') {
      before.push({ kind: 'row', row })
    } else if (index <= lastItem && (role === 'item' || role === 'text' || role === 'line')) {
      inside.push(row)
      if (role === 'text') notes += 1
      if (row.kind === 'tools') tools += row.steps.length
      if (row.kind === 'entry' && role === 'item') notices.push(noticeOf(row.entry)?.card ?? null)
    } else {
      after.push({ kind: 'row', row })
    }
  })
  const first = inside[0] ?? rows[0]
  const box: Block = {
    kind: 'turn',
    key: `turn-${first === undefined ? op : rowKey(first)}`,
    inside,
    ordinals: inside.flatMap(ordinalsOf),
    summary: { notices, tools, notes },
    live,
  }
  return [...before, box, ...after]
}

/** Groups each turn's rows into one box. A row that names no turn, and a turn
 * with no notice or tool call, stay as they are. `entries` tells which turn a
 * durable tool step belongs to; `runningOp` is the turn still running, which
 * owns the streamed text and the live steps. */
export function turnBlocks(
  rows: readonly Row[],
  runningOp: string | null,
  entries: readonly ThreadEntry[],
): Block[] {
  const opByEntry = new Map(entries.map((entry) => [entry.id, entry.operation_id]))
  const blocks: Block[] = []
  let run: Row[] = []
  let runOp: string | null = null
  const flush = () => {
    if (runOp === null) blocks.push(...run.map((row): Block => ({ kind: 'row', row })))
    else blocks.push(...turnOf(runOp, run, runOp === runningOp))
    run = []
  }
  for (const row of rows) {
    const op = opOf(row, runningOp, opByEntry)
    if (op !== runOp) {
      flush()
      runOp = op
    }
    run.push(row)
  }
  flush()
  return blocks
}
