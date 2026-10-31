// One job: reading the `data:` of each `/api/subscribe` frame into a typed
// value. The frame shapes are the daemon's (`src/protocol/sse.rs`, described in
// the OpenAPI document's `/api/subscribe` entry); OpenAPI cannot type them, so
// they are checked here, and a frame that does not match is a protocol error.

import type { AccountLimits, Choice, SessionChoices } from '@/api/sessions'
import type {
  CallKind,
  CallStatus,
  DiffHunk,
  DiffLine,
  TodoItem,
  TodoListCard,
  ToolCallCard,
} from '@/api/threads'

export type { TodoItem }

/** One journal event. `operationId` and `threadId` are the ids the event
 * names, `null` where it names none; `payload` is the event's JSON payload. */
export interface DurableEvent {
  seq: number
  kind: string
  operationId: string | null
  threadId: string | null
  payload: unknown
}

/** ProjectDesignChanged payload: identifiers only, never vision content. */
export interface ProjectDesignChanged {
  project_id: string
  revision: number
  changed_parts: string[]
  changed_outcomes: string[]
  vision_changed: boolean
}

/** Streamed text of a running turn. Transient. */
export interface Delta {
  op: string
  text: string
}

/** The harness finished a turn. Transient. */
export interface TurnEnd {
  op: string
  subtype: string
  stopReason: string | null
}

/** Accept transient operational labels from streams that supply them. */
export interface Meta {
  op: string
  label: string
}

/** One account limit window: `utilization` from 0 to 1, `resetsAt` in Unix seconds. */
export interface LimitWindow {
  utilization: number
  resetsAt: number
}

/** Unreported account limit windows stay `null` so the UI never estimates them. */
export interface Limits {
  fiveHour: LimitWindow | null
  sevenDay: LimitWindow | null
  observedAt: string
}

/** The session's context use and the account's limits, each `null` when the
 * harness did not report it. Transient. */
export interface UsageFrame {
  threadId: string
  contextUsed: number | null
  contextWindow: number | null
  limits: Limits | null
}

/** The session's choices changed. Transient. */
export interface OptionsFrame {
  threadId: string
  choices: SessionChoices
}

export type PlanPlace = 'inline' | 'side' | 'page'

/** Live-only navigation targets the sending tab so other tabs keep their current view. */
export interface PlanShowFrame {
  threadId: string
  /** The tab that sent the turn; `null` when it named none. */
  targetTab: string | null
  workflowId: string
  version: number
  taskNumber: number | null
  place: PlanPlace
}

/** Durable entries and transient frames share this card shape so either can render it. */
export interface SubagentCard {
  id: string
  title: string
  agentType: string | null
  /** The model it ran on, or else the one it asked for. */
  model: string | null
  status: 'running' | 'completed' | 'failed' | 'stopped'
  prompt: string | null
  /** The titles of its own tool calls, in the order they ended. */
  steps: readonly string[]
  report: string | null
  durationMs: number | null
  tokens: number | null
  toolCount: number | null
}

/** A running turn's subagent card after a change. Transient. */
export interface SubagentFrame {
  op: string
  card: SubagentCard
}

const STATUSES: readonly string[] = ['running', 'completed', 'failed', 'stopped']

/** The card `value` holds, or `null` when it is not one. */
export function readSubagentCard(value: unknown): SubagentCard | null {
  if (!isRecord(value)) return null
  const { id, title, status, steps } = value
  const text = (v: unknown) => (typeof v === 'string' ? v : null)
  const count = (v: unknown) => (isSeq(v) ? v : null)
  if (
    typeof id !== 'string' ||
    typeof title !== 'string' ||
    typeof status !== 'string' ||
    !STATUSES.includes(status) ||
    !Array.isArray(steps) ||
    !steps.every((s) => typeof s === 'string')
  ) {
    return null
  }
  return {
    id,
    title,
    agentType: text(value.agent_type),
    model: text(value.model),
    status: status as SubagentCard['status'],
    prompt: text(value.prompt),
    steps,
    report: text(value.report),
    durationMs: count(value.duration_ms),
    tokens: count(value.tokens),
    toolCount: count(value.tool_count),
  }
}

export function parseSubagent(data: string): SubagentFrame {
  const frame = object('subagent', data)
  const card = readSubagentCard(frame.card)
  if (typeof frame.op !== 'string' || card === null) throw new FrameError('subagent', data)
  return { op: frame.op, card }
}

/** A running turn's tool call as last sent (the ToolLive body plus op). Transient. */
export interface ToolFrame {
  op: string
  id: string
  tool: string | null
  kind: CallKind
  title: string
  target: string | null
  lines: readonly number[] | null
  status: CallStatus
  startedAt: string
}

/** A running turn's whole task list. Transient. */
export interface TodosFrame {
  op: string
  entries: readonly TodoItem[]
}

const KINDS: readonly string[] = [
  'read', 'edit', 'delete', 'move', 'search', 'execute', 'think', 'fetch', 'switch_mode', 'other',
]
const CALL_STATUSES: readonly string[] = ['pending', 'in_progress', 'completed', 'failed', 'stopped']
const TODO_STATUSES: readonly string[] = ['pending', 'in_progress', 'completed']
const DIFF_OPS: readonly string[] = ['context', 'add', 'remove']

function textOrNull(v: unknown): string | null {
  return typeof v === 'string' ? v : null
}

function readLines(v: unknown): [number, number] | null {
  if (!Array.isArray(v) || v.length !== 2) return null
  const [a, b] = v
  return isSeq(a) && isSeq(b) ? [a, b] : null
}

function readHunk(value: unknown): DiffHunk | null {
  if (!isRecord(value) || typeof value.path !== 'string' || !Array.isArray(value.lines)) return null
  const lines: DiffLine[] = []
  for (const line of value.lines) {
    if (
      !isRecord(line) ||
      typeof line.op !== 'string' ||
      !DIFF_OPS.includes(line.op) ||
      typeof line.text !== 'string'
    ) {
      return null
    }
    lines.push({ op: line.op as DiffLine['op'], text: line.text })
  }
  const place = (v: unknown) => (isSeq(v) ? v : null)
  return {
    path: value.path,
    old_start: place(value.old_start),
    new_start: place(value.new_start),
    lines,
  }
}

/** The tool call card `value` holds, or `null` when it is not one. */
export function readToolCard(value: unknown): ToolCallCard | null {
  if (!isRecord(value)) return null
  const { id, kind, status, started_at, finished_at } = value
  if (
    typeof id !== 'string' ||
    typeof kind !== 'string' ||
    !KINDS.includes(kind) ||
    typeof status !== 'string' ||
    !CALL_STATUSES.includes(status) ||
    typeof started_at !== 'string' ||
    typeof finished_at !== 'string'
  ) {
    return null
  }
  let diff: DiffHunk[] | null = null
  if (Array.isArray(value.diff)) {
    diff = []
    for (const raw of value.diff) {
      const hunk = readHunk(raw)
      if (hunk === null) return null
      diff.push(hunk)
    }
  }
  const count = (v: unknown) => (isSeq(v) ? v : null)
  return {
    id,
    tool: textOrNull(value.tool),
    kind: kind as CallKind,
    status: status as CallStatus,
    target: textOrNull(value.target),
    lines: readLines(value.lines),
    added: count(value.added),
    removed: count(value.removed),
    diff,
    error: status === 'failed' ? textOrNull(value.error) : null,
    started_at,
    finished_at,
  }
}

function readTodoItems(value: unknown): TodoItem[] | null {
  if (!Array.isArray(value)) return null
  const items: TodoItem[] = []
  for (const item of value) {
    if (
      !isRecord(item) ||
      typeof item.content !== 'string' ||
      typeof item.status !== 'string' ||
      !TODO_STATUSES.includes(item.status)
    ) {
      return null
    }
    items.push({ content: item.content, status: item.status as TodoItem['status'] })
  }
  return items
}

/** The task list card `value` holds, or `null` when it is not one. */
export function readTodoList(value: unknown): TodoListCard | null {
  if (!isRecord(value)) return null
  const entries = readTodoItems(value.entries)
  return entries === null ? null : { entries }
}

export function parseTool(data: string): ToolFrame {
  const frame = object('tool', data)
  const { op, id, kind, title, status, started_at } = frame
  if (
    typeof op !== 'string' ||
    typeof id !== 'string' ||
    typeof kind !== 'string' ||
    !KINDS.includes(kind) ||
    typeof title !== 'string' ||
    typeof status !== 'string' ||
    !CALL_STATUSES.includes(status) ||
    typeof started_at !== 'string'
  ) {
    throw new FrameError('tool', data)
  }
  return {
    op,
    id,
    tool: textOrNull(frame.tool),
    kind: kind as CallKind,
    title,
    target: textOrNull(frame.target),
    lines: readLines(frame.lines),
    status: status as CallStatus,
    startedAt: started_at,
  }
}

export function parseTodos(data: string): TodosFrame {
  const frame = object('todos', data)
  const entries = readTodoItems(frame.entries)
  if (typeof frame.op !== 'string' || entries === null) throw new FrameError('todos', data)
  return { op: frame.op, entries }
}

export class FrameError extends Error {
  constructor(event: string, data: string) {
    super(`malformed \`${event}\` frame: ${data.slice(0, 200)}`)
    this.name = 'FrameError'
  }
}

export function parseDurable(data: string): DurableEvent {
  const frame = object('durable', data)
  const { seq, kind, payload } = frame
  const operationId = frame.operation_id
  const threadId = frame.thread_id
  if (
    !isSeq(seq) ||
    typeof kind !== 'string' ||
    !('payload' in frame) ||
    !isIdOrNull(operationId) ||
    !isIdOrNull(threadId)
  ) {
    throw new FrameError('durable', data)
  }
  return { seq, kind, operationId, threadId, payload }
}

/** `caught-up` carries the last replayed seq. */
export function parseCaughtUp(data: string): number {
  const { seq } = object('caught-up', data)
  if (!isSeq(seq)) {
    throw new FrameError('caught-up', data)
  }
  return seq
}

export function parseDelta(data: string): Delta {
  const frame = object('delta', data)
  const { op, text } = frame
  if (typeof op !== 'string' || typeof text !== 'string') {
    throw new FrameError('delta', data)
  }
  return { op, text }
}

export function parseTurnEnd(data: string): TurnEnd {
  const frame = object('turn-end', data)
  const { op, subtype } = frame
  const stopReason = frame.stop_reason ?? null
  if (
    typeof op !== 'string' ||
    typeof subtype !== 'string' ||
    (stopReason !== null && typeof stopReason !== 'string')
  ) {
    throw new FrameError('turn-end', data)
  }
  return { op, subtype, stopReason }
}

export function parseMeta(data: string): Meta {
  const frame = object('meta', data)
  const { op, label } = frame
  if (typeof op !== 'string' || typeof label !== 'string') {
    throw new FrameError('meta', data)
  }
  return { op, label }
}

export function parseUsage(data: string): UsageFrame {
  const frame = object('usage', data)
  const threadId = frame.thread_id
  const contextUsed = frame.context_used
  const contextWindow = frame.context_window
  const limits = frame.limits
  if (
    typeof threadId !== 'string' ||
    !isCountOrNull(contextUsed) ||
    !isCountOrNull(contextWindow) ||
    !(limits === null || isAccountLimits(limits))
  ) {
    throw new FrameError('usage', data)
  }
  return { threadId, contextUsed, contextWindow, limits: limits === null ? null : toLimits(limits) }
}

export function parseOptions(data: string): OptionsFrame {
  const frame = object('options', data)
  const threadId = frame.thread_id
  const choices = frame.choices
  if (typeof threadId !== 'string' || !isSessionChoices(choices)) {
    throw new FrameError('options', data)
  }
  return { threadId, choices }
}

export type SlashCommand = { name: string; description: string; hint: string | null }
export type CommandsFrame = { threadId: string; commands: readonly SlashCommand[] }

function isSlashCommand(value: unknown): value is SlashCommand {
  if (typeof value !== 'object' || value === null) return false
  const c = value as Record<string, unknown>
  return (
    typeof c.name === 'string' &&
    typeof c.description === 'string' &&
    (c.hint === null || typeof c.hint === 'string')
  )
}

export function parseCommands(data: string): CommandsFrame {
  const frame = object('commands', data)
  const threadId = frame.thread_id
  const commands = frame.commands
  if (
    typeof threadId !== 'string' ||
    !Array.isArray(commands) ||
    !commands.every(isSlashCommand)
  ) {
    throw new FrameError('commands', data)
  }
  return { threadId, commands }
}

export function parsePlanShow(data: string): PlanShowFrame {
  const frame = object('plan-show', data)
  const threadId = frame.thread_id
  const targetTab = frame.target_tab ?? null
  const workflowId = frame.workflow_id
  const { version, place } = frame
  const taskNumber = frame.task_number ?? null
  if (
    typeof threadId !== 'string' ||
    !isStringOrNull(targetTab) ||
    typeof workflowId !== 'string' ||
    !isSeq(version) ||
    !(taskNumber === null || isSeq(taskNumber)) ||
    !(place === 'inline' || place === 'side' || place === 'page')
  ) {
    throw new FrameError('plan-show', data)
  }
  return { threadId, targetTab, workflowId, version, taskNumber, place }
}

/** The daemon's limits (as `GET /api/harnesses` also carries them) in this
 * client's terms. */
export function toLimits(limits: AccountLimits): Limits {
  const window = (w: AccountLimits['five_hour']): LimitWindow | null =>
    w === null ? null : { utilization: w.utilization, resetsAt: w.resets_at }
  return {
    fiveHour: window(limits.five_hour),
    sevenDay: window(limits.seven_day),
    observedAt: limits.observed_at,
  }
}

function isCountOrNull(value: unknown): value is number | null {
  return value === null || (typeof value === 'number' && Number.isFinite(value) && value >= 0)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isWindowOrNull(value: unknown): boolean {
  return (
    value === null ||
    (isRecord(value) && typeof value.utilization === 'number' && typeof value.resets_at === 'number')
  )
}

function isAccountLimits(value: unknown): value is AccountLimits {
  return (
    isRecord(value) &&
    isWindowOrNull(value.five_hour) &&
    isWindowOrNull(value.seven_day) &&
    typeof value.observed_at === 'string'
  )
}

function isStringOrNull(value: unknown): value is string | null {
  return value === null || typeof value === 'string'
}

function isChoice(value: unknown): value is Choice {
  return (
    isRecord(value) &&
    typeof value.id === 'string' &&
    typeof value.label === 'string' &&
    isStringOrNull(value.description) &&
    typeof value.enabled === 'boolean' &&
    isStringOrNull(value.reason)
  )
}

function isSessionChoices(value: unknown): value is SessionChoices {
  if (!isRecord(value)) return false
  const { models, efforts, modes, current } = value
  const list = (v: unknown) => Array.isArray(v) && v.every(isChoice)
  return (
    list(models) &&
    list(efforts) &&
    list(modes) &&
    isRecord(current) &&
    typeof current.model === 'string' &&
    typeof current.mode === 'string' &&
    isStringOrNull(current.effort)
  )
}

function isSeq(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
}

/** Present, and either an id or `null`. A missing field is a daemon that
 * does not send it, which this client cannot follow. */
function isIdOrNull(value: unknown): value is string | null {
  return value === null || typeof value === 'string'
}

function json(event: string, data: string): unknown {
  try {
    return JSON.parse(data)
  } catch {
    throw new FrameError(event, data)
  }
}

function object(event: string, data: string): Record<string, unknown> {
  const value = json(event, data)
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new FrameError(event, data)
  }
  return Object.fromEntries(Object.entries(value))
}
