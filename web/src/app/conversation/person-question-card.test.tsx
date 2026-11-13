// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { entryOfKind } from '@/test/contract-fixtures'
import { startApp, until, type TestApp, type Answer } from '@/test/test-app'

const card = (chosen: number | null = null) =>
  ({ question_id: 'q1', question: 'Which database?', options: [{ label: 'SQLite' }, { label: 'Postgres' }], chosen })
const asked = (chosen: number | null = null) => ({ ...entryOfKind('q-entry', 'PersonQuestion', 'Which database?'), card: card(chosen) })
const answerRoute = 'POST /api/threads/t1/questions/q1/answer'
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
async function open(entry: ReturnType<typeof asked>, overrides: Record<string, Answer> = {}) {
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers(), 'GET /api/threads/t1/entries': [entry], ...overrides })
  await until(() => a.text().includes('Which database?'))
  return a
}

it('renders_the_planners_options', async () => {
  const a = await open(asked())
  expect(a.button('SQLite')).toBeDefined()
  expect(a.button('Postgres')).toBeDefined()
})

it('a_click_answers_once', async () => {
  const a = await open(asked(), { [answerRoute]: () => Response.json({ id: 'm1' }, { status: 202 }) })
  act(() => { a.button('Postgres')!.click(); a.button('Postgres')!.click() })
  await until(() => a.button('Postgres') === undefined && a.text().includes('You chose: Postgres'))
  expect(a.calls.filter(call => call === answerRoute)).toHaveLength(1)
  expect(a.bodies.at(-1)).toEqual({ command_id: expect.any(String), option: 1 })
  expect(a.button('SQLite')).toBeUndefined()
})

it('an_answered_question_has_no_buttons', async () => {
  const a = await open(asked(0))
  expect(a.text()).toContain('You chose: SQLite')
  expect(a.button('SQLite')).toBeUndefined()
  expect(a.button('Postgres')).toBeUndefined()
  const conflicted = await open(asked(), { [answerRoute]: () => Response.json({ code: 'COMMAND_CONFLICT', message: 'answered' }, { status: 409 }) })
  act(() => conflicted.button('SQLite')!.click())
  await until(() => conflicted.calls.includes(answerRoute) && conflicted.button('Postgres') === undefined)
})

it('a_forked_question_has_no_buttons', async () => {
  const a = await open(asked(), { [answerRoute]: () => Response.json({ code: 'INVALID_COMMAND', message: 'asked in another conversation' }, { status: 422 }) })
  act(() => a.button('SQLite')!.click())
  await until(() => a.calls.includes(answerRoute) && a.button('SQLite') === undefined)
  expect(a.text()).toContain('Postgres')
})
