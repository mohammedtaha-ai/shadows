// Share session-ready composer fixtures and cleanup so both suites exercise the same send path.

import { afterEach, vi } from 'vitest'
import { fakeChoices } from './contract-fixtures'
import { answers } from './fake-daemon'
import { type TestApp, startApp } from '@/app/test-app'

export const composer = { open: null as TestApp | null }
afterEach(() => {
  composer.open?.unmount()
  composer.open = null
  vi.restoreAllMocks()
})

export async function start(url: string, table: ReturnType<typeof answers>): Promise<TestApp> {
  composer.open = await startApp(url, {
    'PUT /api/threads/t1/session/mode': async (r: Request) => {
      const { mode } = await r.json() as { mode: string }
      return Response.json({ ...fakeChoices, current: { ...fakeChoices.current, mode } })
    },
    ...table,
  })
  return composer.open
}

/** The session answered and the thread's turns are known: Send can act. */
export const ready = (app: TestApp) => () =>
  app.button('fake-large') !== undefined && app.calls.includes('GET /api/threads/t1/operations')

export const textarea = (app: TestApp) => {
  const box = app.container.querySelector('textarea')
  if (box === null) throw new Error('no message box')
  return box
}

export const turnStarts = (app: TestApp) =>
  app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length
