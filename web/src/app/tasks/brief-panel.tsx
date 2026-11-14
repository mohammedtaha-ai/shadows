// One job: an attempt's saved context drawn as Core recorded it, from the plan version the attempt began on.

import { useQuery } from '@tanstack/react-query'
import { attemptBriefQuery, contextNotRecorded, type AttemptBrief } from '@/api/runs'
import { ErrorLine } from '../error-line'

/** Fields Core gives as open objects; shown as the plain text they carry, never interpreted. */
function loose(value: unknown): string {
  return JSON.stringify(value)
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
          <List label="Produces" items={contract.produces?.map(loose)} />
          <List label="Uses" items={contract.uses?.map(loose)} />
        </div>
      )}
      <List label="Resolved contracts" items={context.contracts?.map(loose)} />
      <div aria-label="Original layout" className="space-y-1">
        <List label="Parts" items={context.parts.map((p) => `${loose(p.part)}: ${p.paths.map((x) => `${x.path} (${loose(x.via)})`).join(', ')}`)} />
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
