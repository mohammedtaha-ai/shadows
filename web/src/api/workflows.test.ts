import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { workflowKeys, planQuery, plansQuery, planVersionsQuery, planMapQuery, savePlanAnswer } from './workflows'
import type { Plan } from './workflows'
it('workflow reads and refresh keys preserve their arrays', () => {
  expect(workflowKeys.all()).toEqual(['workflows'])
  expect(workflowKeys.plans()).toEqual(['workflows', 'plan'])
  expect(planQuery('w').queryKey).toEqual(['workflows', 'plan', 'w'])
  expect(planMapQuery('p').queryKey).toEqual(['workflows', 'map', 'p'])
  expect(plansQuery('p', true).queryKey).toEqual(['workflows', 'list', 'p', { archived: true }])
  expect(planVersionsQuery('p').queryKey).toEqual(['workflows', 'versions', 'p'])
})
it('a stale plan read cannot overwrite a saved binding answer', async () => {
  const client = new QueryClient()
  let release!: (value: Plan) => void
  const saved = { id: 'w', revision: 2 } as Plan
  const pending = client.fetchQuery({ ...planQuery('w'), queryFn: () => new Promise<Plan>(resolve => { release = resolve }) }).catch(() => undefined)
  await savePlanAnswer(client, saved)
  release({ ...saved, revision: 1 })
  await pending
  expect(client.getQueryData(workflowKeys.plan('w'))).toEqual(saved)
  client.clear()
})
