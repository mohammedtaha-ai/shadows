// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'


import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '@/test/test-app'

const PAGE = '/projects/p1/settings'
let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

const textarea = () => document.querySelector('textarea')


function click(app: TestApp, name: string) {
  const button = app.button(name)
  if (button === undefined) throw new Error(`no button reads "${name}"`)
  act(() => button.click())
}

describe('project settings', () => {
  it('saving instructions sends the body with a command id', async () => {
    const app = (open = await startApp(
      PAGE,
      answers({
        instructions: { body: 'Plan in small steps.', number: 1, created_at: '2026-09-24T08:00:00Z' },
      }),
    ))
    await until(() => textarea()?.value === 'Plan in small steps.')
    expect(textarea()?.getAttribute('dir')).toBe('auto')
    expect(app.text()).toContain('Last changed')

    typeInto(textarea()!, 'اكتب الخطة بخطوات صغيرة.')
    click(app, 'Save')
    await until(() => app.calls.includes('PUT /api/projects/p1/planner-instructions'))
    const sent = app.bodies.at(-1) as { body: string; command_id: string }
    expect(sent.body).toBe('اكتب الخطة بخطوات صغيرة.')
    expect(sent.command_id).toMatch(/.+/)
    await until(() => app.text().includes('version 2'))
  })

  it('a newer version fetched in the background keeps unsaved edits', async () => {
    const table = answers({
      instructions: { body: 'Plan in small steps.', number: 1, created_at: '2026-09-24T08:00:00Z' },
    })
    const app = (open = await startApp(PAGE, table))
    await until(() => textarea()?.value === 'Plan in small steps.')
    typeInto(textarea()!, 'Plan in small steps, and ask first.')

    // Another tab saved version 2; a refetch (on focus, say) brings it here.
    table['GET /api/projects/p1/planner-instructions'] = {
      body: 'Saved elsewhere.',
      number: 2,
      created_at: '2026-09-25T08:00:00Z',
    }
    await act(() => app.queryClient.invalidateQueries())
    await until(() => app.text().includes('version 2'))
    expect(textarea()?.value).toBe('Plan in small steps, and ask first.')
  })

  it("another project's settings do not show this one's draft", async () => {
    const table = answers()
    table['GET /api/projects/p2/planner-instructions'] = null
    const app = (open = await startApp(PAGE, table))
    await until(() => textarea() !== null)
    typeInto(textarea()!, 'Only for p1.')
    expect(app.button('Connect')).toBeUndefined()

    const { router } = await import('@/router')
    await act(() =>
      router.navigate({ to: '/projects/$projectId/settings', params: { projectId: 'p2' } }),
    )
    await until(() => app.calls.includes('GET /api/projects/p2/planner-instructions'))
    await until(() => textarea() !== null)
    expect(textarea()?.value).toBe('')
  })
})
