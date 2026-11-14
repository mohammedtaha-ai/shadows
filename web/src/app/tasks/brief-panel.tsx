// One job: an attempt's saved context drawn as Core recorded it, from the plan version the attempt began on.

import { useQuery } from '@tanstack/react-query'
import { attemptBriefQuery, contextNotRecorded, type AttemptBrief } from '@/api/runs'
import { ErrorLine } from '../error-line'

type TaskContract = NonNullable<AttemptBrief['context']['task']['contract']>
type ProducedOutput = NonNullable<TaskContract['produces']>[number]
type ContractUse = NonNullable<TaskContract['uses']>[number]
type ContractSource = ContractUse['source']
type SymbolWork = NonNullable<TaskContract['symbols']>[number]
type ResolvedContract = NonNullable<AttemptBrief['context']['contracts']>[number]
type TaskPart = AttemptBrief['context']['parts'][number]

/**
 * An open object as readable text: each key with its value, nested objects in brackets,
 * nulls left out. It names only the keys Core sent and reads no meaning into them;
 * the API types a part and its `via` as open objects, so they are read this way.
 */
function plain(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (Array.isArray(value)) return value.map(plain).join(', ')
  if (typeof value === 'object') {
    return Object.entries(value as Record<string, unknown>)
      .filter(([, v]) => v !== null && v !== undefined)
      .map(([k, v]) => (typeof v === 'object' ? `${k} (${plain(v)})` : `${k} ${plain(v)}`))
      .join(', ')
  }
  return String(value)
}

function symbolText(symbol: SymbolWork): string {
  return `${symbol.kind} ${symbol.name} in ${symbol.path}`
}

/**
 * Where a used output comes from: an existing path and symbol, or a task's output. The
 * producing task is named by its title when Core resolved one, otherwise by its number.
 */
function sourceText(source: ContractSource, producer?: string | null): string {
  if (source.kind === 'existing') return `existing ${source.path}${source.symbol !== undefined ? ` ${source.symbol}` : ''}`
  return `output ${source.output} of ${producer ?? `T${source.target.task}`}`
}

/** A resolved use: its source, then the symbol or artifact its producer promises, when there is one. */
function resolved(contract: ResolvedContract): string {
  const promised = [
    contract.symbol != null ? symbolText(contract.symbol) : null,
    contract.artifact != null ? `artifact ${contract.artifact}` : null,
  ].filter((x) => x !== null)
  const source = sourceText(contract.reference, contract.producer_title)
  return promised.length === 0 ? source : `${source}: ${promised.join(', ')}`
}

/** One layout part with the paths of this task it owns; both come as open objects. */
function partText(part: TaskPart): string {
  const paths = part.paths.map((p) => `${p.path} (via ${plain(p.via)})`).join('; ')
  return `${plain(part.part)}: ${paths}`
}

/** One planned output: its id, the symbol or file it names in the same contract, and the acceptance items that prove it. */
function produced(output: ProducedOutput, contract: TaskContract): string {
  const symbol = output.symbol_id == null ? undefined : contract.symbols?.find((s) => s.id === output.symbol_id)
  const what = symbol !== undefined
    ? symbolText(symbol)
    : output.symbol_id != null ? `symbol ${output.symbol_id}` : output.artifact ?? null
  const parts = [output.id]
  if (what !== null) parts.push(what)
  if (symbol !== undefined && output.artifact != null) parts.push(`artifact ${output.artifact}`)
  const proof = output.acceptance !== undefined && output.acceptance.length > 0 ? ` (acceptance ${output.acceptance.join(', ')})` : ''
  return `${parts.join(': ')}${proof}`
}

/** One thing the task relies on: an existing path and symbol, or another task's output; then why. */
function used(use: ContractUse): string {
  return `${sourceText(use.source)}: ${use.purpose}`
}

function List({ label, items }: { label: string; items: readonly string[] | undefined }) {
  if (items === undefined || items.length === 0) return null
  return (
    <div>
      <p className="font-semibold">{label}</p>
      <ul className="list-disc ps-5">{items.map((item, i) => <li key={i} dir="auto" className="whitespace-pre-wrap break-words">{item}</li>)}</ul>
    </div>
  )
}

function Brief({ brief }: { brief: AttemptBrief }) {
  const { context, attempt } = brief
  const contract = context.task.contract
  return (
    <section aria-label="Attempt brief" className="space-y-2">
      <p>
        <span className="font-semibold">Original plan</span>{' '}
        <span dir="auto">{context.plan.title}</span>, version {context.plan.version}; saved {brief.captured_at}
      </p>
      <p dir="auto" className="whitespace-pre-wrap break-words">{context.task.goal}</p>
      <p className="text-muted-foreground">
        base {attempt.model} · map revision {context.map_revision} · standards {context.standards.base}
        {context.standards.additions != null && ` with ${context.standards.additions} additions`}
      </p>
      <p>
        <span className="font-semibold">Attempt began from </span>
        {attempt.addition === null ? 'Start' : "the Planner's addition"}
        {attempt.addition !== null && <span dir="auto" className="block whitespace-pre-wrap break-words">{attempt.addition}</span>}
      </p>
      <List label="Reads" items={context.task.reads} />
      <List label="Writes" items={context.task.writes} />
      {contract != null && (
        <div aria-label="Original contract" className="space-y-1">
          <List label="Steps" items={contract.steps} />
          <List label="Invariants" items={contract.invariants} />
          <List label="Forbidden" items={contract.forbidden} />
          <List label="Produces" items={contract.produces?.map((o) => produced(o, contract))} />
          <List label="Uses" items={contract.uses?.map(used)} />
        </div>
      )}
      <List label="Resolved contracts" items={context.contracts?.map(resolved)} />
      <div aria-label="Original layout" className="space-y-1">
        <List label="Parts" items={context.parts.map(partText)} />
        <List label="Unowned paths" items={context.unowned_paths} />
      </div>
      <div aria-label="Planned consumers">
        <p className="font-semibold">Planned consumers of the original version</p>
        {brief.consumers.length === 0 ? (
          <p className="text-muted-foreground">None planned in that version.</p>
        ) : (
          <ul className="list-disc ps-5">
            {brief.consumers.map((c) => (
              <li key={`${c.task}:${c.use_index}`} dir="auto">
                T{c.task} {c.title}{c.output !== null && ` · uses ${c.output}`}: {c.purpose}
              </li>
            ))}
          </ul>
        )}
        {brief.consumers_more && <p className="text-muted-foreground">The first consumers only; more are planned.</p>}
      </div>
    </section>
  )
}

/** The context one attempt received. An attempt from before contexts were saved says so and is not rebuilt. */
export function BriefPanel({ attemptId }: { attemptId: string }) {
  const brief = useQuery(attemptBriefQuery(attemptId))
  if (brief.data !== undefined) return <Brief brief={brief.data} />
  if (brief.error === null) return <p className="text-faint-foreground">Reading the brief…</p>
  if (contextNotRecorded(brief.error)) {
    return <p>This attempt began before its context was saved, so there is no brief to show.</p>
  }
  return <ErrorLine error={brief.error} />
}
