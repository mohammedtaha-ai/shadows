// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

const fixture = vi.hoisted(() => ({ desktop: false }))
vi.mock('@/desktop/runtime', () => ({ isDesktop: () => fixture.desktop }))

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
  fixture.desktop = false
})

const general = () => document.querySelector('form[aria-label="General preferences"]')

describe('settings', () => {
  it("the sidebar's Settings opens the dialog on General", async () => {
    const app = (open = await startApp('/', answers()))
    act(() => app.button('Settings')?.click())
    await until(() => general() !== null)
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
    expect([...nav!.querySelectorAll('h2, button')].map(element => element.textContent).join(' ')).toBe('You Account General AI Providers Agents Plugins & Skills Code Usage This machine AYLUL character Local Core')
  })

  it('Escape closes the dialog and keeps the page', async () => {
    const app = (open = await startApp('/projects/p1', answers()))
    await until(() => app.path() === '/projects/p1/new')
    const path = app.path()
    const trigger = app.button('Settings')!
    act(() => { trigger.focus(); trigger.click() })
    await until(() => general() !== null)
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
    await until(() => general() !== null)
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
    await until(() => general() !== null)
    expect(app.path()).toBe('/')
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('General')
  })

  it('a desktop-only section in the browser falls back to General', async () => {
    open = await startApp('/?settings=character', answers())
    await until(() => general() !== null)
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('General')
    expect(document.querySelector('nav[aria-label="Settings sections"]')?.textContent).not.toContain('This machine')
    expect(document.querySelector('nav[aria-label="Settings sections"]')?.textContent).toContain('Account')
  })

  it('an unknown or hidden section falls back to General', async () => {
    open = await startApp('/?settings=unknown-section', answers())
    await until(() => general() !== null)
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('General')
  })

  it('opening_code_shows_its_servers', async () => {
    open = await startApp('/projects/p1/new?settings=code', { ...answers(), 'GET /api/projects/p1/code/servers': [
      { id: 'gopls', added: false, wanted: true, extensions: ['.go'], state: { state: 'running' }, install: null },
    ] })
    await until(() => document.querySelector('ul[aria-label="Language servers"]')?.textContent?.includes('gopls') === true)
    expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('Code')
  })
})
