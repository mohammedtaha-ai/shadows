// @vitest-environment happy-dom
//
// The global Settings page over a faked daemon (§15.6, §13.11): reached from
// the sidebar, it saves the active limit as one command and refuses a number
// out of range before sending it.

import { codeKeys } from '@/api/code'
import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '@/test/test-app'

const fixture = vi.hoisted(() => ({ desktop: false }))
vi.mock('@/desktop/runtime', () => ({ isDesktop: () => fixture.desktop }))

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
  fixture.desktop = false
})

const field = () => document.querySelector<HTMLInputElement>('input[aria-label="Active projects"]')

describe('settings', () => {
  it("the sidebar's Settings opens the dialog on General", async () => {
    const app = (open = await startApp('/', answers()))
    act(() => app.button('Settings')?.click())
    await until(() => field()?.value === '5')
    expect(document.querySelector('[role="dialog"]')).not.toBeNull()
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('General')
    expect(app.path()).toBe('/')
    expect(window.location.search).toContain('settings=general')
  })

  it('the dialog lists its sections in three groups', async () => {
    const app = (open = await startApp('/', answers()))
    fixture.desktop = true
    act(() => app.button('Settings')?.click())
    await until(() => document.querySelector('[role="dialog"]') !== null)
    const nav = document.querySelector('nav[aria-label="Settings sections"]')
    expect([...nav!.querySelectorAll('h2, button')].map(element => element.textContent).join(' ')).toBe('You Account General AI Providers Agents Plugins & Skills Usage This machine AYLUL character Local Core')
  })

  it('Escape closes the dialog and keeps the page', async () => {
    const app = (open = await startApp('/projects/p1', answers()))
    await until(() => app.path() === '/projects/p1/new')
    const path = app.path()
    const trigger = app.button('Settings')!
    act(() => { trigger.focus(); trigger.click() })
    await until(() => field() !== null)
    act(() => document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })))
    await until(() => document.querySelector('[role="dialog"]') === null)
    expect(app.path()).toBe(path)
    expect(window.location.search).not.toContain('settings')
    await until(() => document.activeElement === trigger)
  })

  it('a backdrop click closes the dialog and keeps the page', async () => {
    const app = (open = await startApp('/', answers()))
    const trigger = app.button('Settings')!
    act(() => { trigger.focus(); trigger.click() })
    await until(() => field() !== null)
    const backdrop = document.querySelector<HTMLElement>('[data-slot="dialog-overlay"]')!
    act(() => {
      backdrop.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
      backdrop.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, button: 0 }))
      backdrop.click()
    })
    await until(() => document.querySelector('[role="dialog"]') === null)
    expect(app.path()).toBe('/')
    expect(window.location.search).not.toContain('settings')
    await until(() => document.activeElement === trigger)
  })

  it('/settings redirects to the dialog', async () => {
    const app = (open = await startApp('/settings', answers()))
    await until(() => field() !== null)
    expect(app.path()).toBe('/')
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('General')
  })

  it('a desktop-only section in the browser falls back to General', async () => {
    open = await startApp('/?settings=character', answers())
    await until(() => field() !== null)
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('General')
    expect(document.querySelector('nav[aria-label="Settings sections"]')?.textContent).not.toContain('This machine')
    expect(document.querySelector('nav[aria-label="Settings sections"]')?.textContent).toContain('Account')
  })

  it('an unknown or hidden section falls back to General', async () => {
    open = await startApp('/?settings=unknown-section', answers())
    await until(() => field() !== null)
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('General')
  })

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
    const link = () => app.button('Settings')
    await until(() => link()?.textContent?.trim() === 'Settings')
    act(() => link()?.click())
    await until(() => app.path() === '/' && field()?.value === '5')

    typeInto(field()!, '25')
    expect(app.text()).toContain('A whole number from 1 to 20.')
    expect(app.button('Save')?.disabled).toBe(true)

    typeInto(field()!, '8')
    act(() => app.button('Save')?.click())
    await until(() => app.calls.includes('PUT /api/code/settings'))
    expect(app.bodies.at(-1)).toEqual({ active_limit: 8, command_id: expect.stringMatching(/.+/) })
    // Save is also disabled while pending: wait for the response to reach
    // the cache before checking the completed field and button state.
    await until(() => app.queryClient.getQueryData<{ active_limit: number }>(codeKeys.settings())?.active_limit === 8)
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
    await act(() => app.queryClient.invalidateQueries({ queryKey: codeKeys.settings() }))
    await until(() => field()?.value === '7')
    expect(app.button('Save')?.disabled).toBe(true)
  })
})
