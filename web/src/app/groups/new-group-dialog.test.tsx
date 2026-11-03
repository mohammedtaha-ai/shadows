// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, typeInto, until, type TestApp } from '@/test/test-app'

const agents = Array.from({ length: 7 }, (_, i) => ({ slug: `agent-${i}`, name: `Agent ${i}` }))
const group = { id: 'g1', name: 'New team', members: [], needs_you: false, created_at: 'now' }
let app: TestApp | undefined
afterEach(() => { app?.unmount(); app = undefined })

async function open(loseAnswer = false) {
  let attempts = 0
  const a = app = await startApp('/aylul', {
    ...answers(),
    'GET /api/aylul/agents': { project_id: 'p1', agents },
    'GET /api/groups': () => Response.json(attempts > 0 ? [group] : []),
    'GET /api/groups/g1': group,
    'GET /api/groups/g1/messages': [],
    'POST /api/groups': () => {
      attempts++
      if (loseAnswer && attempts === 1) throw new TypeError('Failed to fetch')
      return Response.json(group)
    },
  })
  await until(() => a.button('New group')?.disabled === false)
  act(() => a.button('New group')?.click())
  await until(() => a.checkbox('Agent 0') !== undefined)
  return a
}

it('does not let the seventh agent be picked', async () => {
  const a = await open()
  for (let i = 0; i < 6; i++) act(() => a.checkbox(`Agent ${i}`)!.click())
  expect(a.checkbox('Agent 6')?.disabled).toBe(true)
})

it.each([false, true])('opens the new empty group and refreshes the sidebar after create (lost answer: %s)', async lost => {
  const a = await open(lost)
  typeInto(document.querySelector<HTMLInputElement>('#new-group-name')!, 'New team')
  act(() => a.checkbox('Agent 0')!.click())
  act(() => a.button('Create group')!.click())
  if (lost) {
    await until(() => a.bodies.length === 1 && a.button('Create group')?.disabled === false)
    act(() => a.button('Create group')!.click())
  }
  await until(() => a.path() === '/aylul/groups/g1')
  await until(() => !!a.container.querySelector('nav a[href="/aylul/groups/g1"]'))
  expect(a.bodies).toHaveLength(lost ? 2 : 1)
  if (lost) expect(a.bodies[1]).toEqual(a.bodies[0])
})
