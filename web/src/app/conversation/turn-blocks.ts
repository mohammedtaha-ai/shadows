// One job: grouping a Planner turn's notices and tool calls into boxes, leaving
// the person's messages, the Planner's text and the cards they act on in place.

import type { ThreadEntry } from '@/api/threads'
import type { Row } from './activity-rows'
import { noticeOf } from './tool-text'

/** A notice in the box: who sent it, and its card (null where it has none or it could not be read). */
export type BoxNotice = NonNullable<ReturnType<typeof noticeOf>>

export interface TurnSummary {
  /** One per notice the turn received, in order. */
  notices: readonly BoxNotice[]
  tools: number
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

/** One run of activity as its box, or as plain rows when it holds no notice or tool call. */
function boxOf(inside: Row[], roles: readonly Role[], live: boolean): Block[] {
  if (!roles.includes('item')) return inside.map((row) => ({ kind: 'row', row }))
  const notices: BoxNotice[] = []
  let tools = 0
  inside.forEach((row, index) => {
    if (row.kind === 'tools') tools += row.steps.length
    const notice = row.kind === 'entry' && roles[index] === 'item' ? noticeOf(row.entry) : null
    if (notice !== null) notices.push(notice)
  })
  const first = inside[0]
  return [{
    kind: 'turn',
    key: `turn-${first === undefined ? '' : rowKey(first)}`,
    inside,
    ordinals: inside.flatMap(ordinalsOf),
    summary: { notices, tools },
    live,
  }]
}

/** One turn's rows, in order, as the blocks it draws. A box holds only a run of notices and tool
 * calls (with the permission lines among them); the Planner's own text stays outside, in its
 * place, and ends the run before it. A card the person acts on stays outside too, drawn under the
 * box of the run it came in, so the run stays one box. While the turn runs,
 * only its last box is live. A message the person steers in mid-turn stays where it was sent: the
 * activity before it has ended, and what follows is drawn as the turn's rows from that message on. */
function turnOf(rows: readonly Row[], live: boolean): Block[] {
  const roles = rows.map(roleOf)
  const steered = roles.findIndex((role, index) => role === 'prompt' && roles.slice(0, index).some((r) => r !== 'prompt'))
  if (steered > 0) return [...turnOf(rows.slice(0, steered), false), ...turnOf(rows.slice(steered), live)]
  const blocks: Block[] = []
  let run: Row[] = []
  let runRoles: Role[] = []
  let held: Block[] = []
  const close = (last: boolean) => {
    blocks.push(...boxOf(run, runRoles, live && last), ...held)
    run = []
    runRoles = []
    held = []
  }
  rows.forEach((row, index) => {
    const role = roles[index] ?? 'outside'
    if (role === 'item' || role === 'line') {
      run.push(row)
      runRoles.push(role)
    } else if (role === 'outside' && run.length > 0) {
      held.push({ kind: 'row', row })
    } else {
      close(!roles.slice(index).includes('item'))
      blocks.push({ kind: 'row', row })
    }
  })
  close(true)
  return blocks
}

/** Groups each turn's runs of notices and tool calls into boxes. A row that names
 * no turn, and a turn with no notice or tool call, stay as they are. `entries` tells which turn a
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
    else blocks.push(...turnOf(run, runOp === runningOp))
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
