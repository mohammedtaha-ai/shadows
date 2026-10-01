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
    table['PUT /api/code/settings'] = async (r: Request) => {
      const { active_limit } = (await r.json()) as { active_limit: number }
      return Response.json({ active_limit })
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
    // Saved: the number now stands, so there is nothing left to save.
    await until(() => app.button('Save')?.disabled === true)
    expect(field()?.value).toBe('8')
  })
})
