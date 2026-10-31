// One job: the words of a tool group's summary and of each step's line.

import type { CallKind } from '@/api/threads'
import type { ToolStep } from './activity-rows'

interface Words {
  one: string
  many: (n: number) => string
  /** Counted by distinct targets rather than calls. */
  byTarget: boolean
}

const WORDS: Readonly<Record<CallKind | 'other', Words>> = {
  read: { one: 'read a file', many: (n) => `read ${n} files`, byTarget: true },
  edit: { one: 'edited a file', many: (n) => `edited ${n} files`, byTarget: true },
  delete: { one: 'deleted a file', many: (n) => `deleted ${n} files`, byTarget: true },
  move: { one: 'moved a file', many: (n) => `moved ${n} files`, byTarget: true },
  search: { one: 'searched once', many: (n) => `searched ${n} times`, byTarget: false },
  execute: { one: 'ran a command', many: (n) => `ran ${n} commands`, byTarget: false },
  fetch: { one: 'fetched a page', many: (n) => `fetched ${n} pages`, byTarget: false },
  think: { one: 'thought', many: () => 'thought', byTarget: false },
  switch_mode: { one: 'switched mode', many: () => 'switched mode', byTarget: false },
  other: { one: 'used a tool', many: (n) => `used ${n} tools`, byTarget: false },
}

const VERBS: Readonly<Record<CallKind | 'other', string | null>> = {
  read: 'Read',
  edit: 'Edit',
  delete: 'Delete',
  move: 'Move',
  search: 'Search',
  execute: 'Run',
  fetch: 'Fetch',
  think: 'Think',
  switch_mode: 'Switch mode',
  other: null,
}

const wordsOf = (kind: CallKind | null): CallKind | 'other' =>
  kind !== null && kind in WORDS ? kind : 'other'

/** ` +A −R`, each part only when it is above zero. */
function counts(added: number | null, removed: number | null): string | null {
  const parts = [
    added !== null && added > 0 ? `+${added}` : null,
    removed !== null && removed > 0 ? `−${removed}` : null,
  ].filter((part) => part !== null)
  return parts.length === 0 ? null : parts.join(' ')
}

/** "Read 3 files, ran 2 commands, edited a file +36", kinds in the order they first appear. */
export function groupSummary(steps: readonly ToolStep[]): string {
  const order: (CallKind | 'other')[] = []
  for (const step of steps) {
    const key = wordsOf(step.kind)
    if (!order.includes(key)) order.push(key)
  }
  const phrases = order.map((key) => {
    const own = steps.filter((step) => wordsOf(step.kind) === key)
    const words = WORDS[key]
    const targets = new Set(own.flatMap((step) => (step.target === null ? [] : [step.target])))
    const n = words.byTarget && targets.size > 0 ? targets.size : own.length
    const phrase = n === 1 ? words.one : words.many(n)
    if (key !== 'edit') return phrase
    const sum = (pick: (step: ToolStep) => number | null) =>
      own.some((step) => pick(step) !== null)
        ? own.reduce((total, step) => total + (pick(step) ?? 0), 0)
        : null
    const edited = counts(
      sum((step) => step.added),
      sum((step) => step.removed),
    )
    return edited === null ? phrase : `${phrase} ${edited}`
  })
  const text = phrases.join(', ')
  return text.charAt(0).toUpperCase() + text.slice(1)
}

/** One step's line: what it did, and for an edit its counts apart. */
export function stepLine(step: ToolStep): { text: string; counts: string | null } {
  const own = step.kind === 'edit' ? counts(step.added, step.removed) : null
  if (step.sentence !== null) return { text: step.sentence, counts: own }
  const verb = VERBS[wordsOf(step.kind)]
  if (verb === null || step.target === null) return { text: step.title, counts: own }
  const lines =
    step.lines !== null && step.lines.length === 2 ? ` (${step.lines[0]}–${step.lines[1]})` : ''
  return { text: `${verb} ${step.target}${lines}`, counts: own }
}
