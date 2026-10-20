// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { AccountSection } from './sections/account'
import type { AuthView } from '@/desktop/auth'
vi.mock('./you-card', () => ({ YouCard: () => null }))

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
const auth: AuthView = { state: 'signed_in', hasEnrollment: true, principal: { accountId: 'account', deviceId: 'device', sessionId: 'session' }, providerLogout: null }
let host: HTMLDivElement, root: Root
const flush = () => act(async () => { await new Promise(resolve => setTimeout(resolve, 0)) })
const buttons = () => [...host.querySelectorAll<HTMLButtonElement>('button')]
async function click(label: string) {
  const button = buttons().find(button => button.textContent === label)
  expect(button, label).toBeTruthy()
  await act(async () => button!.click())
  await flush()
}
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })
  Object.assign(window, { __TAURI_INTERNALS__: {} })
  host = document.createElement('div'); document.body.append(host); root = createRoot(host)
  invoke.mockReset().mockResolvedValue(auth)
})
afterEach(async () => {
  await act(async () => root.unmount()); host.remove(); vi.restoreAllMocks()
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})
async function render() { await act(async () => root.render(<AccountSection />)); await flush() }

it('signed in shows three actions and no Sign in', async () => {
  await render()
  expect(buttons().map(button => button.textContent)).toEqual(['Sign out', 'Sign out all sessions', 'Revoke this device'])
  expect(host.textContent).toContain('account')
  expect(invoke.mock.calls.some(([command]) => String(command).includes('assistant'))).toBe(false)
})
it('an action in progress disables every button', async () => {
  await render()
  let finish!: (view: AuthView) => void
  invoke.mockImplementation(() => new Promise<AuthView>(resolve => { finish = resolve }))
  await click('Sign out')
  expect(buttons()).toHaveLength(3)
  expect(buttons().every(button => button.disabled)).toBe(true)
  await act(async () => finish({ ...auth, state: 'signed_out', principal: null }))
  expect(buttons().map(button => button.textContent)).toEqual(['Sign in'])
})
it('the state is checked when the section opens', async () => {
  await render()
  expect(invoke).toHaveBeenCalledTimes(1)
  expect(invoke).toHaveBeenCalledWith('auth_action', { action: 'status' })
  expect(host.textContent).not.toContain('Check session')
  await act(async () => root.render(null))
  await render()
  expect(invoke).toHaveBeenCalledTimes(2)
})
it('global logout and device revoke retain explicit confirmation', async () => {
  await render()
  const confirm = vi.fn(() => false)
  Object.assign(window, { confirm })
  await click('Sign out all sessions'); await click('Revoke this device')
  expect(invoke).toHaveBeenCalledTimes(1)
  confirm.mockReturnValue(true)
  await click('Sign out all sessions'); await click('Revoke this device')
  expect(invoke.mock.calls.map(([, args]) => args.action)).toEqual(['status', 'logout_all', 'revoke_device'])
})
it('sign-in can be cancelled independently of its pending worker', async () => {
  invoke.mockImplementation(async (command: string, args?: { action: string }) => {
    if (command === 'auth_action' && args?.action === 'login') return new Promise(() => {})
    if (command === 'auth_action') return { ...auth, state: 'signed_out', principal: null }
  })
  await render(); await click('Sign in')
  expect(host.textContent).toContain('Finish signing in in your browser.')
  expect(buttons().map(button => button.textContent)).toEqual(['Cancel sign-in'])
  expect(buttons()[0].disabled).toBe(false)
  await click('Cancel sign-in')
  expect(invoke).toHaveBeenCalledWith('cancel_login', undefined)
})
it('a failed status check offers Check again', async () => {
  invoke.mockRejectedValueOnce(new Error('unavailable'))
  await render()
  expect(host.textContent).toContain('Unable to check account.')
  expect(buttons().map(button => button.textContent)).toEqual(['Check again'])
  await click('Check again')
  expect(host.textContent).toContain('Signed in')
})
