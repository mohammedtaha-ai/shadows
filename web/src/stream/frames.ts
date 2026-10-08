// One job: reading the `data:` of each `/api/subscribe` frame into a typed
// value. The frame shapes are the daemon's (`src/protocol/sse.rs`, described in
// the OpenAPI document's `/api/subscribe` entry); OpenAPI cannot type them, so
// they are checked here, and a frame that does not match is a protocol error.

import type { AccountLimits, Choice, SessionChoices } from '@/api/client'

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

/** Any other harness line, by label. Transient. The daemon no longer sends
 * it since the harness runs over ACP; a stream that does is still read. */
export interface Meta {
  op: string
  label: string
}

/** One account limit window: `utilization` from 0 to 1, `resetsAt` in Unix seconds. */
export interface LimitWindow {
  utilization: number
  resetsAt: number
}

/** The account's limits as the harness last reported them (spec §12.8). A
 * window it did not report is `null`, never estimated. */
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

/** Where the Planner put a shown plan (§13.9). */
export type PlanPlace = 'inline' | 'side' | 'page'

/** The Planner showed a plan (§13.9). Transient and live only: it is never
 * replayed, and only the tab named `targetTab` acts on it. */
export interface PlanShowFrame {
  threadId: string
  /** The tab that sent the turn; `null` when it named none. */
  targetTab: string | null
  workflowId: string
  version: number
  taskNumber: number | null
  place: PlanPlace
}

/** One subagent as the conversation shows it (§22.2): the `card` of its
 * `Subagent` entry (§23.8), and the whole of each `subagent` frame. */
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
