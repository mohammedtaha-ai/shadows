// @vitest-environment happy-dom
//
// The global Settings page over a faked daemon (§15.6, §13.11): reached from
// the sidebar, it saves the active limit as one command and refuses a number
// out of range before sending it.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

const field = () => document.querySelector<HTMLInputElement>('input[aria-label="Active projects"]')

describe('settings', () => {
  it('saves the active limit', async () => {
    const table = answers()
    // The daemon keeps what was saved, so a refetch after the save reads it.
    let limit = 5
    table['GET /api/code/settings'] = () => Response.json({ active_limit: limit })
    table['PUT /api/code/settings'] = async (r: Request) => {
      ;({ active_limit: limit } = (await r.json()) as { active_limit: number })
      return Response.json({ active_limit: limit })
    }
    const app = (open = await startApp('/', table))
    const link = () => document.querySelector<HTMLAnchorElement>('a[href="/settings"]')
    await until(() => link()?.textContent?.trim() === 'Settings')
    act(() => link()?.click())
    await until(() => app.path() === '/settings' && field()?.value === '5')

    typeInto(field()!, '25')
    expect(app.text()).toContain('A whole number from 1 to 20.')
    expect(app.button('Save')?.disabled).toBe(true)

    typeInto(field()!, '8')
    act(() => app.button('Save')?.click())
    await until(() => app.calls.includes('PUT /api/code/settings'))
    expect(app.bodies.at(-1)).toEqual({ active_limit: 8, command_id: expect.stringMatching(/.+/) })
    // Save is also disabled while pending: wait for the response to reach
    // the cache before checking the completed field and button state.
    await until(() => app.queryClient.getQueryData<{ active_limit: number }>(['code', 'settings'])?.active_limit === 8)
    await until(() => app.button('Save')?.disabled === true && field()?.value === '8')
    expect(field()?.value).toBe('8')
  })

  it('follows a value changed elsewhere while the field is untouched', async () => {
    const table = answers()
    let limit = 5
    table['GET /api/code/settings'] = () => Response.json({ active_limit: limit })
    const app = (open = await startApp('/settings', table))
    await until(() => field()?.value === '5')

    // Another tab saved 7: a refetch brings it, and the field shows it.
    limit = 7
    await act(() => app.queryClient.invalidateQueries({ queryKey: ['code', 'settings'] }))
    await until(() => field()?.value === '7')
    expect(app.button('Save')?.disabled).toBe(true)
  })
})
