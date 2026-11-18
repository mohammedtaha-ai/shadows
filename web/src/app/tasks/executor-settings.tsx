import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { catalogUnavailable, harnessCatalogQuery, harnessesQuery } from '@/api/harnesses'
import type { ExecutorChoice, ExecutorInput } from '@/api/runs'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

const SELECT = 'w-full min-w-0 rounded-md border border-border bg-background px-2 py-1.5 text-foreground'
const EMPTY: ExecutorInput = { harness: '', model: '', effort: null }

export function ExecutorSettings({ saved, pending, error, save }: {
  saved: ExecutorChoice | null
  pending: boolean
  error: Error | null
  save: (input: ExecutorInput) => Promise<ExecutorChoice>
}) {
  const [edits, setEdits] = useState<ExecutorInput | null>(null)
  const selection = edits ?? saved ?? EMPTY
  const { harness, model, effort } = selection
  const harnesses = useQuery(harnessesQuery)
  const choices = harnesses.data?.filter(item => item.executor) ?? []
  const cli = choices.find(item => item.kind === harness)
  const catalog = useQuery({ ...harnessCatalogQuery(harness), enabled: cli?.available === true })
  const models = catalog.data?.models ?? []
  const selected = models.find(item => item.id === model)
  const efforts = selected?.efforts ?? []
  const unavailable = catalogUnavailable(catalog.error)
  const changed = saved === null || saved.harness !== harness || saved.model !== model || saved.effort !== effort
  const canSave = changed && !unavailable && cli?.available === true && selected !== undefined &&
    (efforts.length === 0 || efforts.some(item => item.enabled && item.id === effort))

  return (
    <section aria-label="Executor settings" className="space-y-3 border-t border-border pt-4">
      <h3 className="font-semibold">Executor</h3>
      {saved === null ? <p className="text-xs text-muted-foreground">No executor chosen yet</p> : (
        <p className="break-words text-xs text-muted-foreground">
          Saved: {choices.find(item => item.kind === saved.harness)?.label ?? saved.harness} · {saved.model}
          {saved.effort === null ? '' : ` · ${saved.effort}`} · Chosen by {saved.chosen_by === 'person' ? 'you' : 'the Planner'}
        </p>
      )}
      {harnesses.error && <ErrorLine error={harnesses.error} />}
      <form className="space-y-3" onSubmit={(event) => {
        event.preventDefault()
        if (!canSave || pending) return
        void save({ harness, model, effort: efforts.length === 0 ? null : effort }).then(
          () => setEdits(null), () => {},
        )
      }}>
        <div className="grid gap-3 sm:grid-cols-3">
          <label className="min-w-0 space-y-1 text-xs text-muted-foreground">
            <span>Executor CLI</span>
            <select aria-label="Executor CLI" className={SELECT} value={harness} disabled={pending || harnesses.data === undefined}
              onChange={(event) => setEdits({ harness: event.target.value, model: '', effort: null })}>
              <option value="">Choose CLI</option>
              {harness !== '' && cli === undefined && <option value={harness} disabled>{harness}</option>}
              {choices.map(item => (
                <option key={item.kind} value={item.kind}
                  disabled={!item.available || (item.kind === harness && unavailable)}>
                  {item.label}{!item.available && item.reason ? ` · ${item.reason}` : ''}
                </option>
              ))}
            </select>
          </label>
          <label className="min-w-0 space-y-1 text-xs text-muted-foreground">
            <span>Executor model</span>
            <select aria-label="Executor model" className={SELECT} value={model} disabled={pending || catalog.data === undefined}
              onChange={(event) => setEdits({ harness, model: event.target.value, effort: null })}>
              <option value="">{catalog.isFetching ? 'Loading models…' : 'Choose model'}</option>
              {model !== '' && selected === undefined && <option value={model} disabled>{model}</option>}
              {models.map(item => <option key={item.id} value={item.id}>{item.label}</option>)}
            </select>
          </label>
          {efforts.length > 0 && (
            <label className="min-w-0 space-y-1 text-xs text-muted-foreground">
              <span>Executor effort</span>
              <select aria-label="Executor effort" className={SELECT} value={effort ?? ''} disabled={pending}
                onChange={(event) => setEdits({ harness, model, effort: event.target.value || null })}>
                <option value="">Choose effort</option>
                {effort !== null && !efforts.some(item => item.id === effort) &&
                  <option disabled value={effort}>{effort}</option>}
                {efforts.map(item => <option key={item.id} value={item.id} disabled={!item.enabled}>
                  {item.label}{!item.enabled && item.reason ? ` · ${item.reason}` : ''}
                </option>)}
              </select>
            </label>
          )}
        </div>
        {cli && !cli.available && cli.reason && <p className="text-xs text-muted-foreground">{cli.reason}</p>}
        {catalog.error && (unavailable ? <ErrorLine error={catalog.error} /> : (
          <div className="flex items-center gap-2 text-xs text-destructive-foreground">
            <span role="alert">Could not read {cli?.label ?? harness}'s models.</span>
            <Button type="button" size="sm" variant="outline" disabled={catalog.isFetching}
              onClick={() => { void catalog.refetch() }}>Retry</Button>
          </div>
        ))}
        <Button type="submit" size="sm" disabled={!canSave || pending}>{pending ? 'Saving…' : 'Save executor'}</Button>
        {error && <ErrorLine error={error} />}
      </form>
    </section>
  )
}
